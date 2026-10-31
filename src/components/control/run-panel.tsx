"use client";

/* لوحة إدارة التشغيل — أنا (الوكيل) والدرايفر على GitHub Actions:
 * تشغيل الجلسة، إيقافها، ومتابعة حالة الـ Run لحظياً —
 * كله عبر GitHub API من الخادم، بلا نسخ/لصق منك.
 */

import { useState } from "react";
import {
  CircleStop,
  ExternalLink,
  Loader2,
  Play,
  Rocket,
} from "lucide-react";
import type { RunStatusDTO } from "@/lib/control-types";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { cn } from "@/lib/utils";
import { arNum, relativeAr, type ApiResult } from "./utils";

interface RunPanelProps {
  run: RunStatusDTO | null;
  tokenAvailable: boolean;
  launching: boolean;
  onLaunch: (hours: number) => Promise<ApiResult>;
  onCancel: () => Promise<ApiResult>;
  now: number;
}

const HOURS_OPTIONS = [2, 3, 4, 5.5];

export function RunPanel({
  run,
  tokenAvailable,
  launching,
  onLaunch,
  onCancel,
  now,
}: RunPanelProps) {
  const [hours, setHours] = useState(5.5);
  const [cancelling, setCancelling] = useState(false);

  const inProgress =
    run !== null &&
    (run.status === 'in_progress' || run.status === 'queued' || run.status === 'waiting');
  const failed = run !== null && run.conclusion === 'failure';
  const done = run !== null && run.conclusion === 'success';
  const cancelled = run !== null && run.conclusion === 'cancelled';

  const statusChip = inProgress
    ? { text: 'الجلسة تعمل الآن', cls: 'border-emerald-400/40 bg-emerald-400/10 text-emerald-300', dot: 'bg-emerald-400 animate-pulse' }
    : failed
      ? { text: 'فشلت آخر جلسة', cls: 'border-red-400/40 bg-red-500/10 text-red-300', dot: 'bg-red-400' }
      : done
        ? { text: 'اكتملت الجلسة بنجاح', cls: 'border-sky-400/40 bg-sky-400/10 text-sky-300', dot: 'bg-sky-400' }
        : cancelled
          ? { text: 'أُوقفت الجلسة', cls: 'border-zinc-500/40 bg-zinc-500/10 text-zinc-400', dot: 'bg-zinc-400' }
          : { text: 'لا جلسة بعد', cls: 'border-white/10 bg-white/[0.04] text-zinc-400', dot: 'bg-zinc-600' };

  const elapsed = run ? Math.max(0, Math.round((now - new Date(run.createdAt).getTime()) / 1000)) : 0;
  const elapsedLabel =
    elapsed >= 3600
      ? `${arNum(Math.floor(elapsed / 3600))} س ${arNum(Math.floor((elapsed % 3600) / 60))} د`
      : elapsed >= 60
        ? `${arNum(Math.floor(elapsed / 60))} د`
        : `${arNum(elapsed)} ث`;

  return (
    <Card className="gap-3 rounded-2xl border-white/10 bg-white/[0.03] py-4 shadow-xl shadow-black/30 backdrop-blur-sm">
      <CardHeader className="px-4">
        <CardTitle className="flex items-center gap-2 text-sm font-bold text-zinc-100">
          <span className="grid size-8 place-items-center rounded-lg border border-white/10 bg-white/[0.04] text-zinc-300">
            <Rocket className="size-4" aria-hidden="true" />
          </span>
          إدارة التشغيل (GitHub Actions)
        </CardTitle>
      </CardHeader>
      <CardContent className="space-y-3 px-4">
        {/* حالة آخر تشغيل */}
        <div className="flex flex-wrap items-center gap-2 rounded-xl border border-white/10 bg-black/30 px-3 py-2.5">
          <span
            role="status"
            className={cn(
              'inline-flex items-center gap-1.5 rounded-full border px-2.5 py-1 text-[11px] font-semibold',
              statusChip.cls,
            )}
          >
            <span className={cn('size-1.5 rounded-full', statusChip.dot)} aria-hidden="true" />
            {statusChip.text}
          </span>
          {run && (
            <>
              <span dir="ltr" className="font-mono text-[11px] text-zinc-500">
                #{arNum(run.number)}
              </span>
              <span className="text-[11px] text-zinc-500">
                {inProgress ? `مدة: ${elapsedLabel}` : relativeAr(run.updatedAt, now)}
              </span>
              <a
                href={run.htmlUrl}
                target="_blank"
                rel="noreferrer"
                className="ms-auto inline-flex items-center gap-1 text-[11px] text-amber-300 underline-offset-2 hover:underline"
              >
                فتح في GitHub
                <ExternalLink className="size-3" aria-hidden="true" />
              </a>
            </>
          )}
        </div>

        {/* التحكم */}
        <div className="flex flex-wrap items-center gap-2">
          <div className="flex items-center gap-1 rounded-lg border border-white/10 bg-black/40 p-1">
            {HOURS_OPTIONS.map((h) => (
              <button
                key={h}
                type="button"
                onClick={() => setHours(h)}
                aria-pressed={hours === h}
                className={cn(
                  'rounded-md px-2.5 py-1 text-[11px] font-semibold tabular-nums transition',
                  hours === h
                    ? 'bg-amber-400/90 text-black'
                    : 'text-zinc-400 hover:bg-white/10 hover:text-zinc-100',
                )}
              >
                {h === 5.5 ? '٥٫٥ س' : `${arNum(h)} س`}
              </button>
            ))}
          </div>

          {inProgress ? (
            <Button
              type="button"
              variant="outline"
              disabled={cancelling || !tokenAvailable}
              onClick={async () => {
                setCancelling(true);
                await onCancel();
                setCancelling(false);
              }}
              className="h-9 gap-1.5 border-red-400/30 bg-red-400/[0.07] text-[12px] text-red-200 hover:bg-red-400/15 hover:text-red-100"
            >
              {cancelling ? (
                <Loader2 className="size-4 animate-spin" aria-hidden="true" />
              ) : (
                <CircleStop className="size-4" aria-hidden="true" />
              )}
              إيقاف الجلسة
            </Button>
          ) : (
            <Button
              type="button"
              disabled={launching || !tokenAvailable}
              onClick={() => void onLaunch(hours)}
              className="h-9 gap-1.5 bg-amber-400 text-[12px] font-bold text-black hover:bg-amber-300 disabled:opacity-50"
            >
              {launching ? (
                <Loader2 className="size-4 animate-spin" aria-hidden="true" />
              ) : (
                <Play className="size-4" aria-hidden="true" />
              )}
              {launching ? 'جارٍ الإطلاق…' : 'ابدأ الجلسة الآن'}
            </Button>
          )}
        </div>

        {!tokenAvailable && (
          <p className="text-[11px] leading-relaxed text-red-300/80">
            توكن GitHub غير متوفر للخادم — لا يمكن إدارة التشغيل من الموقع حالياً.
          </p>
        )}
        <p className="text-[11px] leading-relaxed text-zinc-500">
          {inProgress
            ? 'المحاكي يعمل على GitHub Actions والدرايفر متصل بهذا الموقع — راقب البث والسجل بالأسفل.'
            : 'الضغط على «ابدأ» يشغّل محاكي أندرويد على GitHub Actions ويوصله بهذا الموقع تلقائياً (بلا نسخ روابط).'}
        </p>
      </CardContent>
    </Card>
  );
}
