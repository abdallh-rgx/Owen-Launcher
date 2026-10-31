// ---------------------------------------------------------------------------
// OWEN AUTOPILOT — الطيار الآلي لغرفة تحكم أوين.
//
// المستخدم طلب: "أنت افعل كل شيء بنفسك" — هذه الوحدة هي العقل الذي:
//   1. يراقب طلبات التعليق (PENDING tasks) الواصلة من الدرايفر
//      (GitHub Actions / mock driver) ويحلّل لقطة الشاشة بنموذج الرؤية
//      glm-4.6v عبر z-ai-web-dev-sdk، ثم يجيب تلقائياً: أين يضغط، وكم
//      ينتظر، وماذا يرى.
//   2. يصنّف الشاشة (قاذفة/تحميل/لوبي/مباراة…) ويرصد نسبة التحميل.
//   3. يحفظ الأدلة (lobby + in_match) تلقائياً كصور مسمّاة.
//   4. يرسل الإشارات تلقائياً: download_done عند اكتمال التحميل، وstart_sdk
//      بعد الوصول للمباراة، وrestart_app عند الكراش.
//   5. يراقب سجل الكراش (live-store CRASH lines) ويحلّل السبب بالـ LLM.
//   6. يراقب نبضة الدرايفر وينبّه إذا توقفت.
//
// المستخدم يستطيع دائماً التدخل يدوياً قبل مهلة السماح (humanGraceMs) —
// إجابته تُلتزم فوراً لأن الطيار لن يلمس طلباً أجابه إنسان.
//
// يبدأ العمل تلقائياً عبر src/instrumentation.ts وعند أول استدعاء لأي مسار
// API (ensureAutopilot idempotent) ويبقى حياً عبر إعادة التحميل الحار
// (globalThis singleton) والقرص (data/control/autopilot-state.json).
// ---------------------------------------------------------------------------
import { copyFile, mkdir, readFile, writeFile } from 'fs/promises';
import { existsSync } from 'fs';
import path from 'path';
import sharp from 'sharp';
import ZAI from 'z-ai-web-dev-sdk';
import { db } from '@/lib/db';
import { SHOTS_DIR } from '@/lib/control';
import { liveStore, notifyLive, pushLogs } from '@/lib/live-store';

// ---------------------------------------------------------------------------
// الحالة
// ---------------------------------------------------------------------------

export type AutopilotScreen =
  | 'launcher_splash'
  | 'launcher_home'
  | 'downloading'
  | 'download_done'
  | 'fortnite_boot'
  | 'eula'
  | 'login'
  | 'lobby'
  | 'match_loading'
  | 'in_match'
  | 'crash_dialog'
  | 'android_home'
  | 'other';

export interface AutopilotDecision {
  ts: number;
  taskId: string;
  seq: number;
  screen: string;
  summary: string;
  action: string;
  x: number | null;
  y: number | null;
  waitMs: number;
  downloadPct: number | null;
  evidence: string | null;
  vlmFail: boolean;
}

export interface AutopilotEvidence {
  kind: 'lobby' | 'in_match';
  name: string;
  ts: number;
  summary: string;
}

export interface AutopilotCrash {
  ts: number;
  analysis: string;
  excerpt: string;
  restarted: boolean;
}

interface AutopilotState {
  enabled: boolean;
  startedAt: number;
  lastTickAt: number;
  lastScreen: string | null;
  prevScreen: string | null;
  downloadPct: number | null;
  downloadHighPct: number | null;
  /** عدد قراءات "lobby/in_match" المتتالية — نحتاج قراءتين متتاليتين للدليل */
  lobbyStreak: number;
  matchStreak: number;
  stepsAnswered: number;
  vlmFailures: number;
  /** تباعد أسي بعد أعطال VLM (خاصة 429) — لا محاولات قبل هذا الوقت */
  vlmBackoffUntil: number;
  consecutiveFails: number;
  /** تباعد عام بين استدعاءات VLM (حماية من 429 في الجلسات الطويلة) */
  lastVlmAt: number;
  humanGraceMs: number;
  autoSdk: boolean;
  autoRestart: boolean;
  downloadDoneSignalSent: boolean;
  sdkSignalSent: number | null;
  lastRestartSignalAt: number | null;
  lastCrashLogSeq: number;
  crashDebounceUntil: number;
  alert: string | null;
  decisions: AutopilotDecision[];
  evidence: AutopilotEvidence[];
  crashes: AutopilotCrash[];
  /** داخلي */
  busyTaskId: string | null;
  inMatchAt: number | null;
}

const STATE_FILE = path.join(
  process.cwd(),
  'data',
  'control',
  'autopilot-state.json',
);
const TICK_MS = 3500;
const VLM_TIMEOUT_MS = 90_000;
/** حد أدنى بين استدعاءات VLM — ~٨ استدعاءات/دقيقة كحد أقصى */
const VLM_MIN_INTERVAL_MS = 7_000;
const VLM_WIDTH = 640;
const HEARTBEAT_STALE_ALERT_MS = 180_000;
const CRASH_RESTART_THROTTLE_MS = 150_000;
const SDK_AFTER_MATCH_DELAY_MS = 90_000;
const MAX_DECISIONS = 40;
const MAX_CRASHES = 12;

function freshState(): AutopilotState {
  return {
    enabled: true,
    startedAt: Date.now(),
    lastTickAt: 0,
    lastScreen: null,
    prevScreen: null,
    downloadPct: null,
    downloadHighPct: null,
    lobbyStreak: 0,
    matchStreak: 0,
    stepsAnswered: 0,
    vlmFailures: 0,
    vlmBackoffUntil: 0,
    consecutiveFails: 0,
    lastVlmAt: 0,
    humanGraceMs: 6000,
    autoSdk: true,
    autoRestart: true,
    downloadDoneSignalSent: false,
    sdkSignalSent: null,
    lastRestartSignalAt: null,
    lastCrashLogSeq: 0,
    crashDebounceUntil: 0,
    alert: null,
    decisions: [],
    evidence: [],
    crashes: [],
    busyTaskId: null,
    inMatchAt: null,
  };
}

/* مُفرد عبر globalThis — يصمد أمام hot-reload */
const g = globalThis as unknown as {
  __owenAutopilot?: {
    state: AutopilotState;
    timer: ReturnType<typeof setInterval> | null;
    persistTimer: ReturnType<typeof setTimeout> | null;
    started: boolean;
    /** مرجع دالة tick التي سلّحت المؤقت — يتغيّر عند hot-reload */
    tickFn: (() => Promise<void>) | null;
  };
};

function holder() {
  if (!g.__owenAutopilot) {
    g.__owenAutopilot = {
      state: freshState(),
      timer: null,
      persistTimer: null,
      started: false,
      tickFn: null,
    };
    void loadState();
  }
  return g.__owenAutopilot;
}

async function loadState(): Promise<void> {
  try {
    if (!existsSync(STATE_FILE)) return;
    const raw = JSON.parse(await readFile(STATE_FILE, 'utf8')) as Partial<
      AutopilotState & { busyTaskId?: string | null }
    >;
    const h = g.__owenAutopilot;
    if (!h) return;
    h.state = {
      ...freshState(),
      ...raw,
      busyTaskId: null, // عملية معلّقة من عملية سابقة تُهمل دائماً
      lastTickAt: 0,
      alert: null,
    };
  } catch {
    /* ملف تالف → نبدأ من جديد */
  }
}

function persist(): void {
  const h = holder();
  if (h.persistTimer) clearTimeout(h.persistTimer);
  h.persistTimer = setTimeout(() => {
    h.persistTimer = null;
    const snapshot = { ...h.state, busyTaskId: null };
    void (async () => {
      try {
        await mkdir(path.dirname(STATE_FILE), { recursive: true });
        await writeFile(STATE_FILE, JSON.stringify(snapshot), 'utf8');
      } catch {
        /* القرص ممتلئ أو للقراءة فقط — لا يوقف الطيار */
      }
    })();
  }, 700);
}

/** شغّل الطيار (idempotent) — يستدعى من instrumentation ومسارات الـ API.
 *  عند hot-reload يُعاد التسليح تلقائياً لأن مرجع tick يتغيّر مع الكود الجديد. */
export function ensureAutopilot(): void {
  const h = holder();
  if (h.started && h.tickFn === tick) return;
  if (h.timer) clearInterval(h.timer);
  h.tickFn = tick;
  h.started = true;
  h.state.startedAt = h.state.startedAt || Date.now();
  h.timer = setInterval(() => {
    void tick().catch(() => {
      /* أخطاء الدورة لا توقف الحلقة */
    });
  }, TICK_MS);
  pushLogs([
    {
      level: 'DRIVER',
      src: 'driver',
      text: '🤖 OWEN AUTOPILOT مسلّح — القيادة الآلية الكاملة نشطة (glm-4.6v)',
    },
  ]);
}

// ---------------------------------------------------------------------------
// DTO للواجهة
// ---------------------------------------------------------------------------

export function autopilotDTO() {
  const s = holder().state;
  const withUrl = (e: AutopilotEvidence) => ({
    ...e,
    url: `/api/control/shot/${e.name}?v=1`,
  });
  const lobby = s.evidence.find((e) => e.kind === 'lobby') ?? null;
  const match = s.evidence.find((e) => e.kind === 'in_match') ?? null;
  return {
    enabled: s.enabled,
    running: true,
    startedAt: s.startedAt,
    lastTickAt: s.lastTickAt,
    screen: s.lastScreen,
    downloadPct: s.downloadPct,
    stepsAnswered: s.stepsAnswered,
    vlmFailures: s.vlmFailures,
    humanGraceMs: s.humanGraceMs,
    autoSdk: s.autoSdk,
    autoRestart: s.autoRestart,
    sdkSignalSent: s.sdkSignalSent !== null,
    downloadDoneSignalSent: s.downloadDoneSignalSent,
    alert: s.alert,
    lobbyEvidence: lobby ? withUrl(lobby) : null,
    matchEvidence: match ? withUrl(match) : null,
    evidence: s.evidence.map(withUrl),
    decisions: s.decisions.slice(-14).reverse(),
    crashes: s.crashes.slice(-6).reverse(),
  };
}

export type AutopilotDTO = ReturnType<typeof autopilotDTO>;

// ---------------------------------------------------------------------------
// الحلقة الرئيسية
// ---------------------------------------------------------------------------

async function tick(): Promise<void> {
  const s = holder().state;
  s.lastTickAt = Date.now();

  if (!s.enabled) {
    s.alert = null;
    persist();
    return;
  }

  await scanCrashes(s);
  await checkHeartbeat(s);
  await maybeAutoSignals(s);
  await answerPending(s);
  persist();
}

// ---------------------------------------------------------------------------
// 1) الإجابة على الطلبات المعلّقة عبر VLM
// ---------------------------------------------------------------------------

async function answerPending(s: AutopilotState): Promise<void> {
  if (s.busyTaskId) return;
  // تباعد بعد الأعطال (429) — لا نضغط على النموذج وهو يرفض
  if (Date.now() < s.vlmBackoffUntil) return;
  // تباعد عام بين الاستدعاءات (حماية من حصر المعدل في الجلسات الطويلة)
  if (Date.now() - s.lastVlmAt < VLM_MIN_INTERVAL_MS) return;

  let task: {
    id: string;
    seq: number;
    createdAt: Date;
    imageName: string | null;
    imageW: number | null;
    imageH: number | null;
    meta: string | null;
    status: string;
  } | null = null;
  try {
    task = await db.controlTask.findFirst({
      where: { status: 'PENDING' },
      orderBy: { seq: 'asc' },
      select: {
        id: true,
        seq: true,
        createdAt: true,
        imageName: true,
        imageW: true,
        imageH: true,
        meta: true,
        status: true,
      },
    });
  } catch {
    return; // قاعدة البيانات غير جاهزة بعد
  }
  if (!task) return;

  // مهلة سماح بشرية قصيرة: إن سبق إنسانٌ الجواب فلا نلمس الطلب أبداً
  if (Date.now() - task.createdAt.getTime() < s.humanGraceMs) return;
  if (!task.imageName) return;

  const devW = task.imageW ?? 1080;
  const devH = task.imageH ?? 2280;
  s.busyTaskId = task.id;

  try {
    // ربما أجاب إنسان خلال ثواني التحضير — تحقق مجدداً
    const fresh = await db.controlTask.findUnique({
      where: { id: task.id },
      select: { status: true },
    });
    if (!fresh || fresh.status !== 'PENDING') return;

    const pngPath = path.join(SHOTS_DIR, task.imageName);
    if (!existsSync(pngPath)) {
      await answerTask(s, task.id, task.seq, {
        screen: 'other',
        summary: 'لقطة الشاشة غير موجودة على القرص — انتظار قصير',
        action: 'wait',
        bbox: null,
        waitMs: 5000,
        evidence: null,
        downloadPct: null,
      }, devW, devH);
      return;
    }

    // صغّر الصورة قبل الإرسال (توفير في الحجم والزمن — الإحداثيات منسوبة 0..1000)
    const jpeg = await sharp(pngPath)
      .resize({ width: VLM_WIDTH, withoutEnlargement: true })
      .jpeg({ quality: 70 })
      .toBuffer();
    const dataUrl = `data:image/jpeg;base64,${jpeg.toString('base64')}`;

    const history = s.decisions
      .slice(-4)
      .map((d) => `#${d.seq}: ${d.screen} → ${d.action} (${d.waitMs}ms)`)
      .join(' | ') || 'none';
    const lobbyDone = s.evidence.some((e) => e.kind === 'lobby');
    const matchDone = s.evidence.some((e) => e.kind === 'in_match');

    const prompt = buildPrompt(history, lobbyDone, matchDone, s.lastScreen);

    s.lastVlmAt = Date.now();
    let decision = sanitizeDecision(await vlmDecide(dataUrl, prompt), s);
    s.consecutiveFails = 0;

    // إعادة محاولة واحدة إذا أراد الضغط لكنه لم يحدد مربعاً صالحاً
    if (decision.action === 'tap' && !decision.bbox) {
      decision = sanitizeDecision(
        await vlmDecide(dataUrl, prompt, 'tap without a valid bbox'),
        s,
      );
      if (decision.action === 'tap' && !decision.bbox) {
        decision = {
          ...decision,
          action: 'wait',
          waitMs: Math.max(decision.waitMs, 8000),
          summary: `${decision.summary} (تعذّر تحديد موضع الزر — انتظار وإعادة)`,
        };
      }
    }

    await answerTask(s, task.id, task.seq, decision, devW, devH);

    // متابعة الشاشة + الأدلة + إشارات التحميل
    trackScreen(s, decision, pngPath, task.id);
  } catch (err) {
    s.vlmFailures += 1;
    s.consecutiveFails += 1;
    const msg = err instanceof Error ? err.message : String(err);
    const isRate = /429|too many/i.test(msg);
    // تباعد أسي: ٢٠ث ← ٤٠ث ← ٨٠ث (سقف ٩٠ث) — أطول مع 429
    const backoffMs = Math.min(
      90_000,
      (isRate ? 30_000 : 15_000) * Math.pow(2, Math.max(0, s.consecutiveFails - 1)),
    );
    s.vlmBackoffUntil = Date.now() + backoffMs;
    pushLogs([
      {
        level: 'ERROR',
        src: 'driver',
        text: `🤖 VLM فشل (${msg.slice(0, 140)}) — تباعد ${Math.round(backoffMs / 1000)}ث قبل المحاولة التالية`,
      },
    ]);
    // لا نعلّق الحلقة: جاوب "انتظار" بمدة التباعد نفسها حتى يهدأ المعدل
    try {
      await answerTask(s, task.id, task.seq, {
        screen: s.lastScreen ?? 'other',
        summary: `تعذّر تحليل الشاشة (${isRate ? 'ضغط على النموذج' : 'خطأ VLM'}) — انتظار ${Math.round(backoffMs / 1000)}ث ثم إعادة`,
        action: 'wait',
        bbox: null,
        waitMs: backoffMs,
        evidence: null,
        downloadPct: null,
      }, devW, devH, true);
    } catch {
      /* الطلب اختفى — لا بأس */
    }
  } finally {
    s.busyTaskId = null;
  }
}

function buildPrompt(
  history: string,
  lobbyDone: boolean,
  matchDone: boolean,
  lastScreen: string | null,
): string {
  // المرحلة الحالية توجّه القرار: دليل اللوبي أولاً (انتظار)، ثم بدء المباراة
  let stage: string;
  if (!lobbyDone) {
    stage =
      'CURRENT STAGE: CAPTURE THE LOBBY. When the Fortnite lobby becomes visible answer action="wait" with waitMs 4000-6000 and evidence="lobby" — do NOT press PLAY yet (an evidence screenshot is captured server-side). Only press menu/accept buttons if popups block the lobby. Once you are told lobby evidence is collected, move to the match stage.';
  } else if (!matchDone) {
    stage =
      'CURRENT STAGE: START A MATCH. From the lobby tap PLAY, then wait through matchmaking/loading (screen=match_loading) until live gameplay; report evidence="in_match" only when the gameplay HUD is clearly visible.';
  } else {
    stage =
      'CURRENT STAGE: HOLD. Both evidence shots are captured. Keep the session alive: wait, report what you see; if a menu appears over the match dismiss it; never leave the match on purpose.';
  }
  return `You are OWEN AUTOPILOT — an AI operator driving an Android emulator (portrait phone screen ~1080x2280). The app flow: OwenLauncher (offline Fortnite 12.41 launcher) → accept consents/EULA → tap the big download/install CTA → wait while game data downloads (percent shown) → tap LAUNCH/PLAY → Fortnite boots → accept EULA / skip login → reach the LOBBY → press PLAY → matchmaking/loading → IN-MATCH gameplay.

MISSION (in order): reach the Fortnite LOBBY (evidence #1), then start a match and reach IN-MATCH gameplay (evidence #2).

${stage}

Screen classes: launcher_splash, launcher_home, downloading, download_done, fortnite_boot, eula, login, lobby, match_loading, in_match, crash_dialog, android_home, other.

Recent actions you took: ${history}
Evidence already collected: lobby=${lobbyDone}, in_match=${matchDone}
Previous screen classification: ${lastScreen ?? 'none'}

Answer with STRICT JSON ONLY (no markdown, no prose):
{"screen":"<class>","summary":"<قصير بالعربية: ماذا ترى وماذا ستفعل>","action":"tap|swipe|wait|back|home","bbox":[x0,y0,x1,y1],"waitMs":<int>,"evidence":"lobby|in_match|null","downloadPct":<int 0-100 or null>}

RULES:
- bbox is NORMALIZED 0..1000 relative to the image width/height (not pixels). It must tightly cover the ONE button to press now; the tap lands at its center. null when action is wait/back/home.
- action tap = press that button; wait = nothing to press (loading/progress); back = close a popup via back key; home = go to Android home.
- waitMs AFTER the action: launcher_splash 8000-15000, downloading 20000-45000, fortnite_boot 12000-20000, match_loading 15000-30000, eula/login 3000-6000, simple menus 2500-4000.
- evidence="lobby" ONLY when the Fortnite main lobby is clearly visible (character + menus). evidence="in_match" ONLY when real gameplay HUD is visible (health/shield/ammo/minimap during a live match). Otherwise null.
- downloadPct: the visible download percent, else null.
- If an Android crash/"keeps stopping" dialog is visible → screen=crash_dialog and tap its dismiss/close/OK button.
- Never propose "done" — the mission controller decides when to stop.
- Prefer the most obvious primary CTA (big yellow/green button).`;
}

interface RawDecision {
  screen: string;
  summary: string;
  action: string;
  bbox: number[] | null;
  waitMs: number;
  evidence: string | null;
  downloadPct: number | null;
}

async function vlmDecide(
  dataUrl: string,
  prompt: string,
  retryNote?: string,
): Promise<RawDecision> {
  const zai = await ZAI.create();
  const messages = [
    {
      role: 'user' as const,
      content: [
        {
          type: 'text' as const,
          text: retryNote
            ? `${prompt}\n\nIMPORTANT: your previous answer was invalid (${retryNote}). Respond again with VALID strict JSON, bbox normalized 0..1000.`
            : prompt,
        },
        { type: 'image_url' as const, image_url: { url: dataUrl } },
      ],
    },
  ];
  const call = zai.chat.completions.createVision({
    model: 'glm-4.6v',
    messages,
    thinking: { type: 'disabled' },
  });
  const timeout = new Promise<never>((_, reject) =>
    setTimeout(() => reject(new Error('VLM timeout')), VLM_TIMEOUT_MS),
  );
  const res = (await Promise.race([call, timeout])) as {
    choices?: Array<{ message?: { content?: string } }>;
  };
  const text = res.choices?.[0]?.message?.content ?? '';
  const parsed = parseJsonLoose(text);
  if (!parsed) throw new Error('VLM returned non-JSON');
  return parsed as unknown as RawDecision;
}

function parseJsonLoose(text: string): Record<string, unknown> | null {
  const cleaned = text
    .replace(/```json/gi, '')
    .replace(/```/g, '')
    .trim();
  const start = cleaned.indexOf('{');
  const end = cleaned.lastIndexOf('}');
  if (start < 0 || end <= start) return null;
  try {
    return JSON.parse(cleaned.slice(start, end + 1)) as Record<
      string,
      unknown
    >;
  } catch {
    return null;
  }
}

const VALID_ACTIONS = new Set(['tap', 'swipe', 'wait', 'back', 'home']);
const VALID_SCREENS = new Set([
  'launcher_splash', 'launcher_home', 'downloading', 'download_done',
  'fortnite_boot', 'eula', 'login', 'lobby', 'match_loading', 'in_match',
  'crash_dialog', 'android_home', 'other',
]);

function validBbox(bbox: unknown): number[] | null {
  if (!Array.isArray(bbox) || bbox.length !== 4) return null;
  const n = bbox.map((v) => Number(v));
  if (n.some((v) => !Number.isFinite(v))) return null;
  const [x0, y0, x1, y1] = n.map((v) => Math.min(1000, Math.max(0, v)));
  if (x1 - x0 < 10 || y1 - y0 < 10) return null;
  return [x0, y0, x1, y1];
}

function sanitizeDecision(raw: RawDecision, s: AutopilotState): RawDecision {
  const action = VALID_ACTIONS.has(raw.action) ? raw.action : 'wait';
  const screen = VALID_SCREENS.has(raw.screen) ? raw.screen : 'other';
  let waitMs = Number.isFinite(raw.waitMs) ? Math.round(raw.waitMs) : 4000;
  waitMs = Math.min(60_000, Math.max(1500, waitMs));
  if (screen === 'downloading') waitMs = Math.max(waitMs, 15_000);
  if (screen === 'match_loading') waitMs = Math.max(waitMs, 12_000);
  const pct =
    Number.isFinite(raw.downloadPct as number)
      ? Math.min(100, Math.max(0, Math.round(raw.downloadPct as number)))
      : null;

  // التصنيف هو مصدر الحقيقة للأدلة (مع اشتراط قراءتين متتاليتين لاحقاً)
  let evidence: 'lobby' | 'in_match' | null = null;
  if (screen === 'lobby') evidence = 'lobby';
  else if (screen === 'in_match') evidence = 'in_match';
  else if (raw.evidence === 'lobby' || raw.evidence === 'in_match') {
    evidence = raw.evidence;
  }

  let finalAction = action;
  let bbox = validBbox(raw.bbox);
  let summary = String(raw.summary ?? '').slice(0, 300) || 'قرار آلي';

  // 🔒 منع ميكانيكي: لا نبدأ المباراة قبل التقاط دليل اللوبي —
  // أي نقر داخل اللوبي قبل الدليل يتحول إلى انتظار (لا نعتمد على طاعة
  // النموذج للتعليمات). بعد حفظ الدليل يُسمح بضغط PLAY.
  const lobbyDone = s.evidence.some((e) => e.kind === 'lobby');
  if (screen === 'lobby' && !lobbyDone && action === 'tap') {
    finalAction = 'wait';
    bbox = null;
    waitMs = Math.max(waitMs, 5000);
    summary = `${summary} — تحويل لانتظار: التقاط دليل اللوبي أولاً قبل بدء المباراة`;
  }

  // المهمة لم تكتمل؟ لا نسمح بإنهاء مبكر — كل قرار يبني على السابق
  const matchDone = s.evidence.some((e) => e.kind === 'in_match');
  if (matchDone && screen !== 'in_match' && finalAction === 'wait' && waitMs < 5000) {
    waitMs = 10_000; // ابقَ حياً داخل المباراة دون طقطقة سريعة
  }

  return {
    screen,
    summary,
    action: finalAction,
    bbox: finalAction === 'tap' || finalAction === 'swipe' ? bbox : null,
    waitMs,
    evidence,
    downloadPct: pct,
  };
}

// ---------------------------------------------------------------------------
// 2) كتابة الجواب في قاعدة البيانات (نفس شكل مسار الإجابة اليدوي)
// ---------------------------------------------------------------------------

async function answerTask(
  s: AutopilotState,
  taskId: string,
  seq: number,
  d: RawDecision,
  devW: number,
  devH: number,
  vlmFail = false,
): Promise<void> {
  let x: number | null = null;
  let y: number | null = null;
  let x2: number | null = null;
  let y2: number | null = null;
  if (d.bbox) {
    const cx = (d.bbox[0] + d.bbox[2]) / 2;
    const cy = (d.bbox[1] + d.bbox[3]) / 2;
    x = Math.round((cx / 1000) * devW);
    y = Math.round((cy / 1000) * devH);
    if (d.action === 'swipe') {
      x2 = Math.round((d.bbox[2] / 1000) * devW);
      y2 = Math.round((d.bbox[3] / 1000) * devH);
    }
  }

  let meta: Record<string, unknown> = {};
  try {
    const t = await db.controlTask.findUnique({
      where: { id: taskId },
      select: { meta: true },
    });
    if (t?.meta) meta = JSON.parse(t.meta) as Record<string, unknown>;
  } catch {
    /* meta قديمة تالفة */
  }
  meta.autopilot = true;
  meta.screen = d.screen;
  if (d.downloadPct !== null) meta.downloadPct = d.downloadPct;
  if (d.evidence) meta.evidence = d.evidence;

  await db.controlTask.update({
    where: { id: taskId },
    data: {
      status: 'ANSWERED',
      action: d.action,
      x: d.bbox ? x : null,
      y: d.bbox ? y : null,
      x2: d.action === 'swipe' && d.bbox ? x2 : null,
      y2: d.action === 'swipe' && d.bbox ? y2 : null,
      description: `🤖 ${d.summary}`,
      waitMs: d.waitMs,
      wantsRetake: false,
      answeredAt: new Date(),
      meta: JSON.stringify(meta).slice(0, 4000),
    },
  });

  s.stepsAnswered += 1;
  s.decisions.push({
    ts: Date.now(),
    taskId,
    seq,
    screen: d.screen,
    summary: d.summary,
    action: d.action,
    x: d.bbox ? x : null,
    y: d.bbox ? y : null,
    waitMs: d.waitMs,
    downloadPct: d.downloadPct,
    evidence: d.evidence,
    vlmFail,
  });
  if (s.decisions.length > MAX_DECISIONS) {
    s.decisions.splice(0, s.decisions.length - MAX_DECISIONS);
  }

  const coordTxt = d.bbox ? ` (${x},${y})` : '';
  pushLogs([
    {
      level: 'DRIVER',
      src: 'driver',
      text: `🤖 خطوة ${seq}: [${d.screen}] ${d.action}${coordTxt} — انتظار ${Math.round(d.waitMs / 1000)}ث — ${d.summary}`,
    },
  ]);
  notifyLive();
}

// ---------------------------------------------------------------------------
// 3) تصنيف الشاشة → أدلة + إشارة انتهاء التحميل
// ---------------------------------------------------------------------------

function trackScreen(
  s: AutopilotState,
  d: RawDecision,
  pngPath: string,
  taskId: string,
): void {
  s.prevScreen = s.lastScreen;
  s.lastScreen = d.screen;
  s.downloadPct = d.screen === 'downloading' ? d.downloadPct : null;
  if (d.screen === 'downloading' && d.downloadPct !== null) {
    s.downloadHighPct = Math.max(s.downloadHighPct ?? 0, d.downloadPct);
  }

  // دليل اللوبي — قراءتان متتاليتان حتى لا يكون التباساً
  if (d.screen === 'lobby' && d.evidence === 'lobby') {
    s.lobbyStreak += 1;
    if (s.lobbyStreak >= 2 && !s.evidence.some((e) => e.kind === 'lobby')) {
      saveEvidence(s, 'lobby', pngPath, taskId, d.summary);
    }
  } else {
    s.lobbyStreak = 0;
  }

  // دليل المباراة — قراءتان متتاليتان
  if (d.screen === 'in_match' && d.evidence === 'in_match') {
    s.matchStreak += 1;
    if (s.matchStreak >= 2 && !s.evidence.some((e) => e.kind === 'in_match')) {
      saveEvidence(s, 'in_match', pngPath, taskId, d.summary);
      s.inMatchAt = Date.now();
    }
  } else {
    s.matchStreak = 0;
  }

  // انتهاء التحميل: انتقال من downloading إلى مرحلة لاحقة حقيقية، أو نسبة ≥ 98
  // (إعادة التشغيل بعد الكراش لا تُحتسب اكتمالاً — الهدف يجب أن يكون بعد التحميل)
  const RESET_TARGETS = new Set([
    'launcher_splash', 'launcher_home', 'android_home', 'crash_dialog', 'other',
  ]);
  const leftDownload =
    s.prevScreen === 'downloading' &&
    d.screen !== 'downloading' &&
    !RESET_TARGETS.has(d.screen);
  const pctFull = d.downloadPct !== null && d.downloadPct >= 98;
  if ((leftDownload || pctFull) && !s.downloadDoneSignalSent) {
    s.downloadDoneSignalSent = true;
    void createSignal(
      'download_done',
      `🤖 تلقائي: اكتمل التحميل الداخلي (${s.downloadHighPct ?? 100}%) — الانتقال للمرحلة التالية`,
    );
  }
}

async function saveEvidence(
  s: AutopilotState,
  kind: 'lobby' | 'in_match',
  pngPath: string,
  taskId: string,
  summary: string,
): Promise<void> {
  try {
    const name = `evidence-${kind}-${Date.now()}.png`;
    await copyFile(pngPath, path.join(SHOTS_DIR, name));
    s.evidence.push({ kind, name, ts: Date.now(), summary });
    pushLogs([
      {
        level: 'DRIVER',
        src: 'driver',
        text: `🎯 دليل ${kind === 'lobby' ? 'اللوبي' : 'داخل المباراة'} محفوظ: ${name}`,
      },
    ]);
    notifyLive();
  } catch {
    pushLogs([
      {
        level: 'ERROR',
        src: 'driver',
        text: `تعذّر حفظ دليل ${kind} (${taskId})`,
      },
    ]);
  }
}

// ---------------------------------------------------------------------------
// 4) الإشارات التلقائية (SDK بعد المباراة، إعادة تشغيل عند الكراش)
// ---------------------------------------------------------------------------

async function createSignal(type: string, note: string): Promise<void> {
  try {
    const pendingSame = await db.controlSignal.count({
      where: { type, status: 'PENDING' },
    });
    if (pendingSame > 0) return;
    const last = await db.controlSignal.aggregate({ _max: { seq: true } });
    await db.controlSignal.create({
      data: { seq: (last._max.seq ?? 0) + 1, type, note: note.slice(0, 500) },
    });
    liveStore.signalsVersion += 1;
    pushLogs([{ level: 'DRIVER', src: 'signal', text: `🤖 إشارة تلقائية ${type}: ${note}` }]);
    notifyLive();
  } catch {
    /* قاعدة البيانات مشغولة — المحاولة القادمة */
  }
}

async function maybeAutoSignals(s: AutopilotState): Promise<void> {
  // توليد SDK بعد ثبوت المباراة (المستخدم كان سيضغط الزر بنفسه — الآن تلقائي)
  if (
    s.autoSdk &&
    s.inMatchAt !== null &&
    s.sdkSignalSent === null &&
    Date.now() - s.inMatchAt > SDK_AFTER_MATCH_DELAY_MS
  ) {
    s.sdkSignalSent = Date.now();
    await createSignal(
      'start_sdk',
      '🤖 تلقائي: وصلنا داخل المباراة — ابدأ توليد SDK (MobileDumper)',
    );
  }
}

// ---------------------------------------------------------------------------
// 5) مراقبة الكراش + تحليل السبب بالـ LLM
// ---------------------------------------------------------------------------

async function scanCrashes(s: AutopilotState): Promise<void> {
  const newCrashes = liveStore.logs.filter(
    (l) => l.src === 'crash' && l.level === 'CRASH' && l.seq > s.lastCrashLogSeq,
  );
  if (newCrashes.length === 0) return;
  s.lastCrashLogSeq = newCrashes[newCrashes.length - 1].seq;

  const now = Date.now();
  if (now < s.crashDebounceUntil) return; // نفس الحدث يصل على دفعات
  s.crashDebounceUntil = now + 25_000;

  const excerpt = newCrashes
    .slice(0, 8)
    .map((l) => l.text)
    .join('\n')
    .slice(0, 1800);

  // تحليل السبب عبر LLM نصي
  let analysis = 'تحليل غير متاح (فشل الاتصال بالنموذج)';
  try {
    const zai = await ZAI.create();
    const res = await zai.chat.completions.create({
      messages: [
        {
          role: 'user',
          content: `You are an Android native-crash analyst. This crash came from a headless Android 11 x86_64 emulator (ARM translation via libndk) running OwenLauncher (com.abdallhrgx.owenlauncher — a Rust-based offline launcher that boots Fortnite 12.41 with a local Owen Server backend, plus libOwen.so redirector injected via curl hooking). Analyze the crash buffer and answer in ARABIC in 2-4 short sentences: (1) السبب الأرجح (2) هل هو من التطبيق أم من المحاكي/الترجمة (3) اقتراح إصلاح عملي.\n\nCRASH BUFFER:\n${excerpt}`,
        },
      ],
      stream: false,
      thinking: { type: 'disabled' },
    });
    const text = res.choices?.[0]?.message?.content;
    if (text) analysis = text.trim().slice(0, 900);
  } catch {
    /* نحتفظ بالتحليل الافتراضي */
  }

  // إعادة تشغيل تلقائية مع خنق زمني
  let restarted = false;
  if (
    s.autoRestart &&
    (s.lastRestartSignalAt === null ||
      now - s.lastRestartSignalAt > CRASH_RESTART_THROTTLE_MS)
  ) {
    s.lastRestartSignalAt = now;
    restarted = true;
    await createSignal(
      'restart_app',
      '🤖 تلقائي: كراش — إعادة تشغيل OwenLauncher فوراً',
    );
  }

  s.crashes.push({ ts: now, analysis, excerpt: excerpt.slice(0, 600), restarted });
  if (s.crashes.length > MAX_CRASHES) {
    s.crashes.splice(0, s.crashes.length - MAX_CRASHES);
  }
  pushLogs([
    {
      level: 'CRASH',
      src: 'driver',
      text: `🤖 تحليل الكراش: ${analysis}`,
    },
  ]);
  notifyLive();
}

// ---------------------------------------------------------------------------
// 6) نبضة الدرايفر
// ---------------------------------------------------------------------------

async function checkHeartbeat(s: AutopilotState): Promise<void> {
  try {
    const hb = await db.controlHeartbeat.findUnique({ where: { id: 1 } });
    if (!hb) {
      s.alert = null;
      return;
    }
    const age = Date.now() - hb.updatedAt.getTime();
    if (age > HEARTBEAT_STALE_ALERT_MS) {
      s.alert = `لا نبضة من الدرايفر منذ ${Math.round(age / 1000)} ثانية — تحقق من الجلسة على GitHub Actions`;
    } else {
      s.alert = null;
    }
  } catch {
    /* تجاهل */
  }
}

// ---------------------------------------------------------------------------
// أوامر تحكم (من مسار API)
// ---------------------------------------------------------------------------

export async function autopilotCommand(body: {
  enabled?: boolean;
  autoSdk?: boolean;
  autoRestart?: boolean;
  humanGraceMs?: number;
  resetEvidence?: boolean;
  forceSignal?: 'download_done' | 'start_sdk' | 'restart_app' | 'crash_report';
}): Promise<AutopilotDTO> {
  const s = holder().state;

  if (typeof body.enabled === 'boolean') s.enabled = body.enabled;
  if (typeof body.autoSdk === 'boolean') s.autoSdk = body.autoSdk;
  if (typeof body.autoRestart === 'boolean') s.autoRestart = body.autoRestart;
  if (typeof body.humanGraceMs === 'number') {
    s.humanGraceMs = Math.min(120_000, Math.max(0, Math.round(body.humanGraceMs)));
  }
  if (body.resetEvidence) {
    s.evidence = [];
    s.lobbyStreak = 0;
    s.matchStreak = 0;
    s.inMatchAt = null;
    s.sdkSignalSent = null;
    s.downloadDoneSignalSent = false;
    s.downloadHighPct = null;
  }
  if (body.forceSignal) {
    await createSignal(body.forceSignal, '🤖 أمر يدوي مبعوث من غرفة التحكم');
  }
  persist();
  return autopilotDTO();
}
