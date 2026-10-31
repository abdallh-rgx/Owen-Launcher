"use client";

import { useState, useSyncExternalStore } from "react";
import { Check, Copy, Github } from "lucide-react";
import { toast } from "sonner";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Separator } from "@/components/ui/separator";
import { cn } from "@/lib/utils";

interface ConnectionCardProps {
  token: string;
}

/* قراءة origin من المتصفح بعد التحميل — بلا setState داخل effect وبلا تعارض مع SSR */
const noopSubscribe = () => () => {};
const getOriginClient = () => window.location.origin;
const getOriginServer = () => "";

export function ConnectionCard({ token }: ConnectionCardProps) {
  const origin = useSyncExternalStore(
    noopSubscribe,
    getOriginClient,
    getOriginServer,
  );
  const [copied, setCopied] = useState<"url" | "token" | null>(null);

  const copy = async (what: "url" | "token") => {
    const text = what === "url" ? origin : token;
    if (!text) return;
    try {
      await navigator.clipboard.writeText(text);
      setCopied(what);
      toast.success(what === "url" ? "تم نسخ control_url" : "تم نسخ control_token");
      window.setTimeout(() => setCopied(null), 1600);
    } catch {
      toast.error("تعذّر النسخ — انسخ القيمة يدوياً");
    }
  };

  const field = (what: "url" | "token", name: string, value: string) => (
    <div className="space-y-1.5">
      <div className="flex items-center justify-between">
        <span className="font-mono text-[11px] font-semibold text-zinc-400" dir="ltr">
          {name}
        </span>
        <Button
          type="button"
          variant="ghost"
          size="sm"
          onClick={() => void copy(what)}
          aria-label={`نسخ ${name}`}
          className="h-7 gap-1 px-2 text-[11px] text-zinc-400 hover:bg-white/10 hover:text-zinc-100"
        >
          {copied === what ? (
            <Check className="size-3.5 text-emerald-400" aria-hidden="true" />
          ) : (
            <Copy className="size-3.5" aria-hidden="true" />
          )}
          {copied === what ? "تم النسخ" : "نسخ"}
        </Button>
      </div>
      <div
        dir="ltr"
        className="flex items-center rounded-lg border border-white/10 bg-black/50 px-3 py-2"
      >
        <code
          className={cn(
            "min-w-0 flex-1 overflow-hidden text-ellipsis whitespace-nowrap font-mono text-xs",
            what === "url" ? "text-emerald-300" : "text-amber-300",
          )}
        >
          {value || "…"}
        </code>
      </div>
    </div>
  );

  return (
    <Card className="gap-4 rounded-2xl border-white/10 bg-white/[0.03] py-5 shadow-xl shadow-black/30 backdrop-blur-sm">
      <CardHeader className="px-4 sm:px-5">
        <CardTitle className="flex items-center gap-2 text-sm font-bold text-zinc-100">
          <span className="grid size-8 place-items-center rounded-lg border border-white/10 bg-white/[0.04] text-zinc-300">
            <Github className="size-4" aria-hidden="true" />
          </span>
          تشغيل الجلسة (GitHub Action)
        </CardTitle>
      </CardHeader>
      <CardContent className="space-y-3.5 px-4 sm:px-5">
        {field("url", "control_url", origin)}
        {field("token", "control_token", token)}

        <Separator className="bg-white/10" />

        <ol className="space-y-2 text-[11px] leading-relaxed text-zinc-400">
          <li className="flex gap-2">
            <span className="shrink-0 font-bold text-amber-300">١)</span>
            <span>افتح مستودع Owen-Launcher في GitHub ← تبويب Actions</span>
          </li>
          <li className="flex gap-2">
            <span className="shrink-0 font-bold text-amber-300">٢)</span>
            <span>
              اختر Workflow باسم «Owen Emulator Control Room» ← زر Run workflow
            </span>
          </li>
          <li className="flex gap-2">
            <span className="shrink-0 font-bold text-amber-300">٣)</span>
            <span>الصق control_url و control_token في الخانات</span>
          </li>
          <li className="flex gap-2">
            <span className="shrink-0 font-bold text-amber-300">٤)</span>
            <span>اضغط Run وانتظر — ستظهر الطلبات هنا تلقائياً.</span>
          </li>
        </ol>
      </CardContent>
    </Card>
  );
}
