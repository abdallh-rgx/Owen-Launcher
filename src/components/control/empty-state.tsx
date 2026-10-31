"use client";

import { Bot, Radar } from "lucide-react";
import { Card, CardContent } from "@/components/ui/card";

interface EmptyStateProps {
  runnerLive: boolean;
  autopilotEnabled: boolean | null;
}

export function EmptyState({ runnerLive, autopilotEnabled }: EmptyStateProps) {
  return (
    <Card className="items-center gap-6 rounded-2xl border-white/10 bg-white/[0.03] px-4 py-12 text-center shadow-2xl shadow-black/30 backdrop-blur-sm sm:py-16">
      <CardContent className="flex flex-col items-center gap-5">
        {/* رادار نابض */}
        <div className="relative grid size-28 place-items-center" aria-hidden="true">
          <span className="absolute inset-0 animate-ping rounded-full border border-amber-400/25" />
          <span className="absolute inset-3 animate-ping rounded-full border border-amber-400/20 [animation-delay:0.45s]" />
          <span className="absolute inset-6 animate-ping rounded-full border border-amber-400/15 [animation-delay:0.9s]" />
          <Radar className="size-10 text-amber-300" />
        </div>

        <div className="space-y-2">
          <h2 className="text-lg font-bold text-zinc-100 sm:text-xl">
            لا يوجد طلب معلّق حالياً
          </h2>
          <p className="mx-auto max-w-md text-sm leading-relaxed text-zinc-400">
            {autopilotEnabled
              ? runnerLive
                ? "الطيار الآلي يقود الجلسة بنفسه — يراقب الشاشة، يضغط الأزرار، وينتظر التحميلات. ستظهر قراراته في بطاقة الطيار الآلي بالأعلى ولا حاجة لأي تدخل منك."
                : "شغّل الجلسة من بطاقة التشغيل بالجانب — وبمجرد اتصال الدرايفر سيتولى الطيار الآلي كل شيء تلقائياً."
              : runnerLive
                ? "الخط يعمل… سيظهر طلب جديد تلقائياً، والطيار الآلي متوقف حالياً — أعد تشغيله من بطاقة الطيار الآلي أو أجب بنفسك."
                : "شغّل الـ Action حتى يبدأ الخط — استخدم بطاقة التشغيل في الجانب."}
          </p>
        </div>

        {autopilotEnabled && (
          <div className="flex items-center gap-2 rounded-full border border-amber-400/25 bg-amber-400/[0.07] px-4 py-1.5 text-[11px] text-amber-200">
            <Bot className="size-4 animate-pulse" aria-hidden="true" />
            القيادة الآلية الكاملة نشطة — glm-4.6v يرى الشاشة ويقرر
          </div>
        )}

        <div className="w-full max-w-sm space-y-2" aria-hidden="true">
          <div className="h-1.5 animate-pulse rounded-full bg-gradient-to-l from-amber-400/40 via-amber-400/10 to-transparent" />
          <div className="h-1.5 animate-pulse rounded-full bg-gradient-to-l from-amber-400/25 via-amber-400/5 to-transparent [animation-delay:0.3s]" />
          <div className="h-1.5 animate-pulse rounded-full bg-gradient-to-l from-amber-400/15 via-amber-400/5 to-transparent [animation-delay:0.6s]" />
        </div>
      </CardContent>
    </Card>
  );
}
