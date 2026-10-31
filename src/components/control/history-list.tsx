"use client";

import { useState } from "react";
import { History, ImageOff } from "lucide-react";
import type { TaskDTO } from "@/lib/control-types";
import { Badge } from "@/components/ui/badge";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { cn } from "@/lib/utils";
import {
  STATUS_BADGE_CLASSES,
  STATUS_LABELS_AR,
  actionSummary,
  relativeAr,
  waitMsLabel,
} from "./utils";

interface HistoryListProps {
  recent: TaskDTO[];
}

export function HistoryList({ recent }: HistoryListProps) {
  const [openTask, setOpenTask] = useState<TaskDTO | null>(null);

  return (
    <Card className="gap-4 rounded-2xl border-white/10 bg-white/[0.03] py-5 shadow-xl shadow-black/30 backdrop-blur-sm">
      <CardHeader className="px-4 sm:px-5">
        <CardTitle className="flex items-center gap-2 text-sm font-bold text-zinc-100">
          <span className="grid size-8 place-items-center rounded-lg border border-white/10 bg-white/[0.04] text-zinc-300">
            <History className="size-4" aria-hidden="true" />
          </span>
          سجل الطلبات
          <span className="ms-auto font-mono text-[11px] font-normal text-zinc-500" dir="ltr">
            {recent.length}/15
          </span>
        </CardTitle>
      </CardHeader>

      <CardContent className="px-3 sm:px-4">
        {recent.length === 0 ? (
          <p className="px-2 py-6 text-center text-xs text-zinc-600">
            لا يوجد سجل بعد — سيظهر هنا كل طلب يرسله الخط.
          </p>
        ) : (
          <div className="max-h-96 space-y-1 overflow-y-auto pr-1 [&::-webkit-scrollbar]:w-1.5 [&::-webkit-scrollbar-thumb]:rounded-full [&::-webkit-scrollbar-thumb]:bg-amber-500/40">
            {recent.map((t) => {
              const clickable = !!t.imageUrl;
              const inner = (
                <>
                  <div className="flex items-center gap-2">
                    <span className="shrink-0 font-mono text-[11px] text-zinc-500" dir="ltr">
                      #{t.seq}
                    </span>
                    <span className="min-w-0 flex-1 truncate text-xs font-medium text-zinc-200">
                      {t.title}
                    </span>
                    <Badge
                      className={cn(
                        "shrink-0 px-1.5 text-[10px]",
                        STATUS_BADGE_CLASSES[t.status],
                      )}
                    >
                      {STATUS_LABELS_AR[t.status]}
                    </Badge>
                  </div>
                  <div className="mt-1 flex items-center gap-2 text-[10px] text-zinc-500">
                    {t.action && (
                      <span className="font-mono" dir="ltr">
                        {actionSummary(t)}
                      </span>
                    )}
                    {t.waitMs !== null && t.waitMs !== undefined && (
                      <span>• {waitMsLabel(t.waitMs)}</span>
                    )}
                    <span className="ms-auto whitespace-nowrap">
                      {relativeAr(t.createdAt, Date.now())}
                    </span>
                  </div>
                </>
              );

              return clickable ? (
                <button
                  key={t.id}
                  type="button"
                  onClick={() => setOpenTask(t)}
                  className="w-full rounded-lg border border-transparent px-2.5 py-2 text-start transition hover:border-white/10 hover:bg-white/[0.04] focus-visible:outline-2 focus-visible:outline-amber-400"
                  aria-label={`تفاصيل الطلب رقم ${t.seq}`}
                >
                  {inner}
                </button>
              ) : (
                <div
                  key={t.id}
                  className="w-full cursor-default rounded-lg border border-transparent px-2.5 py-2"
                >
                  {inner}
                </div>
              );
            })}
          </div>
        )}
      </CardContent>

      {/* تفاصيل الطلب */}
      <Dialog open={openTask !== null} onOpenChange={(o) => !o && setOpenTask(null)}>
        <DialogContent className="max-h-[88vh] gap-4 overflow-y-auto border-white/10 bg-[#0b0e16] text-zinc-100 sm:max-w-2xl [&::-webkit-scrollbar]:w-1.5 [&::-webkit-scrollbar-thumb]:rounded-full [&::-webkit-scrollbar-thumb]:bg-white/15">
          {openTask && (
            <>
              <DialogHeader className="text-start">
                <DialogTitle className="flex flex-wrap items-center gap-2 text-start text-base">
                  <span className="font-mono text-xs text-zinc-500" dir="ltr">
                    #{openTask.seq}
                  </span>
                  <span className="font-bold">{openTask.title}</span>
                  <Badge
                    className={cn(
                      "px-1.5 text-[10px]",
                      STATUS_BADGE_CLASSES[openTask.status],
                    )}
                  >
                    {STATUS_LABELS_AR[openTask.status]}
                  </Badge>
                </DialogTitle>
                <DialogDescription className="text-start">
                  <span dir="ltr" className="font-mono text-[11px]">
                    {openTask.stepKey}
                  </span>
                  {" • "}
                  {relativeAr(openTask.createdAt, Date.now())}
                  {openTask.action && (
                    <>
                      {" • "}
                      <span dir="ltr" className="font-mono text-[11px]">
                        {actionSummary(openTask)}
                      </span>
                    </>
                  )}
                </DialogDescription>
              </DialogHeader>

              {openTask.instruction && (
                <p className="rounded-lg border border-amber-400/20 bg-amber-400/[0.06] p-3 text-xs leading-relaxed text-amber-100/80">
                  {openTask.instruction}
                </p>
              )}

              {openTask.description && (
                <div>
                  <p className="mb-1 text-[11px] font-semibold text-zinc-500">
                    وصف الزر
                  </p>
                  <p className="rounded-lg border border-white/10 bg-white/[0.03] p-3 text-sm leading-relaxed text-zinc-200">
                    {openTask.description}
                  </p>
                </div>
              )}

              {openTask.error && (
                <p className="rounded-lg border border-red-400/30 bg-red-500/10 p-3 text-sm text-red-300">
                  {openTask.error}
                </p>
              )}

              {openTask.imageUrl ? (
                <div>
                  <p className="mb-1.5 text-[11px] font-semibold text-zinc-500">
                    اللقطة قبل التنفيذ
                  </p>
                  <img
                    src={openTask.imageUrl}
                    alt={`لقطة الطلب رقم ${openTask.seq}`}
                    className="w-full rounded-xl border border-white/10"
                  />
                </div>
              ) : (
                <div className="flex items-center justify-center gap-2 rounded-xl border border-dashed border-white/15 bg-black/30 p-6 text-xs text-zinc-600">
                  <ImageOff className="size-4" aria-hidden="true" />
                  لا توجد لقطة
                </div>
              )}

              {openTask.afterImageUrl && (
                <div>
                  <p className="mb-1.5 text-[11px] font-semibold text-emerald-400/80">
                    بعد التنفيذ
                  </p>
                  <img
                    src={openTask.afterImageUrl}
                    alt={`لقطة ما بعد التنفيذ للطلب رقم ${openTask.seq}`}
                    className="w-full rounded-xl border border-emerald-400/20"
                  />
                </div>
              )}
            </>
          )}
        </DialogContent>
      </Dialog>
    </Card>
  );
}
