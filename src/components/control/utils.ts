// Owen Control Room — client-side helpers (formatting, beep, shared UI types).
// Pure functions only; imported exclusively by client components.

import type {
  ControlAction,
  ControlStatus,
  TaskDTO,
} from "@/lib/control-types";

/** A point in emulator-screen pixels (physical coordinates — never RTL-flipped). */
export interface Point {
  x: number;
  y: number;
}

/** Body accepted by POST /api/control/tasks/[id]/answer. */
export interface AnswerPayload {
  action: ControlAction;
  x?: number;
  y?: number;
  x2?: number;
  y2?: number;
  description?: string;
  waitMs?: number;
}

/** Result of a mutating API call, surfaced to the UI as a toast. */
export interface ApiResult {
  ok: boolean;
  error?: string;
}

export function clamp(v: number, min: number, max: number): number {
  return Math.min(max, Math.max(min, v));
}

/** Arabic-Indic digits (٠١٢…) — used for counts and durations. */
export function arNum(n: number): string {
  return new Intl.NumberFormat("ar").format(n);
}

/** Baghdad wall clock HH:MM:SS with Latin digits. */
export function baghdadTime(d: Date): string {
  return new Intl.DateTimeFormat("ar-IQ-u-nu-latn", {
    timeZone: "Asia/Baghdad",
    hour: "2-digit",
    minute: "2-digit",
    second: "2-digit",
    hourCycle: "h23",
  }).format(d);
}

/** Arabic relative time, e.g. "قبل ١٢ ثانية". */
export function relativeAr(iso: string, now: number): string {
  const t = new Date(iso).getTime();
  if (!Number.isFinite(t)) return "—";
  const diffSec = Math.round((t - now) / 1000);
  const rtf = new Intl.RelativeTimeFormat("ar", { numeric: "auto" });
  const abs = Math.abs(diffSec);
  if (abs < 60) return rtf.format(diffSec, "second");
  if (abs < 3600) return rtf.format(Math.round(diffSec / 60), "minute");
  if (abs < 86400) return rtf.format(Math.round(diffSec / 3600), "hour");
  return rtf.format(Math.round(diffSec / 86400), "day");
}

/** Arabic pluralisation helper: 1 / 2 / 3-10 / 11+ forms. */
function arUnit(
  value: number,
  forms: [singular: string, dual: string, few: string, many: string],
): string {
  if (value === 1) return forms[0];
  if (value === 2) return forms[1];
  if (!Number.isInteger(value)) return `${arNum(value)} ${forms[0]}`;
  return value >= 3 && value <= 10
    ? `${arNum(value)} ${forms[2]}`
    : `${arNum(value)} ${forms[3]}`;
}

/** Human label for a wait duration, e.g. 2000 → "ثانيتان". */
export function waitMsLabel(ms: number | null | undefined): string {
  if (ms === null || ms === undefined) return "";
  if (ms <= 0) return "بلا انتظار";
  if (ms < 1000) return `${arNum(ms)} مللي`;
  const s = ms / 1000;
  if (s < 60) return arUnit(s, ["ثانية", "ثانيتان", "ثوان", "ثانية"]);
  return arUnit(s / 60, ["دقيقة", "دقيقتان", "دقائق", "دقيقة"]);
}

/** Quick wait-duration chips shown under the wait input. */
export const WAIT_PRESETS: ReadonlyArray<{ ms: number; label: string }> = [
  { ms: 500, label: "٠٫٥ ثانية" },
  { ms: 1000, label: "ثانية" },
  { ms: 2000, label: "ثانيتان" },
  { ms: 5000, label: "٥ ثوان" },
  { ms: 10000, label: "١٠ ثوان" },
  { ms: 30000, label: "٣٠ ثانية" },
  { ms: 60000, label: "دقيقة" },
];

export const STATUS_LABELS_AR: Record<ControlStatus, string> = {
  PENDING: "قيد الانتظار",
  ANSWERED: "تم الرد",
  APPLIED: "تم التنفيذ",
  FAILED: "فشل",
  CANCELLED: "ملغي",
};

export const STATUS_BADGE_CLASSES: Record<ControlStatus, string> = {
  PENDING: "border-amber-400/40 bg-amber-400/10 text-amber-300",
  ANSWERED: "border-emerald-400/40 bg-transparent text-emerald-300",
  APPLIED: "border-emerald-400/50 bg-emerald-500/90 font-semibold text-black",
  FAILED: "border-red-400/40 bg-red-500/10 text-red-300",
  CANCELLED: "border-zinc-500/40 bg-zinc-500/10 text-zinc-400",
};

/** Short LTR summary for history rows, e.g. "tap (640, 360)". */
export function actionSummary(t: TaskDTO): string {
  const p = (v: number | null) => (v === null ? "?" : String(v));
  switch (t.action) {
    case "tap":
      return `tap (${p(t.x)}, ${p(t.y)})`;
    case "swipe":
      return `swipe (${p(t.x)}, ${p(t.y)})→(${p(t.x2)}, ${p(t.y2)})`;
    case "wait":
      return t.waitMs !== null ? `wait ${t.waitMs}ms` : "wait";
    case "back":
      return "back";
    case "home":
      return "home";
    case "done":
      return "done";
    case "skip":
      return "skip";
    default:
      return "—";
  }
}

/** Two quick 880Hz/1320Hz notes (~150ms) announcing a fresh task. */
export function beepNewTask(): void {
  try {
    const AC =
      window.AudioContext ??
      (window as unknown as { webkitAudioContext?: typeof AudioContext })
        .webkitAudioContext;
    if (!AC) return;
    const ctx = new AC();
    const t0 = ctx.currentTime;
    const note = (freq: number, start: number, dur: number) => {
      const osc = ctx.createOscillator();
      const gain = ctx.createGain();
      osc.type = "sine";
      osc.frequency.setValueAtTime(freq, t0 + start);
      gain.gain.setValueAtTime(0.0001, t0 + start);
      gain.gain.exponentialRampToValueAtTime(0.22, t0 + start + 0.012);
      gain.gain.exponentialRampToValueAtTime(0.0001, t0 + start + dur);
      osc.connect(gain);
      gain.connect(ctx.destination);
      osc.start(t0 + start);
      osc.stop(t0 + start + dur + 0.02);
    };
    note(880, 0, 0.07);
    note(1320, 0.075, 0.075);
    window.setTimeout(() => {
      void ctx.close().catch(() => undefined);
    }, 400);
  } catch {
    /* audio unavailable — stay silent */
  }
}
