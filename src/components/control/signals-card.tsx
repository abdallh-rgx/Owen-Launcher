"use client";

/* بطاقة الإشارات — أوامر يدوية منك إلى الدرايفر أثناء الجلسة:
 *   ✅ انتهى التحميل الداخلي  → يكمل الدرايفر للمرحلة التالية
 *   🧩 ابدأ توليد SDK         → يفتح MobileDumper ويسحب الملفات
 *   🔄 إعادة فتح التطبيق      → force-stop + تشغيل OwenLauncher
 *   🚨 بلّغ عن كراش           → يجمع الدرايفر تقرير الكراش فوراً
 *   📝 ملاحظة حرة
 */

import { useState } from "react";
import {
  CheckCheck,
  Download,
  Hammer,
  MessageSquarePlus,
  RefreshCw,
  Siren,
} from "lucide-react";
import type { SignalDTO, SignalType } from "@/lib/control-types";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { cn } from "@/lib/utils";
import { relativeAr, type ApiResult } from "./utils";

interface SignalsCardProps {
  signals: SignalDTO[];
  onSignal: (type: SignalType, note?: string) => Promise<ApiResult>;
  now: number;
}

const BUTTONS: ReadonlyArray<{
  type: SignalType;
  label: string;
  hint: string;
  icon: typeof Download;
  classes: string;
}> = [
  {
    type: "download_done",
    label: "انتهى التحميل الداخلي",
    hint: "أخبر الدرايفر أن تحميل بيانات اللعبة اكتمل",
    icon: Download,
    classes:
      "border-emerald-400/30 bg-emerald-400/[0.07] text-emerald-200 hover:bg-emerald-400/15 hover:text-emerald-100",
  },
  {
    type: "start_sdk",
    label: "ابدأ توليد SDK",
    hint: "يشغّل MobileDumper ويسحب ملفات الـ dump",
    icon: Hammer,
    classes:
      "border-fuchsia-400/30 bg-fuchsia-400/[0.07] text-fuchsia-200 hover:bg-fuchsia-400/15 hover:text-fuchsia-100",
  },
  {
    type: "restart_app",
    label: "إعادة فتح التطبيق",
    hint: "force-stop + تشغيل OwenLauncher من جديد",
    icon: RefreshCw,
    classes:
      "border-sky-400/30 bg-sky-400/[0.07] text-sky-200 hover:bg-sky-400/15 hover:text-sky-100",
  },
  {
    type: "crash_report",
    label: "بلّغ عن كراش",
    hint: "يجمع الدرايفر تقرير الكراش من logcat فوراً",
    icon: Siren,
    classes:
      "border-red-400/30 bg-red-400/[0.07] text-red-200 hover:bg-red-400/15 hover:text-red-100",
  },
];

export function SignalsCard({ signals, onSignal, now }: SignalsCardProps) {
  const [busy, setBusy] = useState<SignalType | null>(null);
  const [note, setNote] = useState("");

  const send = async (type: SignalType, noteText?: string) => {
    if (busy) return;
    setBusy(type);
    const res = await onSignal(type, noteText);
    setBusy(null);
    if (res.ok) {
      if (type === "note") setNote("");
    }
    return res;
  };

  const pendingSame = (t: SignalType) =>
    signals.some((s) => s.type === t && s.status === "PENDING");

  return (
    <Card className="gap-3 rounded-2xl border-white/10 bg-white/[0.03] py-4 shadow-xl shadow-black/30 backdrop-blur-sm">
      <CardHeader className="px-4">
        <CardTitle className="flex items-center gap-2 text-sm font-bold text-zinc-100">
          <span className="grid size-8 place-items-center rounded-lg border border-white/10 bg-white/[0.04] text-zinc-300">
            <Siren className="size-4" aria-hidden="true" />
          </span>
          إشارات يدوية للدرايفر
        </CardTitle>
      </CardHeader>
      <CardContent className="space-y-3 px-4">
        <div className="grid grid-cols-2 gap-2">
          {BUTTONS.map((b) => {
            const Icon = b.icon;
            const disabled = busy !== null || pendingSame(b.type);
            return (
              <Button
                key={b.type}
                type="button"
                variant="outline"
                disabled={disabled}
                onClick={() => void send(b.type)}
                title={b.hint}
                className={cn(
                  "h-auto flex-col items-start gap-1 px-3 py-2.5 text-start",
                  b.classes,
                  disabled && "opacity-50",
                )}
              >
                <span className="flex w-full items-center gap-1.5 text-[13px] font-bold">
                  <Icon className="size-4 shrink-0" aria-hidden="true" />
                  {b.label}
                </span>
                <span className="text-[10px] font-normal opacity-70">
                  {pendingSame(b.type) ? "قيد الانتظار…" : b.hint}
                </span>
              </Button>
            );
          })}
        </div>

        <form
          className="flex gap-2"
          onSubmit={(e) => {
            e.preventDefault();
            if (note.trim()) void send("note", note.trim());
          }}
        >
          <Input
            value={note}
            onChange={(e) => setNote(e.target.value)}
            placeholder="ملاحظة حرة للدرايفر…"
            maxLength={400}
            className="h-9 border-white/10 bg-black/40 text-xs text-zinc-200 placeholder:text-zinc-600"
          />
          <Button
            type="submit"
            variant="outline"
            disabled={!note.trim() || busy !== null}
            className="h-9 shrink-0 gap-1 border-white/10 bg-white/[0.04] text-[11px] text-zinc-300 hover:bg-white/10 hover:text-white"
          >
            <MessageSquarePlus className="size-3.5" aria-hidden="true" />
            إرسال
          </Button>
        </form>

        {signals.length > 0 && (
          <ul className="max-h-40 space-y-1.5 overflow-y-auto pe-1 text-[11px]">
            {signals
              .slice()
              .reverse()
              .slice(0, 8)
              .map((s) => (
                <li
                  key={s.id}
                  className="flex items-start gap-2 rounded-lg border border-white/10 bg-black/30 px-2.5 py-1.5"
                >
                  <span
                    className={cn(
                      "mt-0.5 grid size-4 shrink-0 place-items-center rounded-full",
                      s.status === "ACK"
                        ? "bg-emerald-500/20 text-emerald-300"
                        : "bg-amber-500/20 text-amber-300",
                    )}
                  >
                    {s.status === "ACK" ? (
                      <CheckCheck className="size-2.5" aria-hidden="true" />
                    ) : (
                      <span className="size-1.5 animate-pulse rounded-full bg-amber-300" />
                    )}
                  </span>
                  <div className="min-w-0 flex-1">
                    <div className="flex flex-wrap items-baseline gap-x-2">
                      <span className="font-semibold text-zinc-200">
                        {signalLabel(s)}
                      </span>
                      <span className="text-zinc-600">
                        {relativeAr(s.createdAt, now)}
                      </span>
                    </div>
                    {s.note && (
                      <p className="truncate text-zinc-500">{s.note}</p>
                    )}
                    {s.status === "ACK" && s.result && (
                      <p dir="auto" className="mt-0.5 text-emerald-300/80">
                        {s.result}
                      </p>
                    )}
                  </div>
                </li>
              ))}
          </ul>
        )}
      </CardContent>
    </Card>
  );
}

function signalLabel(s: SignalDTO): string {
  if (s.type === "note") return "ملاحظة";
  if (s.type === "download_done") return "انتهى التحميل الداخلي";
  if (s.type === "start_sdk") return "ابدأ توليد SDK";
  if (s.type === "restart_app") return "إعادة فتح التطبيق";
  if (s.type === "crash_report") return "بلّغ عن كراش";
  return s.type;
}
