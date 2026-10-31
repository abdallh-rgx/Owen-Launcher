"use client";

import { useState } from "react";
import {
  Loader2,
  MousePointerClick,
  Power,
  SkipForward,
} from "lucide-react";
import {
  ACTION_LABELS_AR,
  CONTROL_ACTIONS,
  type ControlAction,
} from "@/lib/control-types";
import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
} from "@/components/ui/alert-dialog";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Textarea } from "@/components/ui/textarea";
import { cn } from "@/lib/utils";
import { WAIT_PRESETS, clamp, type Point } from "./utils";

interface AnswerFormProps {
  action: ControlAction;
  onActionChange: (a: ControlAction) => void;
  p1: Point | null;
  p2: Point | null;
  onSetPoint: (which: 1 | 2, p: Point | null) => void;
  description: string;
  onDescriptionChange: (v: string) => void;
  waitMs: number;
  onWaitMsChange: (v: number) => void;
  submitting: boolean;
  onSubmit: () => void;
  onSkip: () => void;
  onDone: () => void;
}

const NEEDS_COORDS: ReadonlySet<ControlAction> = new Set(["tap", "swipe"]);

export function AnswerForm({
  action,
  onActionChange,
  p1,
  p2,
  onSetPoint,
  description,
  onDescriptionChange,
  waitMs,
  onWaitMsChange,
  submitting,
  onSubmit,
  onSkip,
  onDone,
}: AnswerFormProps) {
  const [skipOpen, setSkipOpen] = useState(false);
  const [doneOpen, setDoneOpen] = useState(false);

  const coordsEnabled = NEEDS_COORDS.has(action);
  const dimmed = !coordsEnabled;

  const handleCoord = (
    which: 1 | 2,
    axis: "x" | "y",
    raw: string,
  ) => {
    if (raw === "") {
      onSetPoint(which, null);
      return;
    }
    const n = Number(raw);
    if (!Number.isFinite(n) || n < 0) return;
    const base = which === 1 ? p1 : p2;
    const other = axis === "x" ? (base?.y ?? 0) : (base?.x ?? 0);
    const v = Math.round(clamp(n, 0, 12000));
    onSetPoint(which, axis === "x" ? { x: v, y: other } : { x: other, y: v });
  };

  const coordInput = (
    which: 1 | 2,
    axis: "x" | "y",
    label: string,
    id: string,
  ) => {
    const base = which === 1 ? p1 : p2;
    const value = base ? String(axis === "x" ? base.x : base.y) : "";
    return (
      <div className="space-y-1">
        <Label
          htmlFor={id}
          className="font-mono text-[11px] text-zinc-400"
          dir="ltr"
        >
          {label}
        </Label>
        <Input
          id={id}
          type="number"
          min={0}
          step={1}
          dir="ltr"
          inputMode="numeric"
          disabled={!coordsEnabled}
          value={value}
          onChange={(e) => handleCoord(which, axis, e.target.value)}
          className="h-9 border-white/10 bg-black/40 text-center font-mono text-sm tabular-nums text-amber-200 focus-visible:ring-amber-400/40 disabled:opacity-40"
        />
      </div>
    );
  };

  return (
    <form
      className="grid gap-4 md:grid-cols-2"
      onSubmit={(e) => {
        e.preventDefault();
        onSubmit();
      }}
    >
      {/* العمود الأول: نوع الإجراء + الإحداثيات */}
      <div
        className={cn(
          "space-y-3 rounded-xl border border-white/10 bg-white/[0.02] p-3.5 transition-opacity",
          dimmed && "opacity-50",
        )}
      >
        <div className="space-y-1.5">
          <Label
            htmlFor="ctl-action"
            className="text-xs font-semibold text-zinc-300"
          >
            نوع الإجراء
          </Label>
          <Select
            value={action}
            onValueChange={(v) => onActionChange(v as ControlAction)}
          >
            <SelectTrigger
              id="ctl-action"
              className="w-full border-white/10 bg-black/40 text-zinc-100 focus-visible:ring-amber-400/40"
            >
              <SelectValue placeholder="اختر الإجراء" />
            </SelectTrigger>
            <SelectContent className="border-white/10 bg-[#0b0e16] text-zinc-100">
              {CONTROL_ACTIONS.map((a) => (
                <SelectItem
                  key={a}
                  value={a}
                  className="focus:bg-white/10 focus:text-amber-200"
                >
                  {ACTION_LABELS_AR[a]}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
        </div>

        <div className="space-y-1.5">
          <p className="text-xs font-semibold text-zinc-300">
            الإحداثيات{" "}
            <span className="font-normal text-zinc-500">(بكسل — انقر الصورة)</span>
          </p>
          {action === "swipe" ? (
            <div className="grid grid-cols-2 gap-2 sm:grid-cols-4">
              {coordInput(1, "x", "X", "ctl-x")}
              {coordInput(1, "y", "Y", "ctl-y")}
              {coordInput(2, "x", "X2", "ctl-x2")}
              {coordInput(2, "y", "Y2", "ctl-y2")}
            </div>
          ) : (
            <div className="grid grid-cols-2 gap-2">
              {coordInput(1, "x", "X", "ctl-x")}
              {coordInput(1, "y", "Y", "ctl-y")}
            </div>
          )}
        </div>
      </div>

      {/* العمود الثاني: الوصف + مدة الانتظار */}
      <div className="space-y-3 rounded-xl border border-white/10 bg-white/[0.02] p-3.5">
        <div className="space-y-1.5">
          <Label
            htmlFor="ctl-desc"
            className="text-xs font-semibold text-zinc-300"
          >
            وصف وظيفة الزر (ماذا يفعل هذا الزر؟)
          </Label>
          <Textarea
            id="ctl-desc"
            value={description}
            onChange={(e) => onDescriptionChange(e.target.value)}
            placeholder="زر PLAY في اللوبي — يفتح اختيار نمط اللعب"
            rows={2}
            className="min-h-[2.75rem] resize-y border-white/10 bg-black/40 text-sm text-zinc-100 placeholder:text-zinc-600 focus-visible:ring-amber-400/40"
          />
        </div>

        <div className="space-y-1.5">
          <Label
            htmlFor="ctl-wait"
            className="text-xs font-semibold text-zinc-300"
          >
            مدة الانتظار{" "}
            <span className="font-mono font-normal text-zinc-500">(ms)</span>
          </Label>
          <Input
            id="ctl-wait"
            type="number"
            min={0}
            step={100}
            dir="ltr"
            inputMode="numeric"
            value={waitMs}
            onChange={(e) => {
              const n = Number(e.target.value);
              if (Number.isFinite(n) && n >= 0) {
                onWaitMsChange(Math.round(clamp(n, 0, 600000)));
              }
            }}
            className="h-9 border-white/10 bg-black/40 text-center font-mono text-sm tabular-nums text-amber-200 focus-visible:ring-amber-400/40"
          />
          <div className="flex flex-wrap gap-1.5 pt-0.5">
            {WAIT_PRESETS.map((preset) => (
              <button
                key={preset.ms}
                type="button"
                onClick={() => onWaitMsChange(preset.ms)}
                aria-pressed={waitMs === preset.ms}
                className={cn(
                  "rounded-full border px-2.5 py-1 text-[11px] transition focus-visible:outline-2 focus-visible:outline-offset-1 focus-visible:outline-amber-400",
                  waitMs === preset.ms
                    ? "border-amber-400/50 bg-amber-400/15 font-semibold text-amber-200"
                    : "border-white/10 bg-white/[0.03] text-zinc-400 hover:border-amber-400/30 hover:text-amber-200/90",
                )}
              >
                {preset.label}
              </button>
            ))}
          </div>
          <p className="text-[11px] leading-relaxed text-zinc-500">
            كم ينتظر الخط بعد الضغط قبل اللقطة التالية؟
          </p>
        </div>
      </div>

      {/* صف الأزرار */}
      <div className="flex flex-col-reverse gap-2 md:col-span-2 sm:flex-row">
        <Button
          type="button"
          variant="outline"
          disabled={submitting}
          onClick={() => setSkipOpen(true)}
          className="h-11 border-white/15 bg-transparent text-sm text-zinc-300 hover:bg-white/5 hover:text-zinc-100"
        >
          <SkipForward className="size-4" aria-hidden="true" />
          تخطي (لقطة جديدة)
        </Button>
        <Button
          type="button"
          variant="outline"
          disabled={submitting}
          onClick={() => setDoneOpen(true)}
          className="h-11 border-red-400/40 bg-transparent text-sm text-red-300 hover:bg-red-500/10 hover:text-red-200"
        >
          <Power className="size-4" aria-hidden="true" />
          إنهاء الجلسة
        </Button>
        <Button
          type="submit"
          disabled={submitting}
          className="h-11 bg-amber-500 text-sm font-bold text-black shadow-lg shadow-amber-500/20 hover:bg-amber-400 sm:flex-1"
        >
          {submitting ? (
            <Loader2 className="size-4 animate-spin" aria-hidden="true" />
          ) : (
            <MousePointerClick className="size-4" aria-hidden="true" />
          )}
          إرسال الإحداثيات ▸
        </Button>
      </div>

      {/* تأكيد التخطي */}
      <AlertDialog open={skipOpen} onOpenChange={setSkipOpen}>
        <AlertDialogContent className="border-white/10 bg-[#0b0e16] text-zinc-100 sm:max-w-sm">
          <AlertDialogHeader>
            <AlertDialogTitle className="text-right">
              تخطي هذا الطلب؟
            </AlertDialogTitle>
            <AlertDialogDescription className="text-right text-zinc-400">
              سيُلغى هذا الطلب ويطلب الخط لقطة جديدة للمتابعة.
            </AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter className="flex-row gap-2 sm:justify-start">
            <AlertDialogAction
              onClick={() => {
                setSkipOpen(false);
                onSkip();
              }}
              className="border border-amber-400/40 bg-amber-500/90 text-black hover:bg-amber-400"
            >
              نعم، تخطٍ
            </AlertDialogAction>
            <AlertDialogCancel className="border-white/15 bg-transparent text-zinc-200 hover:bg-white/10">
              إلغاء
            </AlertDialogCancel>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>

      {/* تأكيد الإنهاء */}
      <AlertDialog open={doneOpen} onOpenChange={setDoneOpen}>
        <AlertDialogContent className="border-white/10 bg-[#0b0e16] text-zinc-100 sm:max-w-sm">
          <AlertDialogHeader>
            <AlertDialogTitle className="text-right">
              إنهاء الجلسة؟
            </AlertDialogTitle>
            <AlertDialogDescription className="text-right text-zinc-400">
              سيوقف الخط المحاكي وينهي الجلسة، متأكد؟
            </AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter className="flex-row gap-2 sm:justify-start">
            <AlertDialogAction
              onClick={() => {
                setDoneOpen(false);
                onDone();
              }}
              className="bg-red-600 text-white hover:bg-red-500"
            >
              نعم، أنهِ الجلسة
            </AlertDialogAction>
            <AlertDialogCancel className="border-white/15 bg-transparent text-zinc-200 hover:bg-white/10">
              إلغاء
            </AlertDialogCancel>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
    </form>
  );
}
