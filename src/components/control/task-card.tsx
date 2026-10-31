"use client";

import { useEffect, useState } from "react";
import type { ReactNode } from "react";
import { toast } from "sonner";
import {
  AppWindow,
  Crosshair,
  Focus,
  ImageOff,
  Timer,
} from "lucide-react";
import type { ControlAction, TaskDTO } from "@/lib/control-types";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader } from "@/components/ui/card";
import { AnswerForm } from "./answer-form";
import { ScreenshotAnnotator } from "./screenshot-annotator";
import { arNum, relativeAr, type AnswerPayload, type ApiResult, type Point } from "./utils";

interface TaskCardProps {
  task: TaskDTO;
  queueCount: number;
  onAnswer: (payload: AnswerPayload) => Promise<ApiResult>;
  onRetake: () => Promise<ApiResult>;
  /** نقطة وصلت من النقر على البث المباشر (تُعبّأ كنقطة أولى) */
  livePick?: { point: Point; at: number } | null;
}

const NEEDS_COORDS: ReadonlySet<ControlAction> = new Set(["tap", "swipe"]);

export function TaskCard({ task, queueCount, onAnswer, onRetake, livePick }: TaskCardProps) {
  const [action, setAction] = useState<ControlAction>("tap");
  const [p1, setP1] = useState<Point | null>(null);
  const [p2, setP2] = useState<Point | null>(null);
  const [activePoint, setActivePoint] = useState<1 | 2>(1);
  const [description, setDescription] = useState("");
  const [waitMs, setWaitMs] = useState(2000);
  const [submitting, setSubmitting] = useState(false);
  const [retaking, setRetaking] = useState(false);

  // Reset the form whenever a different task becomes current.
  useEffect(() => {
    setAction("tap");
    setP1(null);
    setP2(null);
    setActivePoint(1);
    setDescription("");
    setWaitMs(2000);
  }, [task.id]);

  // نقطة من البث المباشر → عبّئها كنقطة أولى (أو ثانية في وضع السحب)
  useEffect(() => {
    if (!livePick) return;
    if (action === "swipe") {
      if (!p1) setP1(livePick.point);
      else setP2(livePick.point);
      setActivePoint(p1 ? 2 : 1);
      toast.success(p1 ? "تم تحديد نقطة النهاية من البث" : "تم تحديد نقطة البداية من البث");
    } else {
      setP1(livePick.point);
      toast.success("تم تحديد النقطة من البث — أكمل الوصف واضغط إرسال");
    }
  }, [livePick?.at]);

  const setPoint = (which: 1 | 2, pt: Point | null) => {
    if (which === 1) setP1(pt);
    else setP2(pt);
  };

  const submit = async (act: ControlAction) => {
    if (submitting) return;
    const payload: AnswerPayload = { action: act };

    if (act !== "skip" && act !== "done") {
      payload.waitMs = waitMs;
      if (description.trim()) payload.description = description.trim();
    }

    if (act === "tap") {
      if (!p1) {
        toast.error("انقر على الصورة أولاً لتحديد مكان الزر");
        return;
      }
      payload.x = p1.x;
      payload.y = p1.y;
    } else if (act === "swipe") {
      if (!p1 || !p2) {
        toast.error("حدّد نقطتَي البداية والنهاية للسحب على الصورة");
        return;
      }
      payload.x = p1.x;
      payload.y = p1.y;
      payload.x2 = p2.x;
      payload.y2 = p2.y;
    }

    setSubmitting(true);
    const res = await onAnswer(payload);
    setSubmitting(false);
    if (res.ok) toast.success("تم الإرسال — الخط ينفّذ الآن");
    else toast.error(res.error ?? "تعذّر الإرسال");
  };

  const retake = async () => {
    if (retaking || task.wantsRetake) return;
    setRetaking(true);
    const res = await onRetake();
    setRetaking(false);
    if (!res.ok) toast.error(res.error ?? "تعذّر طلب لقطة جديدة");
  };

  const meta = task.meta;
  const metaChips: Array<{ key: string; label: string; value: string; icon: ReactNode }> = [];
  if (meta?.app)
    metaChips.push({
      key: "app",
      label: "التطبيق الحالي",
      value: String(meta.app),
      icon: <AppWindow className="size-3" aria-hidden="true" />,
    });
  if (meta?.focus)
    metaChips.push({
      key: "focus",
      label: "التركيز",
      value: String(meta.focus),
      icon: <Focus className="size-3" aria-hidden="true" />,
    });
  if (meta?.uptime)
    metaChips.push({
      key: "uptime",
      label: "مدة التشغيل",
      value: String(meta.uptime),
      icon: <Timer className="size-3" aria-hidden="true" />,
    });

  return (
    <Card className="gap-4 rounded-2xl border-white/10 bg-white/[0.03] py-5 shadow-2xl shadow-black/40 backdrop-blur-sm sm:py-6">
      <CardHeader className="gap-3 px-4 sm:px-6">
        <div className="flex flex-wrap items-center gap-2">
          <Badge
            dir="ltr"
            className="border-amber-400/40 bg-amber-400/10 font-mono text-amber-300"
          >
            #{task.seq}
          </Badge>
          <h2 className="text-lg font-bold leading-tight sm:text-xl">
            {task.title}
          </h2>
          <span
            dir="ltr"
            className="rounded-md border border-white/10 bg-black/40 px-1.5 py-0.5 font-mono text-[10px] text-zinc-500"
          >
            {task.stepKey}
          </span>
          {queueCount > 0 && (
            <Badge
              variant="outline"
              className="border-white/10 bg-transparent text-zinc-400"
            >
              +{arNum(queueCount)} في الطابور
            </Badge>
          )}
          <span className="ms-auto text-[11px] text-zinc-500">
            {relativeAr(task.createdAt, Date.now())}
          </span>
        </div>

        {metaChips.length > 0 && (
          <div className="flex flex-wrap gap-1.5">
            {metaChips.map((chip) => (
              <span
                key={chip.key}
                className="inline-flex items-center gap-1 rounded-full border border-white/10 bg-white/[0.04] px-2.5 py-0.5 text-[11px] text-zinc-300"
              >
                <span className="text-zinc-500" aria-hidden="true">
                  {chip.icon}
                </span>
                {chip.label}: {chip.value}
              </span>
            ))}
          </div>
        )}
      </CardHeader>

      <CardContent className="space-y-4 px-4 sm:px-6">
        {/* التعليمات */}
        <div className="rounded-xl border border-amber-400/25 bg-amber-400/[0.07] p-3.5 sm:p-4">
          <div className="mb-1 flex items-center gap-1.5 text-[11px] font-semibold text-amber-300/90">
            <Crosshair className="size-3.5" aria-hidden="true" />
            التعليمات
          </div>
          <p className="text-sm leading-relaxed text-amber-100/90 sm:text-[15px]">
            {task.instruction}
          </p>
          {meta?.note && (
            <p className="mt-2 border-t border-amber-400/15 pt-2 text-xs text-amber-200/60">
              {String(meta.note)}
            </p>
          )}
        </div>

        {/* محدّد اللقطة */}
        {task.imageUrl ? (
          <ScreenshotAnnotator
            imageUrl={task.imageUrl}
            imageW={task.imageW}
            imageH={task.imageH}
            interactive={!submitting && NEEDS_COORDS.has(action)}
            swipeMode={action === "swipe"}
            p1={p1}
            p2={p2}
            activePoint={activePoint}
            onActivePointChange={setActivePoint}
            onSetPoint={setPoint}
            wantsRetake={task.wantsRetake}
            retaking={retaking}
            onRetake={retake}
          />
        ) : (
          <div className="flex flex-col items-center gap-3 rounded-xl border border-dashed border-white/15 bg-black/30 p-8 text-center">
            <ImageOff className="size-8 text-zinc-600" aria-hidden="true" />
            <p className="text-sm text-zinc-400">
              لا توجد لقطة شاشة لهذا الطلب — اطلب لقطة جديدة.
            </p>
            <Button
              type="button"
              variant="outline"
              size="sm"
              onClick={retake}
              disabled={task.wantsRetake || retaking}
              className="border-amber-400/30 bg-amber-400/[0.06] text-amber-300 hover:bg-amber-400/15 hover:text-amber-200"
            >
              لقطة جديدة
            </Button>
          </div>
        )}

        {/* نموذج الإجابة */}
        <AnswerForm
          action={action}
          onActionChange={(a) => {
            setAction(a);
            if (a === "swipe") setActivePoint(1);
          }}
          p1={p1}
          p2={p2}
          onSetPoint={setPoint}
          description={description}
          onDescriptionChange={setDescription}
          waitMs={waitMs}
          onWaitMsChange={setWaitMs}
          submitting={submitting}
          onSubmit={() => void submit(action)}
          onSkip={() => void submit("skip")}
          onDone={() => void submit("done")}
        />
      </CardContent>
    </Card>
  );
}
