"use client";

import { useState } from "react";
import type { ReactNode } from "react";
import {
  Activity,
  ChevronDown,
  Cpu,
  Terminal,
  Timer,
} from "lucide-react";
import { HEARTBEAT_STALE_MS, type ControlStats, type HeartbeatDTO } from "@/lib/control-types";
import { Badge } from "@/components/ui/badge";
import {
  Card,
  CardContent,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import {
  Collapsible,
  CollapsibleContent,
  CollapsibleTrigger,
} from "@/components/ui/collapsible";
import { cn } from "@/lib/utils";
import { arNum, relativeAr } from "./utils";

interface PipelineStatusProps {
  heartbeat: HeartbeatDTO | null;
  stats: ControlStats | null;
}

function stateBadgeClass(state: string): string {
  const s = state.toUpperCase();
  if (s.includes("RUN") || s.includes("LIVE") || s.includes("OK") || s.includes("ONLINE"))
    return "border-emerald-400/40 bg-emerald-400/10 text-emerald-300";
  if (s.includes("BOOT") || s.includes("START") || s.includes("WAIT") || s.includes("INIT"))
    return "border-amber-400/40 bg-amber-400/10 text-amber-300";
  if (s.includes("ERR") || s.includes("FAIL") || s.includes("STOP"))
    return "border-red-400/40 bg-red-500/10 text-red-300";
  return "border-zinc-500/40 bg-zinc-500/10 text-zinc-400";
}

export function PipelineStatus({ heartbeat, stats }: PipelineStatusProps) {
  const [logOpen, setLogOpen] = useState(false);

  const live =
    heartbeat !== null &&
    Date.now() - new Date(heartbeat.updatedAt).getTime() < HEARTBEAT_STALE_MS;

  const row = (label: string, value: ReactNode, mono = false) => (
    <div className="flex items-start justify-between gap-3 text-xs">
      <span className="shrink-0 text-zinc-500">{label}</span>
      <span
        className={cn(
          "min-w-0 text-left font-medium text-zinc-200",
          mono && "font-mono text-[11px]",
        )}
        dir={mono ? "ltr" : undefined}
      >
        {value}
      </span>
    </div>
  );

  return (
    <Card className="gap-4 rounded-2xl border-white/10 bg-white/[0.03] py-5 shadow-xl shadow-black/30 backdrop-blur-sm">
      <CardHeader className="px-4 sm:px-5">
        <CardTitle className="flex items-center gap-2 text-sm font-bold text-zinc-100">
          <span className="relative grid size-8 place-items-center rounded-lg border border-white/10 bg-white/[0.04] text-zinc-300">
            <Activity className="size-4" aria-hidden="true" />
            <span
              className={cn(
                "absolute -end-0.5 -top-0.5 size-2 rounded-full",
                heartbeat ? (live ? "animate-pulse bg-emerald-400" : "bg-amber-400") : "bg-zinc-600",
              )}
              aria-hidden="true"
            />
          </span>
          حالة الخط
        </CardTitle>
      </CardHeader>

      <CardContent className="space-y-3 px-4 sm:px-5">
        {heartbeat ? (
          <div className="space-y-2.5">
            <div className="flex items-center justify-between gap-3">
              {row(
                "الحالة",
                <Badge className={stateBadgeClass(heartbeat.state)}>
                  {heartbeat.state}
                </Badge>,
              )}
            </div>
            {heartbeat.phase && row("المرحلة", heartbeat.phase, true)}
            {heartbeat.device && row("الجهاز", heartbeat.device, true)}
            {heartbeat.message && (
              <p className="rounded-lg border border-white/10 bg-black/30 p-2.5 text-xs leading-relaxed text-zinc-300">
                {heartbeat.message}
              </p>
            )}
            {row("آخر نبضة", relativeAr(heartbeat.updatedAt, Date.now()))}
          </div>
        ) : (
          <div className="flex items-center gap-2.5 rounded-lg border border-dashed border-white/15 bg-black/20 p-3 text-xs text-zinc-500">
            <Cpu className="size-4 shrink-0" aria-hidden="true" />
            لم يتصل الـ Action بعد — شغّل الجلسة من بطاقة التشغيل.
          </div>
        )}

        {stats && (
          <div className="grid grid-cols-5 gap-1.5">
            {(
              [
                { label: "انتظار", value: stats.pending, cls: "text-amber-300 border-amber-400/25 bg-amber-400/[0.06]" },
                { label: "رُدّت", value: stats.answered, cls: "text-emerald-300 border-emerald-400/25 bg-emerald-400/[0.06]" },
                { label: "نُفّذت", value: stats.applied, cls: "text-emerald-200 border-emerald-400/25 bg-emerald-400/[0.1]" },
                { label: "فشل", value: stats.failed, cls: "text-red-300 border-red-400/25 bg-red-400/[0.06]" },
                { label: "الكل", value: stats.total, cls: "text-zinc-300 border-white/10 bg-white/[0.03]" },
              ] as const
            ).map((s) => (
              <div
                key={s.label}
                className={cn("rounded-lg border px-1 py-1.5 text-center", s.cls)}
              >
                <div className="text-sm font-bold leading-none">{arNum(s.value)}</div>
                <div className="mt-1 text-[10px] leading-none text-zinc-500">{s.label}</div>
              </div>
            ))}
          </div>
        )}

        {heartbeat && heartbeat.logTail && (
          <Collapsible open={logOpen} onOpenChange={setLogOpen}>
            <CollapsibleTrigger asChild>
              <button
                type="button"
                className="flex w-full items-center justify-center gap-1.5 rounded-lg border border-white/10 bg-white/[0.03] py-1.5 text-[11px] text-zinc-400 transition hover:bg-white/[0.06] hover:text-zinc-200 focus-visible:outline-2 focus-visible:outline-amber-400"
                aria-expanded={logOpen}
              >
                <Terminal className="size-3.5" aria-hidden="true" />
                سجل الخط
                <ChevronDown
                  className={cn("size-3.5 transition-transform", logOpen && "rotate-180")}
                  aria-hidden="true"
                />
              </button>
            </CollapsibleTrigger>
            <CollapsibleContent>
              <pre
                dir="ltr"
                className="mt-2 max-h-64 overflow-auto rounded-lg border border-white/10 bg-black/60 p-3 text-left font-mono text-[11px] leading-relaxed text-zinc-400 [&::-webkit-scrollbar]:w-1.5 [&::-webkit-scrollbar-thumb]:rounded-full [&::-webkit-scrollbar-thumb]:bg-white/15"
              >
                {heartbeat.logTail}
              </pre>
            </CollapsibleContent>
          </Collapsible>
        )}

        <p className="flex items-center gap-1.5 text-[10px] text-zinc-600">
          <Timer className="size-3" aria-hidden="true" />
          يفحص الموقع حالة الخط كل ثانيتين تلقائياً.
        </p>
      </CardContent>
    </Card>
  );
}
