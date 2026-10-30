"use client";

/* OgsEvidence — the OGS 12.41 on-Android E2E view.
 *
 * Renders what the `ogs-1241-e2e` workflow actually proved INSIDE the game,
 * in an Android emulator, on GitHub Actions:
 *   Phase 1  MobileDumper-7 dumps the real 12.41 SDK from the running game
 *   Phase 2  libOwenOGS.so (the OGS port) + the shim-injected APK get built
 *   Phase 3  the game is driven to the LOBBY and into a live MATCH — with
 *            the real emulator screenshots committed back by CI.
 *
 * No mockups: every image comes from /api/ogs-e2e, which reads
 * docs/e2e/12.41/ straight out of the repo. Missing evidence renders an
 * honest "pending" state, never a placeholder.
 */

import { useCallback, useEffect, useState } from "react";
import { motion } from "framer-motion";
import { Gamepad2, RefreshCw, ShieldCheck, Timer } from "lucide-react";
import { SPRING } from "./glass";
import { cn } from "@/lib/utils";

type OgsStatus = {
  available: boolean;
  files?: Record<string, number>;
  status?: { lobby?: boolean; match?: boolean; with_ogs?: boolean; apk?: string } | null;
  updated?: string | null;
  note?: string;
};

function Card({ className, children }: { className?: string; children: React.ReactNode }) {
  return (
    <div
      className={cn(
        "owen-glass backdrop-blur-[26px] backdrop-saturate-[1.75] rounded-3xl p-5 sm:p-6",
        className,
      )}
    >
      {children}
    </div>
  );
}

function Shot({ name, title, subtitle, ok }: { name: string; title: string; subtitle: string; ok?: boolean }) {
  return (
    <Card className="overflow-hidden">
      <div className="mb-3 flex items-center justify-between gap-3">
        <div>
          <h3 className="text-[15px] font-black" dir="auto">
            {title}
          </h3>
          <p className="text-[12px] text-[var(--owen-muted)]" dir="auto">
            {subtitle}
          </p>
        </div>
        <span
          className={cn(
            "rounded-full px-3 py-1 text-[11px] font-black",
            ok === true
              ? "bg-emerald-400/15 text-emerald-300"
              : ok === false
                ? "bg-rose-400/15 text-rose-300"
                : "bg-white/10 text-[var(--owen-muted)]",
          )}
        >
          {ok === true ? "✔ proven" : ok === false ? "✖ failed" : "… pending"}
        </span>
      </div>
      <div className="relative overflow-hidden rounded-2xl bg-black/40 ring-1 ring-white/10">
        <img
          src={`/api/ogs-e2e?image=${name}`}
          alt={`${title} — real Android emulator screenshot`}
          className="aspect-video w-full object-cover"
          loading="lazy"
          onError={(e) => {
            const el = e.currentTarget;
            el.style.display = "none";
            el.parentElement?.querySelector("[data-empty]")?.setAttribute("data-show", "1");
          }}
        />
        <div
          data-empty
          className="absolute inset-0 hidden aspect-video w-full items-center justify-center p-6 text-center"
        >
          <div>
            <Timer className="mx-auto mb-2 h-6 w-6 opacity-60" />
            <p className="text-[13px] font-bold text-[var(--owen-muted)]" dir="auto">
              السلسلة قيد التشغيل — لقطة الشاشة تُلتقط داخل المحاكي وتُضاف هنا تلقائياً
              <br />
              <span dir="ltr" className="text-[11px] opacity-70">
                ogs-1241-e2e: dump → build → play (GitHub Actions)
              </span>
            </p>
          </div>
        </div>
      </div>
    </Card>
  );
}

export function OgsEvidence() {
  const [data, setData] = useState<OgsStatus | null>(null);
  const [loading, setLoading] = useState(false);

  const load = useCallback(async () => {
    setLoading(true);
    try {
      const r = await fetch("/api/ogs-e2e", { cache: "no-store" });
      if (r.ok) setData(await r.json());
    } catch {
      /* keep the last state */
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    load();
    const iv = setInterval(load, 30000);
    return () => clearInterval(iv);
  }, [load]);

  const st = data?.status ?? null;
  const updated =
    data?.updated && !Number.isNaN(Date.parse(data.updated))
      ? new Date(data.updated).toLocaleString("en-GB", { hour12: false })
      : null;

  return (
    <div className="space-y-5">
      <Card>
        <div className="flex flex-wrap items-start justify-between gap-4">
          <div className="min-w-0">
            <h2 className="text-xl font-black" dir="auto">
              OGS 12.41 → Android · اختبار حقيقي داخل اللعبة
            </h2>
            <p className="mt-1 text-[13px] leading-relaxed text-[var(--owen-muted)]" dir="auto">
              تحويل OGS-12.41 إلى Android واختباره من داخل اللعبة نفسها على محاكي Android في GitHub Actions:
              MobileDumper-7 يولّد الـ SDK من اللعبة الحيّة، ثم يُبنى <span dir="ltr">libOwenOGS.so</span> ويُحقن
              عبر الـ shim (بدون أي جراحة ELF للمحرك)، واللعبة تُقاد حتى الـ lobby ثم داخل مباراة حقيقية.
            </p>
          </div>
          <div className="flex items-center gap-2">
            {updated && (
              <span className="rounded-full bg-white/10 px-3 py-1 text-[11px] font-bold text-[var(--owen-muted)]" dir="ltr">
                {updated}
              </span>
            )}
            <button
              type="button"
              onClick={load}
              className="owen-glass-soft flex items-center gap-2 rounded-full px-3.5 py-2 text-[12px] font-bold text-[var(--owen-muted)] transition-colors hover:text-[var(--owen-text)]"
            >
              <RefreshCw className={cn("h-4 w-4", loading && "animate-spin")} />
              <span dir="auto">تحديث</span>
            </button>
          </div>
        </div>
        <div className="mt-4 grid grid-cols-2 gap-2 sm:grid-cols-4">
          {[
            { k: "SDK dump", v: data?.files?.["ogs-1241-android.tar.zst"] ? "MobileDumper-7 ✔" : "…", ok: !!data?.files?.["ogs-1241-android.tar.zst"] },
            { k: "libOwenOGS.so", v: st?.with_ogs ? "built ✔" : "…", ok: !!st?.with_ogs },
            { k: "Lobby", v: st?.lobby ? "in-game ✔" : st ? "✖" : "…", ok: st?.lobby === true },
            { k: "Match", v: st?.match ? "in-game ✔" : st ? "✖" : "…", ok: st?.match === true },
          ].map((b) => (
            <div key={b.k} className="rounded-2xl bg-black/20 p-3 text-center ring-1 ring-white/10">
              <div className="text-[11px] font-bold uppercase tracking-wide text-[var(--owen-faint)]" dir="ltr">
                {b.k}
              </div>
              <div
                className={cn(
                  "mt-1 text-[13px] font-black",
                  b.ok ? "text-emerald-300" : "text-[var(--owen-muted)]",
                )}
                dir="ltr"
              >
                {b.v}
              </div>
            </div>
          ))}
        </div>
      </Card>

      <div className="grid gap-5 lg:grid-cols-2">
        <motion.div layout transition={SPRING}>
          <Shot
            name="lobby.png"
            title="داخل الـ Lobby"
            subtitle="لحظة وصول اللعبة إلى الواجهة الأمامية بعد تسجيل الدخول عبر Owen Server — لقطة حقيقية من المحاكي"
            ok={st?.lobby === true ? true : st ? false : undefined}
          />
        </motion.div>
        <motion.div layout transition={SPRING}>
          <Shot
            name="match.png"
            title="داخل المباراة"
            subtitle="خريطة Apollo_Terrain مفتوحة بواسطة OGS (bots/loot/booths) — الدليل النهائي داخل اللعبة"
            ok={st?.match === true ? true : st ? false : undefined}
          />
        </motion.div>
      </div>

      <Card>
        <div className="mb-2 flex items-center gap-2">
          <ShieldCheck className="h-4 w-4 text-emerald-300" />
          <h3 className="text-[14px] font-black" dir="auto">
            كيف تُختبر السلسلة (بالأدلة، لا بالادّعاء)
          </h3>
        </div>
        <ol className="space-y-1.5 text-[12.5px] leading-relaxed text-[var(--owen-muted)]" dir="auto">
          <li>
            <b>Phase 1 — الـ SDK:</b> لعبة Fortnite 12.41 الأصلية (APK غير معدّل) تعمل على المحاكي، و
            <span dir="ltr"> MobileDumper-7</span> (MJx0) يقرأ ذاكرتها ويولّد CppSDK + offsets للأندرويد فعلياً.
          </li>
          <li>
            <b>Phase 2 — البناء:</b> يُترجم <span dir="ltr">libOwenOGS.so</span> (منفذ OGS بالكامل) مقابل ذلك الـ
            SDK، ويُرقَّع الـ APK بطريقة الـ shim: المحرك يصبح <span dir="ltr">libUE4real.so</span> وملف
            <span dir="ltr"> libUE4.so</span> الجديد هو محمّل 7KB — صفر جراحة ELF، فئة انهيار الحقن القديم تختفي تصميمياً.
          </li>
          <li>
            <b>Phase 3 — اللعب:</b> owen-server على المنفذ 8080 + <span dir="ltr">adb reverse</span>، اللعبة تنزّل
            محتواها من CDN الإبيك الحقيقي، تسجيل الدخول يكتمل ذاتياً عبر صفحة الـ exchange، ثم يفتح OGS الخريطة بعد
            نافذة الـ lobby — والصور أعلاه هي المخرجات الحرفية.
          </li>
        </ol>
        {data?.files && Object.keys(data.files).length > 0 && (
          <p className="mt-3 text-[11px] text-[var(--owen-faint)]" dir="ltr">
            evidence: {Object.entries(data.files).map(([k, v]) => `${k} (${(v / 1024).toFixed(0)}KiB)`).join(" · ")}
          </p>
        )}
      </Card>
    </div>
  );
}
