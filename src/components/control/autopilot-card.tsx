"use client";

/* بطاقة الطيار الآلي — العقل الذي يقود المحاكي بدلاً من المستخدم:
 *   • مؤشر الحالة + مفتاح تشغيل/إيقاف طوارئ
 *   • متتبع المهمة: قاذفة → تحميل → لوبي → مباراة
 *   • نسبة التحميل الحالية + تصنيف الشاشة الحالية
 *   • معرض الأدلة (لقطة اللوبي + لقطة داخل المباراة)
 *   • بث قرارات الذكاء الاصطناعي (glm-4.6v) وتحليل الكراشات
 *   • أزرار: إعادة ضبط الأدلة، توليد SDK تلقائي، إعادة تشغيل تلقائي
 */

import { useState } from "react";
import {
  Bot,
  Bug,
  Download,
  Gamepad2,
  Home,
  Power,
  Rocket,
  RotateCcw,
  Sparkles,
} from "lucide-react";
import {
  AUTOPILOT_SCREENS_AR,
  type AutopilotDTO,
  type AutopilotEvidenceDTO,
} from "@/lib/control-types";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Switch } from "@/components/ui/switch";
import { cn } from "@/lib/utils";
import { arNum, relativeAr } from "./utils";

interface AutopilotCardProps {
  autopilot: AutopilotDTO | null;
  now: number;
  /** يُستدعى بعد أي أمر حتى تُحدَّث الحالة فوراً */
  onCommanded: () => void;
}

const PAST_LAUNCHER = new Set([
  'downloading', 'download_done', 'fortnite_boot', 'eula',
  'login', 'lobby', 'match_loading', 'in_match',
]);

function StepIcon({
  state,
  icon: Icon,
  label,
}: {
  state: 'done' | 'active' | 'todo';
  icon: typeof Rocket;
  label: string;
}) {
  return (
    <div className="flex min-w-0 flex-1 flex-col items-center gap-1.5">
      <div
        className={cn(
          'grid size-10 place-items-center rounded-xl border transition-all',
          state === 'done' &&
            'border-emerald-400/40 bg-emerald-400/15 text-emerald-300',
          state === 'active' &&
            'animate-pulse border-amber-400/50 bg-amber-400/15 text-amber-300',
          state === 'todo' && 'border-white/10 bg-white/[0.04] text-zinc-600',
        )}
        aria-hidden="true"
      >
        <Icon className="size-5" />
      </div>
      <span
        className={cn(
          'text-[11px] font-medium',
          state === 'done' && 'text-emerald-300',
          state === 'active' && 'text-amber-300',
          state === 'todo' && 'text-zinc-500',
        )}
      >
        {label}
      </span>
    </div>
  );
}

function EvidenceTile({
  evidence,
  title,
  icon: Icon,
  now,
}: {
  evidence: AutopilotEvidenceDTO | null;
  title: string;
  icon: typeof Home;
  now: number;
}) {
  if (!evidence) {
    return (
      <div className="flex h-40 flex-col items-center justify-center gap-2 rounded-xl border border-dashed border-white/15 bg-white/[0.02] text-zinc-600">
        <Icon className="size-6" aria-hidden="true" />
        <span className="text-[11px]">بانتظار الوصول…</span>
      </div>
    );
  }
  return (
    <a
      href={evidence.url}
      target="_blank"
      rel="noreferrer"
      className="group relative block h-40 overflow-hidden rounded-xl border border-emerald-400/30 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-emerald-400"
      title="افتح لقطة الدليل بالحجم الكامل"
    >
      <img
        src={evidence.url}
        alt={`دليل ${title} — لقطة من المحاكي`}
        className="size-full object-cover transition duration-300 group-hover:scale-[1.03]"
        loading="lazy"
      />
      <div className="absolute inset-x-0 bottom-0 flex items-center justify-between gap-2 bg-gradient-to-t from-black/85 to-transparent px-2.5 pb-2 pt-6">
        <span className="flex items-center gap-1.5 text-[11px] font-bold text-emerald-300">
          <Icon className="size-3.5" aria-hidden="true" />
          {title} ✓
        </span>
        <span className="text-[10px] text-zinc-400">
          {relativeAr(new Date(evidence.ts).toISOString(), now)}
        </span>
      </div>
    </a>
  );
}

export function AutopilotCard({ autopilot, now, onCommanded }: AutopilotCardProps) {
  const [busy, setBusy] = useState(false);

  if (!autopilot) return null;

  const send = async (body: Record<string, unknown>) => {
    if (busy) return;
    setBusy(true);
    try {
      await fetch('/api/control/autopilot', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(body),
        cache: 'no-store',
      });
      onCommanded();
    } finally {
      setBusy(false);
    }
  };

  const screen = autopilot.screen;
  const launcherDone =
    autopilot.downloadDoneSignalSent ||
    (screen !== null && PAST_LAUNCHER.has(screen)) ||
    autopilot.evidence.length > 0;
  const downloadDone = autopilot.downloadDoneSignalSent;
  const lobbyDone = autopilot.lobbyEvidence !== null;
  const matchDone = autopilot.matchEvidence !== null;

  const activeStep = !launcherDone
    ? 'launcher'
    : !downloadDone
      ? 'download'
      : !lobbyDone
        ? 'lobby'
        : !matchDone
          ? 'match'
          : 'complete';

  const downloading = screen === 'downloading';

  return (
    <Card
      className={cn(
        'gap-4 rounded-2xl border backdrop-blur-sm transition-colors sm:gap-0',
        autopilot.enabled
          ? 'border-amber-400/25 bg-amber-400/[0.04] shadow-[0_0_36px_rgba(251,191,36,0.07)]'
          : 'border-white/10 bg-white/[0.03]',
      )}
    >
      <CardHeader className="pb-3">
        <div className="flex flex-wrap items-center justify-between gap-3">
          <div className="flex min-w-0 items-center gap-3">
            <div
              className={cn(
                'grid size-11 shrink-0 place-items-center rounded-xl border',
                autopilot.enabled
                  ? 'animate-pulse border-amber-400/40 bg-amber-400/15 text-amber-300'
                  : 'border-white/10 bg-white/[0.04] text-zinc-500',
              )}
              aria-hidden="true"
            >
              <Bot className="size-6" />
            </div>
            <div className="min-w-0">
              <CardTitle className="flex flex-wrap items-center gap-2 text-base sm:text-lg">
                الطيار الآلي — Owen Autopilot
                {autopilot.enabled ? (
                  <Badge className="border-emerald-400/30 bg-emerald-400/10 text-[10px] text-emerald-300 hover:bg-emerald-400/10">
                    يقود الآن
                  </Badge>
                ) : (
                  <Badge className="border-white/15 bg-white/[0.05] text-[10px] text-zinc-400 hover:bg-white/[0.05]">
                    متوقف
                  </Badge>
                )}
              </CardTitle>
              <p className="mt-0.5 truncate text-[11px] leading-relaxed text-zinc-500">
                رؤية glm-4.6v تحلل كل لقطة وتضغط الأزرار بنفسها — تدخلك اختياري
              </p>
            </div>
          </div>

          <label className="flex cursor-pointer items-center gap-2 text-xs text-zinc-400">
            <Power className="size-4" aria-hidden="true" />
            <span className="sr-only">تشغيل أو إيقاف الطيار الآلي</span>
            <Switch
              checked={autopilot.enabled}
              disabled={busy}
              onCheckedChange={(v) => void send({ enabled: v })}
              aria-label="مفتاح الطيار الآلي"
            />
          </label>
        </div>
      </CardHeader>

      <CardContent className="space-y-4">
        {autopilot.alert && (
          <div
            role="alert"
            className="rounded-xl border border-red-400/30 bg-red-500/10 px-3 py-2 text-xs leading-relaxed text-red-300"
          >
            ⚠️ {autopilot.alert}
          </div>
        )}

        {/* متتبع المهمة */}
        <div
          className="flex items-start gap-1 sm:gap-2"
          aria-label="مراحل المهمة"
        >
          <StepIcon
            icon={Rocket}
            label="القاذفة"
            state={launcherDone ? 'done' : 'active'}
          />
          <div
            className={cn(
              'mt-5 h-px flex-1',
              launcherDone ? 'bg-emerald-400/40' : 'bg-white/10',
            )}
            aria-hidden="true"
          />
          <StepIcon
            icon={Download}
            label="التحميل"
            state={
              downloadDone ? 'done' : activeStep === 'download' ? 'active' : 'todo'
            }
          />
          <div
            className={cn(
              'mt-5 h-px flex-1',
              downloadDone ? 'bg-emerald-400/40' : 'bg-white/10',
            )}
            aria-hidden="true"
          />
          <StepIcon
            icon={Home}
            label="اللوبي"
            state={lobbyDone ? 'done' : activeStep === 'lobby' ? 'active' : 'todo'}
          />
          <div
            className={cn(
              'mt-5 h-px flex-1',
              lobbyDone ? 'bg-emerald-400/40' : 'bg-white/10',
            )}
            aria-hidden="true"
          />
          <StepIcon
            icon={Gamepad2}
            label="المباراة"
            state={matchDone ? 'done' : activeStep === 'match' ? 'active' : 'todo'}
          />
        </div>

        {/* الشاشة الحالية + التحميل */}
        <div className="flex flex-wrap items-center justify-between gap-2 rounded-xl border border-white/10 bg-black/20 px-3 py-2.5">
          <div className="flex items-center gap-2 text-xs">
            <Sparkles className="size-3.5 text-amber-300" aria-hidden="true" />
            <span className="text-zinc-400">الشاشة الحالية:</span>
            <span className="font-bold text-zinc-100">
              {screen ? (AUTOPILOT_SCREENS_AR[screen] ?? screen) : '—'}
            </span>
          </div>
          {downloading && autopilot.downloadPct !== null && (
            <div className="flex min-w-40 flex-1 items-center gap-2 sm:max-w-56">
              <div
                className="h-2 flex-1 overflow-hidden rounded-full bg-white/10"
                role="progressbar"
                aria-valuenow={autopilot.downloadPct}
                aria-valuemin={0}
                aria-valuemax={100}
                aria-label="نسبة تحميل بيانات اللعبة"
              >
                <div
                  className="h-full rounded-full bg-gradient-to-l from-emerald-400 to-emerald-300 transition-all duration-700"
                  style={{ width: `${autopilot.downloadPct}%` }}
                />
              </div>
              <span className="font-mono text-xs tabular-nums text-emerald-300">
                {autopilot.downloadPct}%
              </span>
            </div>
          )}
          <span className="font-mono text-[11px] text-zinc-500" dir="ltr">
            {arNum(autopilot.stepsAnswered)} خطوة آلية
            {autopilot.vlmFailures > 0 &&
              ` • ${arNum(autopilot.vlmFailures)} فشل تحليل`}
          </span>
        </div>

        {/* معرض الأدلة */}
        <div className="grid grid-cols-2 gap-3">
          <EvidenceTile
            evidence={autopilot.lobbyEvidence}
            title="داخل اللوبي"
            icon={Home}
            now={now}
          />
          <EvidenceTile
            evidence={autopilot.matchEvidence}
            title="داخل المباراة"
            icon={Gamepad2}
            now={now}
          />
        </div>

        {/* قرارات الذكاء الاصطناعي */}
        {autopilot.decisions.length > 0 && (
          <div>
            <p className="mb-1.5 text-[11px] font-semibold text-zinc-500">
              آخر قرارات الطيار الآلي
            </p>
            <ul className="owen-scroll max-h-56 space-y-1.5 overflow-y-auto pl-1">
              {autopilot.decisions.map((d) => (
                <li
                  key={`${d.taskId}-${d.ts}`}
                  className="flex items-start gap-2 rounded-lg border border-white/[0.06] bg-white/[0.02] px-2.5 py-1.5"
                >
                  <span className="mt-0.5 shrink-0 rounded bg-amber-400/10 px-1.5 py-0.5 font-mono text-[10px] text-amber-300" dir="ltr">
                    #{d.seq}
                  </span>
                  <div className="min-w-0 flex-1">
                    <p className="truncate text-[11px] leading-relaxed text-zinc-300">
                      {d.summary}
                    </p>
                    <p className="mt-0.5 flex flex-wrap items-center gap-x-2 text-[10px] text-zinc-500" dir="ltr">
                      <span>{AUTOPILOT_SCREENS_AR[d.screen] ?? d.screen}</span>
                      <span>·</span>
                      <span>{d.action}{d.x !== null && d.y !== null ? ` (${d.x},${d.y})` : ''}</span>
                      <span>·</span>
                      <span>{Math.round(d.waitMs / 1000)}s</span>
                    </p>
                  </div>
                  <span className="shrink-0 text-[10px] text-zinc-600">
                    {relativeAr(new Date(d.ts).toISOString(), now)}
                  </span>
                </li>
              ))}
            </ul>
          </div>
        )}

        {/* تحليل الكراشات */}
        {autopilot.crashes.length > 0 && (
          <div>
            <p className="mb-1.5 flex items-center gap-1.5 text-[11px] font-semibold text-red-300">
              <Bug className="size-3.5" aria-hidden="true" />
              تحليل الكراشات (تلقائي)
            </p>
            <ul className="owen-scroll max-h-44 space-y-1.5 overflow-y-auto pl-1">
              {autopilot.crashes.map((c) => (
                <li
                  key={c.ts}
                  className="rounded-lg border border-red-400/20 bg-red-500/[0.06] px-2.5 py-2"
                >
                  <p className="text-[11px] leading-relaxed text-red-200">
                    {c.analysis}
                  </p>
                  <p className="mt-1 text-[10px] text-zinc-500">
                    {c.restarted ? '↻ أُعيد تشغيل التطبيق تلقائياً' : 'لم تُعِد التشغيل (خنق زمني)'}
                    {' • '}
                    {relativeAr(new Date(c.ts).toISOString(), now)}
                  </p>
                </li>
              ))}
            </ul>
          </div>
        )}

        {/* أدوات */}
        <div className="flex flex-wrap items-center gap-x-4 gap-y-2 border-t border-white/[0.07] pt-3">
          <label className="flex cursor-pointer items-center gap-2 text-[11px] text-zinc-400">
            <Switch
              checked={autopilot.autoSdk}
              disabled={busy}
              onCheckedChange={(v) => void send({ autoSdk: v })}
              aria-label="توليد SDK تلقائياً بعد المباراة"
              className="scale-90"
            />
            توليد SDK بعد المباراة
          </label>
          <label className="flex cursor-pointer items-center gap-2 text-[11px] text-zinc-400">
            <Switch
              checked={autopilot.autoRestart}
              disabled={busy}
              onCheckedChange={(v) => void send({ autoRestart: v })}
              aria-label="إعادة تشغيل تلقائية عند الكراش"
              className="scale-90"
            />
            إعادة تشغيل عند الكراش
          </label>
          <Button
            variant="ghost"
            size="sm"
            disabled={busy}
            onClick={() => void send({ resetEvidence: true })}
            className="ms-auto h-8 gap-1.5 border border-white/10 bg-white/[0.04] text-[11px] text-zinc-300 hover:bg-white/10 hover:text-white"
          >
            <RotateCcw className="size-3.5" aria-hidden="true" />
            إعادة ضبط الأدلة
          </Button>
        </div>
      </CardContent>
    </Card>
  );
}
