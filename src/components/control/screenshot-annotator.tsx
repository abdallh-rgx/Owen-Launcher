"use client";

import { useRef, useState } from "react";
import type { KeyboardEvent, MouseEvent } from "react";
import { Camera, Loader2, MousePointerClick } from "lucide-react";
import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";
import { clamp, type Point } from "./utils";

interface HoverInfo {
  /** cursor position in physical px, relative to the image box */
  px: number;
  py: number;
  /** cursor position in image (emulator) pixel space */
  ix: number;
  iy: number;
  /** rendered image box size in px (for readout clamping) */
  w: number;
  h: number;
}

interface ScreenshotAnnotatorProps {
  imageUrl: string;
  imageW: number | null;
  imageH: number | null;
  /** false when the chosen action needs no coordinates → dim + ignore clicks */
  interactive: boolean;
  /** swipe mode: two markers (start/end) + arrow */
  swipeMode: boolean;
  p1: Point | null;
  p2: Point | null;
  activePoint: 1 | 2;
  onActivePointChange: (p: 1 | 2) => void;
  onSetPoint: (which: 1 | 2, pt: Point) => void;
  wantsRetake: boolean;
  retaking: boolean;
  onRetake: () => void;
}

function Marker({
  pt,
  imgW,
  imgH,
  tone,
  text,
}: {
  pt: Point;
  imgW: number;
  imgH: number;
  tone: "red" | "green";
  text: string;
}) {
  const red = tone === "red";
  return (
    <div
      className="absolute -translate-x-1/2 -translate-y-1/2"
      style={{
        left: `${(pt.x / imgW) * 100}%`,
        top: `${(pt.y / imgH) * 100}%`,
      }}
    >
      {/* نبضة */}
      <span
        className={cn(
          "absolute left-1/2 top-1/2 size-5 -translate-x-1/2 -translate-y-1/2 animate-ping rounded-full opacity-60",
          red ? "bg-red-500/60" : "bg-emerald-500/60",
        )}
      />
      {/* حلقة */}
      <span
        className={cn(
          "absolute left-1/2 top-1/2 size-5 -translate-x-1/2 -translate-y-1/2 rounded-full border-2",
          red ? "border-red-400 bg-red-500/20" : "border-emerald-400 bg-emerald-500/20",
        )}
      />
      {/* مركز */}
      <span
        className={cn(
          "absolute left-1/2 top-1/2 size-1.5 -translate-x-1/2 -translate-y-1/2 rounded-full",
          red ? "bg-red-400" : "bg-emerald-400",
        )}
      />
      <span
        dir="ltr"
        className="absolute left-1/2 top-full mt-1.5 -translate-x-1/2 whitespace-nowrap rounded-md border border-white/15 bg-black/85 px-1.5 py-0.5 font-mono text-[10px] text-zinc-100"
      >
        {text}
      </span>
    </div>
  );
}

export function ScreenshotAnnotator({
  imageUrl,
  imageW,
  imageH,
  interactive,
  swipeMode,
  p1,
  p2,
  activePoint,
  onActivePointChange,
  onSetPoint,
  wantsRetake,
  retaking,
  onRetake,
}: ScreenshotAnnotatorProps) {
  const imgRef = useRef<HTMLImageElement>(null);
  const [fitMode, setFitMode] = useState(true);
  const [natural, setNatural] = useState<{ w: number; h: number } | null>(null);
  const [hover, setHover] = useState<HoverInfo | null>(null);

  const imgW = imageW ?? natural?.w ?? null;
  const imgH = imageH ?? natural?.h ?? null;

  /** Map a physical cursor position to image-pixel coordinates (never RTL-flipped). */
  const toImage = (e: { clientX: number; clientY: number }): HoverInfo | null => {
    const img = imgRef.current;
    if (!img || !imgW || !imgH) return null;
    const rect = img.getBoundingClientRect();
    const ix = clamp(
      Math.round(((e.clientX - rect.left) / rect.width) * imgW),
      0,
      imgW,
    );
    const iy = clamp(
      Math.round(((e.clientY - rect.top) / rect.height) * imgH),
      0,
      imgH,
    );
    return {
      px: e.clientX - rect.left,
      py: e.clientY - rect.top,
      ix,
      iy,
      w: rect.width,
      h: rect.height,
    };
  };

  const handleMove = (e: MouseEvent) => {
    if (!interactive) return;
    setHover(toImage(e));
  };

  const handleClick = (e: MouseEvent) => {
    if (!interactive || !imgW || !imgH) return;
    const pt = toImage(e);
    if (!pt) return;
    if (swipeMode) {
      onSetPoint(activePoint, { x: pt.ix, y: pt.iy });
      if (activePoint === 1) onActivePointChange(2);
    } else {
      onSetPoint(1, { x: pt.ix, y: pt.iy });
    }
  };

  const handleKeyDown = (e: KeyboardEvent<HTMLDivElement>) => {
    if (!interactive || !imgW || !imgH) return;
    const which: 1 | 2 = swipeMode ? activePoint : 1;
    const cur = which === 1 ? p1 : p2;
    if (!cur) return;
    const step = e.shiftKey ? 10 : 1;
    let { x, y } = cur;
    switch (e.key) {
      case "ArrowLeft":
        x -= step;
        break;
      case "ArrowRight":
        x += step;
        break;
      case "ArrowUp":
        y -= step;
        break;
      case "ArrowDown":
        y += step;
        break;
      default:
        return;
    }
    e.preventDefault();
    onSetPoint(which, { x: clamp(x, 0, imgW), y: clamp(y, 0, imgH) });
  };

  const retakeBusy = wantsRetake || retaking;

  // Arrow (start → end) for swipe mode, drawn in image-pixel space.
  let arrow: { line: { x1: number; y1: number; x2: number; y2: number }; head: string } | null = null;
  if (swipeMode && p1 && p2 && imgW && imgH) {
    const dx = p2.x - p1.x;
    const dy = p2.y - p1.y;
    const len = Math.hypot(dx, dy);
    if (len > 2) {
      const ux = dx / len;
      const uy = dy / len;
      const headLen = Math.min(36, len * 0.45);
      const bx = p2.x - ux * headLen;
      const by = p2.y - uy * headLen;
      const nx = -uy;
      const ny = ux;
      const hw = headLen * 0.42;
      arrow = {
        line: { x1: p1.x, y1: p1.y, x2: bx, y2: by },
        head: `${p2.x},${p2.y} ${bx + nx * hw},${by + ny * hw} ${bx - nx * hw},${by - ny * hw}`,
      };
    }
  }

  return (
    <div className="space-y-2">
      {/* شريط الأدوات */}
      <div className="flex flex-wrap items-center gap-2">
        <div
          role="group"
          aria-label="وضع العرض"
          className="flex rounded-lg border border-white/10 bg-white/[0.03] p-0.5"
        >
          <button
            type="button"
            onClick={() => setFitMode(true)}
            aria-pressed={fitMode}
            className={cn(
              "rounded-md px-2.5 py-1 text-[11px] font-medium transition focus-visible:outline-2 focus-visible:outline-amber-400",
              fitMode
                ? "bg-amber-400/20 text-amber-200"
                : "text-zinc-400 hover:text-zinc-200",
            )}
          >
            ملاءمة
          </button>
          <button
            type="button"
            onClick={() => setFitMode(false)}
            aria-pressed={!fitMode}
            className={cn(
              "rounded-md px-2.5 py-1 text-[11px] font-medium transition focus-visible:outline-2 focus-visible:outline-amber-400",
              !fitMode
                ? "bg-amber-400/20 text-amber-200"
                : "text-zinc-400 hover:text-zinc-200",
            )}
          >
            ١٠٠٪
          </button>
        </div>

        {imgW !== null && imgH !== null && (
          <span
            dir="ltr"
            className="rounded-md border border-white/10 bg-white/[0.03] px-2 py-1 font-mono text-[11px] text-zinc-400"
          >
            {imgW}×{imgH}
          </span>
        )}

        <Button
          type="button"
          variant="outline"
          size="sm"
          onClick={onRetake}
          disabled={retakeBusy}
          className="h-8 border-amber-400/30 bg-amber-400/[0.06] text-xs text-amber-300 hover:bg-amber-400/15 hover:text-amber-200"
        >
          {retakeBusy ? (
            <Loader2 className="size-3.5 animate-spin" aria-hidden="true" />
          ) : (
            <Camera className="size-3.5" aria-hidden="true" />
          )}
          {retakeBusy ? "بانتظار لقطة جديدة…" : "لقطة جديدة"}
        </Button>

        {swipeMode && (
          <div
            role="group"
            aria-label="النقطة النشطة"
            className="flex rounded-lg border border-white/10 bg-white/[0.03] p-0.5"
          >
            <button
              type="button"
              onClick={() => onActivePointChange(1)}
              aria-pressed={activePoint === 1}
              className={cn(
                "rounded-md px-2.5 py-1 text-[11px] font-medium transition focus-visible:outline-2 focus-visible:outline-emerald-400",
                activePoint === 1
                  ? "bg-emerald-400/20 text-emerald-200"
                  : "text-zinc-400 hover:text-zinc-200",
              )}
            >
              النقطة ١ (البداية)
            </button>
            <button
              type="button"
              onClick={() => onActivePointChange(2)}
              aria-pressed={activePoint === 2}
              className={cn(
                "rounded-md px-2.5 py-1 text-[11px] font-medium transition focus-visible:outline-2 focus-visible:outline-red-400",
                activePoint === 2
                  ? "bg-red-400/20 text-red-200"
                  : "text-zinc-400 hover:text-zinc-200",
              )}
            >
              النقطة ٢ (النهاية)
            </button>
          </div>
        )}

        {interactive && (
          <span className="ms-auto hidden items-center gap-1.5 text-[11px] text-amber-300/80 sm:flex">
            <MousePointerClick className="size-3.5" aria-hidden="true" />
            انقر على مكان الزر في الصورة
          </span>
        )}
      </div>

      {/* الصورة */}
      <div
        className={cn(
          "rounded-xl border border-white/10 bg-black",
          fitMode
            ? "relative overflow-hidden"
            : "max-h-[78vh] overflow-auto [&::-webkit-scrollbar]:h-1.5 [&::-webkit-scrollbar]:w-1.5 [&::-webkit-scrollbar-thumb]:rounded-full [&::-webkit-scrollbar-thumb]:bg-white/15",
        )}
      >
        <div
          className={cn(
            "relative outline-none focus-visible:ring-2 focus-visible:ring-amber-400/70",
            fitMode ? "block" : "inline-block",
            interactive ? "cursor-crosshair" : "cursor-default",
          )}
          tabIndex={0}
          role="group"
          aria-label="منطقة تحديد الزر — انقر لتحديد الموقع، والأسهم لتحريك آخر نقطة بدقة"
          onMouseMove={handleMove}
          onMouseLeave={() => setHover(null)}
          onClick={handleClick}
          onKeyDown={handleKeyDown}
        >
          <img
            ref={imgRef}
            src={imageUrl}
            alt="لقطة شاشة المحاكي — انقر لتحديد مكان الزر"
            draggable={false}
            onLoad={(e) =>
              setNatural({
                w: e.currentTarget.naturalWidth,
                h: e.currentTarget.naturalHeight,
              })
            }
            className={cn(
              "block select-none",
              fitMode ? "h-auto w-full" : "h-auto max-w-none",
            )}
            style={fitMode ? undefined : { width: imgW ?? undefined }}
          />

          {/* طبقة القياس (لا تعترض الفأرة) */}
          <div className="pointer-events-none absolute inset-0">
            {interactive && hover && imgW !== null && imgH !== null && (
              <>
                {/* خطا التصويب */}
                <div
                  className="absolute inset-y-0 w-px bg-amber-400/40"
                  style={{ left: `${(hover.ix / imgW) * 100}%` }}
                />
                <div
                  className="absolute inset-x-0 h-px bg-amber-400/40"
                  style={{ top: `${(hover.iy / imgH) * 100}%` }}
                />
                {/* قراءة الإحداثيات بجوار المؤشر */}
                <div
                  dir="ltr"
                  className="absolute rounded-md border border-amber-400/40 bg-black/85 px-1.5 py-0.5 font-mono text-[10px] leading-4 text-amber-200"
                  style={{
                    left: Math.max(2, Math.min(hover.px + 12, hover.w - 78)),
                    top: Math.max(2, Math.min(hover.py + 12, hover.h - 22)),
                  }}
                >
                  ({hover.ix}, {hover.iy})
                </div>
              </>
            )}

            {arrow && imgW !== null && imgH !== null && (
              <svg
                className="absolute inset-0 h-full w-full"
                viewBox={`0 0 ${imgW} ${imgH}`}
                preserveAspectRatio="none"
                aria-hidden="true"
              >
                <line
                  x1={arrow.line.x1}
                  y1={arrow.line.y1}
                  x2={arrow.line.x2}
                  y2={arrow.line.y2}
                  stroke="#fbbf24"
                  strokeWidth={2.5}
                  strokeLinecap="round"
                  vectorEffect="non-scaling-stroke"
                />
                <polygon points={arrow.head} fill="#fbbf24" />
              </svg>
            )}

            {imgW !== null && imgH !== null && (
              <>
                {swipeMode ? (
                  <>
                    {p1 && (
                      <Marker
                        pt={p1}
                        imgW={imgW}
                        imgH={imgH}
                        tone="green"
                        text={`1 (${p1.x}, ${p1.y})`}
                      />
                    )}
                    {p2 && (
                      <Marker
                        pt={p2}
                        imgW={imgW}
                        imgH={imgH}
                        tone="red"
                        text={`2 (${p2.x}, ${p2.y})`}
                      />
                    )}
                  </>
                ) : (
                  p1 && (
                    <Marker
                      pt={p1}
                      imgW={imgW}
                      imgH={imgH}
                      tone="red"
                      text={`X: ${p1.x} Y: ${p1.y}`}
                    />
                  )
                )}
              </>
            )}
          </div>

          {/* تعتيم عندما لا يحتاج الإجراء إلى إحداثيات */}
          {!interactive && (
            <div className="absolute inset-0 bg-black/45" aria-hidden="true" />
          )}
        </div>
      </div>
    </div>
  );
}
