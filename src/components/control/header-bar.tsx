"use client";

import { useEffect, useState } from "react";
import { Bell, BellOff, Clock3, Inbox, Radar, WifiOff } from "lucide-react";
import { HEARTBEAT_STALE_MS, type HeartbeatDTO } from "@/lib/control-types";
import { cn } from "@/lib/utils";
import { arNum, baghdadTime } from "./utils";

interface HeaderBarProps {
  heartbeat: HeartbeatDTO | null;
  pending: number;
  connError: boolean;
  muted: boolean;
  onToggleMute: () => void;
}

export function HeaderBar({
  heartbeat,
  pending,
  connError,
  muted,
  onToggleMute,
}: HeaderBarProps) {
  // Null until mount so SSR HTML and first client render match.
  const [now, setNow] = useState<Date | null>(null);

  useEffect(() => {
    setNow(new Date());
    const iv = window.setInterval(() => setNow(new Date()), 1000);
    return () => window.clearInterval(iv);
  }, []);

  const live =
    heartbeat !== null &&
    Date.now() - new Date(heartbeat.updatedAt).getTime() < HEARTBEAT_STALE_MS;

  return (
    <header className="sticky top-0 z-40 border-b border-white/10 bg-[#05060b]/85 backdrop-blur-md">
      <div className="mx-auto flex w-full max-w-7xl items-center justify-between gap-3 px-4 py-3 sm:px-6">
        {/* العنوان — الجهة اليمنى */}
        <div className="flex min-w-0 items-center gap-3">
          <div className="grid size-10 shrink-0 place-items-center rounded-xl border border-amber-400/30 bg-amber-400/10 text-amber-300 shadow-[0_0_24px_rgba(251,191,36,0.15)]">
            <Radar className="size-5" aria-hidden="true" />
          </div>
          <div className="min-w-0">
            <h1 className="truncate text-base font-extrabold tracking-tight sm:text-lg">
              غرفة تحكم أوين
            </h1>
            <p className="hidden truncate text-[11px] text-zinc-500 md:block">
              Owen Control Room — تحديد الأزرار على لقطات المحاكي
            </p>
          </div>
        </div>

        {/* الحالات — الجهة اليسرى */}
        <div className="flex items-center gap-1.5 sm:gap-2">
          {connError && (
            <span
              role="status"
              className="inline-flex items-center gap-1.5 rounded-full border border-red-400/40 bg-red-500/10 px-2.5 py-1 text-[11px] font-medium text-red-300"
            >
              <WifiOff className="size-3.5" aria-hidden="true" />
              <span className="hidden sm:inline">مشكلة اتصال بالموقع</span>
            </span>
          )}

          <span
            role="status"
            className={cn(
              "inline-flex items-center gap-1.5 rounded-full border px-2.5 py-1 text-[11px] font-medium",
              live
                ? "border-emerald-400/30 bg-emerald-400/10 text-emerald-300"
                : "border-amber-400/30 bg-amber-400/10 text-amber-300",
            )}
          >
            <span
              className={cn(
                "size-1.5 animate-pulse rounded-full",
                live ? "bg-emerald-400" : "bg-amber-400",
              )}
              aria-hidden="true"
            />
            {live ? "الخط متصل" : "بانتظار اتصال الـ Action…"}
          </span>

          <span
            role="status"
            aria-label={`طلبات بانتظار الرد: ${pending}`}
            className="inline-flex items-center gap-1.5 rounded-full border border-amber-400/30 bg-amber-400/10 px-2.5 py-1 text-[11px] font-semibold text-amber-300"
          >
            <Inbox className="size-3.5" aria-hidden="true" />
            {arNum(pending)}
          </span>

          <span
            dir="ltr"
            title="توقيت بغداد"
            className="hidden items-center gap-1.5 rounded-full border border-white/10 bg-white/[0.04] px-2.5 py-1 font-mono text-[11px] tabular-nums text-zinc-300 sm:inline-flex"
          >
            <Clock3 className="size-3.5 text-zinc-500" aria-hidden="true" />
            {now ? baghdadTime(now) : "--:--:--"}
          </span>

          <button
            type="button"
            onClick={onToggleMute}
            aria-label={muted ? "تشغيل صوت التنبيه" : "كتم صوت التنبيه"}
            aria-pressed={muted}
            className="grid size-8 place-items-center rounded-full border border-white/10 bg-white/[0.04] text-zinc-300 transition hover:bg-white/10 hover:text-white focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-amber-400"
          >
            {muted ? (
              <BellOff className="size-4" aria-hidden="true" />
            ) : (
              <Bell className="size-4" aria-hidden="true" />
            )}
          </button>
        </div>
      </div>
    </header>
  );
}
