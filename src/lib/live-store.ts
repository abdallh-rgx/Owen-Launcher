// ---------------------------------------------------------------------------
// Owen Control Room — in-memory live channel (screen stream + merged log).
//
// The GitHub Actions driver pushes JPEG frames (~1 fps) and logcat/driver log
// batches here; browsers long-poll GET /api/control/live and get woken the
// moment anything new arrives. Survives dev-server hot reloads via globalThis.
// ---------------------------------------------------------------------------

export interface LiveFrame {
  seq: number;
  /** data:image/jpeg;base64,... (or png fallback) */
  dataUrl: string;
  /** delivered frame size (downscaled by the driver) */
  w: number;
  h: number;
  /** physical emulator resolution the coordinates map to */
  deviceW: number;
  deviceH: number;
  app: string;
  ts: number;
}

export type LiveLogLevel = 'INFO' | 'WARN' | 'ERROR' | 'CRASH' | 'DRIVER';
export type LiveLogSource = 'logcat' | 'driver' | 'crash' | 'signal';

export interface LiveLogLine {
  seq: number;
  ts: number;
  level: LiveLogLevel;
  src: LiveLogSource;
  text: string;
}

interface LiveStoreState {
  frame: LiveFrame | null;
  frameSeq: number;
  logs: LiveLogLine[];
  logSeq: number;
  /** bumped whenever a ControlSignal is created or acked */
  signalsVersion: number;
  /** every origin a real browser was seen from recently (url → last-seen ts) */
  origins: Map<string, number>;
  /** sticky origin that passed the reachability probe (used for dispatches) */
  preferredOrigin: string | null;
  waiters: Set<() => void>;
}

const LOG_RING = 500;

function makeStore(): LiveStoreState {
  return {
    frame: null,
    frameSeq: 0,
    logs: [],
    logSeq: 0,
    signalsVersion: 0,
    origins: new Map(),
    preferredOrigin: null,
    waiters: new Set(),
  };
}

/* globalThis cache — Next dev reloads modules; keep the stream alive. */
const g = globalThis as unknown as { __owenLiveStore?: LiveStoreState };
export const liveStore: LiveStoreState = g.__owenLiveStore ?? makeStore();
g.__owenLiveStore = liveStore;
/* migrate stores cached by older module versions (hot reload) */
if (!liveStore.origins) {
  (liveStore as { origins?: Map<string, number> }).origins = new Map();
}
if (liveStore.preferredOrigin === undefined) {
  (liveStore as { preferredOrigin?: string | null }).preferredOrigin = null;
}

export function notifyLive(): void {
  for (const wake of liveStore.waiters) {
    try {
      wake();
    } catch {
      /* waiter gone — ignore */
    }
  }
  liveStore.waiters.clear();
}

export function pushFrame(
  dataUrl: string,
  w: number,
  h: number,
  deviceW: number,
  deviceH: number,
  app: string,
): LiveFrame {
  liveStore.frameSeq += 1;
  liveStore.frame = {
    seq: liveStore.frameSeq,
    dataUrl,
    w,
    h,
    deviceW,
    deviceH,
    app,
    ts: Date.now(),
  };
  notifyLive();
  return liveStore.frame;
}

export function pushLogs(
  lines: Array<{ level?: string; src?: string; text: string }>,
): number {
  let added = 0;
  const now = Date.now();
  for (const l of lines) {
    const level = normalizeLevel(l.level);
    const src = normalizeSrc(l.src);
    const text = String(l.text ?? '').slice(0, 2000);
    if (!text) continue;
    liveStore.logSeq += 1;
    liveStore.logs.push({ seq: liveStore.logSeq, ts: now, level, src, text });
    added += 1;
  }
  if (added > 0) {
    if (liveStore.logs.length > LOG_RING) {
      liveStore.logs.splice(0, liveStore.logs.length - LOG_RING);
    }
    notifyLive();
  }
  return added;
}

function normalizeLevel(raw: unknown): LiveLogLevel {
  const v = String(raw ?? 'INFO').toUpperCase();
  if (v === 'WARN' || v === 'W') return 'WARN';
  if (v === 'ERROR' || v === 'E') return 'ERROR';
  if (v === 'CRASH' || v === 'FATAL') return 'CRASH';
  if (v === 'DRIVER' || v === 'D') return 'DRIVER';
  return 'INFO';
}

function normalizeSrc(raw: unknown): LiveLogSource {
  const v = String(raw ?? 'logcat');
  if (v === 'driver' || v === 'crash' || v === 'signal') return v;
  return 'logcat';
}

/** Wait until something changes (frame/log/signal) or `ms` elapses. */
export function waitForChange(ms: number): Promise<void> {
  return new Promise((resolve) => {
    let done = false;
    const wake = () => {
      if (done) return;
      done = true;
      liveStore.waiters.delete(wake);
      clearTimeout(timer);
      resolve();
    };
    const timer = setTimeout(wake, ms);
    liveStore.waiters.add(wake);
  });
}

/** Remember the origin the operator's browser came from (agent relaunches).
 * Keeps a map of every recent origin — some preview gateways only work with
 * session headers (fcapp.run), so the launch route probes them all. */
export function recordOrigin(req: Request): void {
  try {
    const origin =
      req.headers.get('origin') ||
      req.headers.get('referer')?.match(/^https?:\/\/[^/]+/)?.[0] ||
      `${req.headers.get('x-forwarded-proto') ?? 'https'}://${req.headers.get('host') ?? ''}`;
    if (!origin || !/^https?:\/\/.+\..+/.test(origin)) return;
    const url = origin.replace(/\/+$/, '');
    const isLocal = /^http:\/\/(localhost|127\.)/.test(url);
    if (!isLocal) {
      liveStore.origins.set(url, Date.now());
      // prune entries older than 12h
      const cutoff = Date.now() - 12 * 3600_000;
      for (const [u, ts] of liveStore.origins) {
        if (ts < cutoff) liveStore.origins.delete(u);
      }
    }
  } catch {
    /* ignore */
  }
}

/** Most recently seen non-local origin (compat helper). */
export function lastSeenOrigin(): { url: string; ts: number } | null {
  let best: { url: string; ts: number } | null = null;
  for (const [url, ts] of liveStore.origins) {
    if (!best || ts > best.ts) best = { url, ts };
  }
  return best;
}
