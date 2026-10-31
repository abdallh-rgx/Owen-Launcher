"use client";

/* بطاقة البث المباشر — شاشة الهاتف لحظة بلحظة.
 *
 * الدرايفر (على GitHub Actions) يدفع إطار JPEG كل ~ثانية إلى
 * /api/control/live/frame، والصفحة تسحبه عبر long-poll فور وصوله.
 *
 * النقر على البث يحدد إحداثيات الزر (بالبكسل الفعلي للجهاز) ويغذي
 * نموذج الإجابة للطلب المعلّق مباشرة — بديل أسرع من لقطة الطلب الثابتة.
 */

import { useRef, useState } from "react";
import { Expand, Radio, Smartphone } from "lucide-react";
import type { LiveFrameDTO } from "@/lib/control-types";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Dialog, DialogContent, DialogTitle } from "@/components/ui/dialog";
import { cn } from "@/lib/utils";
import { arNum, type Point } from "./utils";

interface LiveStreamCardProps {
  frame: LiveFrameDTO | null;
  /** frame age in ms (updated every second by the parent) */
  ageMs: number | null;
  /** there is a PENDING annotation task right now */
  taskPending: boolean;
  onPick: (p: Point) => void;
}

const FRESH_MS = 6000;

export function LiveStreamCard({
  frame,
  ageMs,
  taskPending,
  onPick,
}: LiveStreamCardProps) {
  const imgRef = useRef<HTMLImageElement | null>(null);
  const [marker, setMarker] = useState<Point | null>(null);
  const [markerSeq, setMarkerSeq] = useState(0);
  const [full, setFull] = useState(false);

  // العلامة تختفي تلقائياً بعد ~15 إطار (~18 ثانية) — بلا effects
  const markerVisible =
    marker !== null && frame !== null && frame.seq - markerSeq < 15;

  const live = frame !== null && (ageMs ?? Infinity) < FRESH_MS;

  const handleClick = (e: React.MouseEvent<HTMLImageElement>) => {
    const img = e.currentTarget;
    const rect = img.getBoundingClientRect();
    const relX = (e.clientX - rect.left) / rect.width;
    const relY = (e.clientY - rect.top) / rect.height;
    if (relX < 0 || relX > 1 || relY < 0 || relY > 1) return;
    const dev =
      frame ??
      ({ deviceW: 1080, deviceH: 2280 } as unknown as LiveFrameDTO);
    const p: Point = {
      x: Math.round(relX * dev.deviceW),
      y: Math.round(relY * dev.deviceH),
    };
    setMarker(p);
    if (frame) setMarkerSeq(frame.seq);
    onPick(p);
  };

  const markerStyle = (dev: LiveFrameDTO | null) =>
    marker && dev
      ? {
          left: `${(marker.x / dev.deviceW) * 100}%`,
          top: `${(marker.y / dev.deviceH) * 100}%`,
        }
      : null;

  return (
    <Card className="gap-3 rounded-2xl border-white/10 bg-white/[0.03] py-4 shadow-2xl shadow-black/40 backdrop-blur-sm">
      <CardHeader className="gap-2 px-4">
        <div className="flex items-center justify-between gap-2">
          <CardTitle className="flex items-center gap-2 text-sm font-bold text-zinc-100">
            <span
              className={cn(
                "grid size-8 place-items-center rounded-lg border",
                live
                  ? "border-red-400/40 bg-red-500/10 text-red-300"
                  : "border-white/10 bg-white/[0.04] text-zinc-500",
              )}
            >
              <Radio
                className={cn("size-4", live && "animate-pulse")}
                aria-hidden="true"
              />
            </span>
            البث المباشر للهاتف
          </CardTitle>
          <Button
            type="button"
            variant="ghost"
            size="sm"
            onClick={() => setFull(true)}
            disabled={!frame}
            aria-label="عرض البث بالحجم الكامل"
            className="h-7 gap-1 px-2 text-[11px] text-zinc-400 hover:bg-white/10 hover:text-zinc-100"
          >
            <Expand className="size-3.5" aria-hidden="true" />
            تكبير
          </Button>
        </div>
        <div className="flex flex-wrap items-center gap-1.5 text-[10.5px] text-zinc-500">
          {frame ? (
            <>
              <span
                dir="ltr"
                className="inline-flex items-center gap-1 rounded-full border border-white/10 bg-black/40 px-2 py-0.5 font-mono tabular-nums text-zinc-400"
              >
                <Smartphone className="size-3" aria-hidden="true" />
                {arNum(frame.deviceW)}×{arNum(frame.deviceH)}
              </span>
              <span dir="ltr" className="font-mono tabular-nums">
                #{arNum(frame.seq)}
              </span>
              <span
                className={cn(
                  "font-mono tabular-nums",
                  live ? "text-emerald-400" : "text-amber-400",
                )}
              >
                {ageMs === null
                  ? "—"
                  : ageMs < 1000
                    ? "الآن"
                    : `قبل ${arNum(Math.round(ageMs / 1000))} ث`}
              </span>
              {frame.app && (
                <span dir="ltr" className="max-w-[120px] truncate font-mono text-zinc-500">
                  {frame.app}
                </span>
              )}
            </>
          ) : (
            <span>بانتظار أول إطار من المحاكي…</span>
          )}
        </div>
      </CardHeader>

      <CardContent className="px-4">
        <div
          className={cn(
            "relative mx-auto w-fit max-w-full overflow-hidden rounded-xl border",
            live ? "border-red-400/30" : "border-white/10",
            taskPending ? "cursor-crosshair" : "cursor-default",
          )}
        >
          {frame ? (
            <img
              ref={imgRef}
              src={frame.dataUrl}
              alt="بث مباشر لشاشة المحاكي"
              onClick={handleClick}
              className="block max-h-[520px] w-auto max-w-full select-none"
              draggable={false}
            />
          ) : (
            <div className="grid h-[320px] w-[180px] place-items-center rounded-xl border border-dashed border-white/15 bg-black/40">
              <Radio className="size-8 animate-pulse text-zinc-700" aria-hidden="true" />
            </div>
          )}

          {markerVisible && markerStyle(frame) && (
            <span
              className="pointer-events-none absolute z-10 -translate-x-1/2 -translate-y-1/2"
              style={markerStyle(frame) ?? undefined}
              aria-hidden="true"
            >
              <span className="absolute -inset-2 animate-ping rounded-full border border-red-400/60" />
              <span className="relative block size-3 rounded-full border-2 border-white bg-red-500 shadow-lg" />
            </span>
          )}

          {live && (
            <span className="pointer-events-none absolute start-2 top-2 inline-flex items-center gap-1 rounded-full bg-red-600/90 px-2 py-0.5 text-[10px] font-bold text-white shadow">
              <span className="size-1.5 animate-pulse rounded-full bg-white" />
              LIVE
            </span>
          )}
        </div>

        <p className="mt-2 text-center text-[11px] leading-relaxed text-zinc-500">
          {taskPending
            ? "انقر على مكان الزر في البث — الإحداثيات ستُعبّأ في نموذج الطلب بالأسفل"
            : "البث للمراقبة — عند وصول طلب تعليق يمكنك النقر هنا لتحديد الزر"}
        </p>
      </CardContent>

      <Dialog open={full} onOpenChange={setFull}>
        <DialogContent className="max-w-[92vw] border-white/10 bg-[#05060b] p-3 sm:max-w-[480px]">
          <DialogTitle className="sr-only">البث المباشر بالحجم الكامل</DialogTitle>
          {frame && (
            <div className="relative mx-auto w-fit">
              <img
                src={frame.dataUrl}
                alt="بث مباشر بالحجم الكامل"
                onClick={handleClick}
                className={cn(
                  "block max-h-[80vh] w-auto max-w-full select-none rounded-xl",
                  taskPending && "cursor-crosshair",
                )}
                draggable={false}
              />
              {markerVisible && markerStyle(frame) && (
                <span
                  className="pointer-events-none absolute z-10 -translate-x-1/2 -translate-y-1/2"
                  style={markerStyle(frame) ?? undefined}
                  aria-hidden="true"
                >
                  <span className="relative block size-4 rounded-full border-2 border-white bg-red-500 shadow-lg" />
                </span>
              )}
            </div>
          )}
        </DialogContent>
      </Dialog>
    </Card>
  );
}
