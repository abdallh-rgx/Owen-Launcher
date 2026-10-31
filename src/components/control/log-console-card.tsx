"use client";

/* السجل المباشر — logcat المحاكي + سجل الدرايفر مباشرة على الموقع.
 * أسطر الكراش (FATAL/tombstone) تظهر بالأحمر المتوهج حتى ترصدها فوراً.
 */

import { useEffect, useMemo, useRef, useState } from "react";
import { Copy, ScrollText, Trash2 } from "lucide-react";
import type { LiveLogLineDTO, LiveLogLevel } from "@/lib/control-types";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { cn } from "@/lib/utils";
import { arNum } from "./utils";

interface LogConsoleCardProps {
  lines: LiveLogLineDTO[];
  liveConnected: boolean;
}

type LevelFilter = "ALL" | "ERROR" | "DRIVER";

const LEVEL_CLASSES: Record<LiveLogLevel, string> = {
  INFO: "text-zinc-400",
  WARN: "text-amber-300",
  ERROR: "text-red-300",
  CRASH: "bg-red-500/15 font-bold text-red-300",
  DRIVER: "text-cyan-300",
};

export function LogConsoleCard({ lines, liveConnected }: LogConsoleCardProps) {
  const [filter, setFilter] = useState("");
  const [levelFilter, setLevelFilter] = useState<LevelFilter>("ALL");
  const [autoScroll, setAutoScroll] = useState(true);
  const [hidden, setHidden] = useState(false);
  const boxRef = useRef<HTMLDivElement | null>(null);
  const crashCount = useMemo(
    () => lines.filter((l) => l.level === "CRASH").length,
    [lines],
  );

  const shown = useMemo(() => {
    let out = lines;
    if (levelFilter === "ERROR") out = out.filter((l) => l.level === "ERROR" || l.level === "CRASH");
    if (levelFilter === "DRIVER") out = out.filter((l) => l.level === "DRIVER");
    if (filter.trim()) {
      const q = filter.trim().toLowerCase();
      out = out.filter((l) => l.text.toLowerCase().includes(q));
    }
    return out.slice(-400);
  }, [lines, filter, levelFilter]);

  useEffect(() => {
    if (autoScroll && boxRef.current) {
      boxRef.current.scrollTop = boxRef.current.scrollHeight;
    }
  }, [shown, autoScroll]);

  const copyAll = async () => {
    try {
      await navigator.clipboard.writeText(
        shown.map((l) => `[${l.level}] ${l.text}`).join("\n"),
      );
    } catch {
      /* ignore */
    }
  };

  if (hidden) {
    return (
      <button
        type="button"
        onClick={() => setHidden(false)}
        className="mx-auto flex items-center gap-2 rounded-full border border-white/10 bg-white/[0.04] px-4 py-2 text-[12px] text-zinc-300 hover:bg-white/10"
      >
        <ScrollText className="size-4" aria-hidden="true" />
        إظهار السجل المباشر ({arNum(lines.length)} سطر)
      </button>
    );
  }

  return (
    <Card className="gap-3 rounded-2xl border-white/10 bg-white/[0.03] py-4 shadow-xl shadow-black/30 backdrop-blur-sm">
      <CardHeader className="px-4">
        <div className="flex flex-wrap items-center gap-2">
          <CardTitle className="flex items-center gap-2 text-sm font-bold text-zinc-100">
            <span className="grid size-8 place-items-center rounded-lg border border-white/10 bg-white/[0.04] text-zinc-300">
              <ScrollText className="size-4" aria-hidden="true" />
            </span>
            السجل المباشر (logcat + الدرايفر)
          </CardTitle>

          <span
            role="status"
            className={cn(
              "inline-flex items-center gap-1.5 rounded-full border px-2 py-0.5 text-[10.5px] font-medium",
              liveConnected
                ? "border-emerald-400/30 bg-emerald-400/10 text-emerald-300"
                : "border-white/10 bg-white/[0.04] text-zinc-500",
            )}
          >
            <span
              className={cn(
                "size-1.5 rounded-full",
                liveConnected ? "animate-pulse bg-emerald-400" : "bg-zinc-600",
              )}
              aria-hidden="true"
            />
            {liveConnected ? "يتدفق" : "متوقف"}
          </span>

          {crashCount > 0 && (
            <span className="inline-flex items-center gap-1 rounded-full border border-red-400/40 bg-red-500/15 px-2 py-0.5 text-[10.5px] font-bold text-red-300">
              🚨 {arNum(crashCount)} سطر كراش
            </span>
          )}

          <div className="ms-auto flex items-center gap-1.5">
            <div className="flex items-center gap-0.5 rounded-lg border border-white/10 bg-black/40 p-0.5">
              {(["ALL", "ERROR", "DRIVER"] as const).map((lv) => (
                <button
                  key={lv}
                  type="button"
                  onClick={() => setLevelFilter(lv)}
                  aria-pressed={levelFilter === lv}
                  className={cn(
                    "rounded-md px-2 py-0.5 text-[10.5px] font-semibold transition",
                    levelFilter === lv
                      ? "bg-white/15 text-zinc-100"
                      : "text-zinc-500 hover:text-zinc-200",
                  )}
                >
                  {lv === "ALL" ? "الكل" : lv === "ERROR" ? "أخطاء" : "الدرايفر"}
                </button>
              ))}
            </div>
            <label className="flex cursor-pointer items-center gap-1 text-[10.5px] text-zinc-400">
              <input
                type="checkbox"
                checked={autoScroll}
                onChange={(e) => setAutoScroll(e.target.checked)}
                className="size-3 accent-amber-400"
              />
              تمرير تلقائي
            </label>
            <Button
              type="button"
              variant="ghost"
              size="sm"
              onClick={() => void copyAll()}
              aria-label="نسخ السجل"
              className="h-7 w-7 p-0 text-zinc-400 hover:bg-white/10 hover:text-zinc-100"
            >
              <Copy className="size-3.5" aria-hidden="true" />
            </Button>
            <Button
              type="button"
              variant="ghost"
              size="sm"
              onClick={() => setHidden(true)}
              aria-label="إخفاء السجل"
              className="h-7 w-7 p-0 text-zinc-400 hover:bg-white/10 hover:text-zinc-100"
            >
              <Trash2 className="size-3.5" aria-hidden="true" />
            </Button>
          </div>
        </div>
      </CardHeader>
      <CardContent className="px-4">
        <Input
          value={filter}
          onChange={(e) => setFilter(e.target.value)}
          placeholder="تصفية السجل… (مثال: FATAL أو OwenLauncher)"
          className="mb-2 h-8 border-white/10 bg-black/40 font-mono text-[11px] text-zinc-200 placeholder:text-zinc-600"
        />
        <div
          ref={boxRef}
          dir="ltr"
          className="max-h-80 overflow-y-auto rounded-xl border border-white/10 bg-black/60 p-2.5 font-mono text-[10.5px] leading-[1.55] owen-log-scroll"
          role="log"
          aria-live="polite"
        >
          {shown.length === 0 ? (
            <p className="p-4 text-center text-zinc-600" dir="rtl">
              لا أسطر بعد — سيظهر logcat المحاكي هنا لحظة اتصال الدرايفر.
            </p>
          ) : (
            shown.map((l) => (
              <div
                key={l.seq}
                className={cn(
                  "whitespace-pre-wrap break-all px-1",
                  LEVEL_CLASSES[l.level],
                )}
              >
                {l.level === "DRIVER" ? "🤖 " : l.level === "CRASH" ? "🚨 " : ""}
                {l.text}
              </div>
            ))
          )}
        </div>
      </CardContent>
    </Card>
  );
}
