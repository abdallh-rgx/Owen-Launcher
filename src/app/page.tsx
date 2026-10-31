"use client";

/* Owen Control Room — غرفة تحكم أوين
 *
 * Human-in-the-loop annotation UI: a GitHub Actions runner drives an Android
 * emulator and posts PENDING tasks (screenshot + instruction). The operator
 * clicks the button location on the screenshot, describes it, sets a wait
 * duration and submits; the runner polls the answer and taps the emulator.
 *
 * The page polls GET /api/control/state every POLL_INTERVAL_MS and renders
 * the current task, the runner pipeline status and the request history.
 */

import { useCallback, useEffect, useRef, useState } from "react";
import { AnimatePresence, motion } from "framer-motion";
import {
  HEARTBEAT_STALE_MS,
  POLL_INTERVAL_MS,
  type StateDTO,
  type TaskDTO,
} from "@/lib/control-types";
import { Skeleton } from "@/components/ui/skeleton";
import { ConnectionCard } from "@/components/control/connection-card";
import { EmptyState } from "@/components/control/empty-state";
import { HeaderBar } from "@/components/control/header-bar";
import { HistoryList } from "@/components/control/history-list";
import { PipelineStatus } from "@/components/control/pipeline-status";
import { TaskCard } from "@/components/control/task-card";
import {
  beepNewTask,
  type AnswerPayload,
  type ApiResult,
} from "@/components/control/utils";

const CONSECUTIVE_FAILURES_FOR_ERROR = 2;
const MUTED_STORAGE_KEY = "owen-ctl-muted";

export default function Page() {
  const [state, setState] = useState<StateDTO | null>(null);
  const [connError, setConnError] = useState(false);
  const [muted, setMuted] = useState(false);

  const ctrlRef = useRef<AbortController | null>(null);
  const failStreakRef = useRef(0);
  const lastPendingIdRef = useRef<string | null>(null);
  const firstLoadRef = useRef(true);
  const mutedRef = useRef(false);
  const baseTitleRef = useRef<string | null>(null);
  const flashRef = useRef<{ iv: number | null; to: number | null }>({
    iv: null,
    to: null,
  });

  /* ---------- كتم الصوت (محفوظ في localStorage) ---------- */
  useEffect(() => {
    try {
      setMuted(window.localStorage.getItem(MUTED_STORAGE_KEY) === "1");
    } catch {
      /* ignore */
    }
  }, []);

  useEffect(() => {
    mutedRef.current = muted;
  }, [muted]);

  const toggleMute = useCallback(() => {
    setMuted((m) => {
      const v = !m;
      try {
        window.localStorage.setItem(MUTED_STORAGE_KEY, v ? "1" : "0");
      } catch {
        /* ignore */
      }
      return v;
    });
  }, []);

  /* ---------- وميض عنوان الصفحة عند وصول طلب جديد ---------- */
  const stopTitleFlash = useCallback(() => {
    if (flashRef.current.iv !== null) window.clearInterval(flashRef.current.iv);
    if (flashRef.current.to !== null) window.clearTimeout(flashRef.current.to);
    if (baseTitleRef.current !== null) document.title = baseTitleRef.current;
    flashRef.current = { iv: null, to: null };
  }, []);

  const flashTitle = useCallback(() => {
    stopTitleFlash();
    const original = baseTitleRef.current ?? document.title;
    let on = false;
    const iv = window.setInterval(() => {
      on = !on;
      document.title = on ? "🔔 طلب جديد!" : original;
    }, 800);
    const to = window.setTimeout(() => stopTitleFlash(), 6000);
    flashRef.current = { iv, to };
  }, [stopTitleFlash]);

  useEffect(() => {
    baseTitleRef.current = document.title;
    return () => stopTitleFlash();
  }, [stopTitleFlash]);

  /* ---------- جلب الحالة (استقصاء كل ثانيتين) ---------- */
  const fetchState = useCallback(async () => {
    ctrlRef.current?.abort();
    const ctrl = new AbortController();
    ctrlRef.current = ctrl;
    try {
      const res = await fetch("/api/control/state", {
        cache: "no-store",
        signal: ctrl.signal,
      });
      if (!res.ok) throw new Error(`HTTP ${res.status}`);
      const data = (await res.json()) as StateDTO;
      if (ctrl.signal.aborted) return;
      setState(data);
      failStreakRef.current = 0;
      setConnError(false);

      // كشف الطلب الجديد → صافرة + وميض العنوان
      const id = data.current?.id ?? null;
      if (firstLoadRef.current) {
        firstLoadRef.current = false;
      } else if (id !== null && id !== lastPendingIdRef.current) {
        if (!mutedRef.current) beepNewTask();
        flashTitle();
      }
      lastPendingIdRef.current = id;
    } catch (err) {
      if (err instanceof DOMException && err.name === "AbortError") return;
      failStreakRef.current += 1;
      if (failStreakRef.current >= CONSECUTIVE_FAILURES_FOR_ERROR) {
        setConnError(true);
      }
    }
  }, [flashTitle]);

  useEffect(() => {
    void fetchState();
    const iv = window.setInterval(() => void fetchState(), POLL_INTERVAL_MS);
    return () => {
      window.clearInterval(iv);
      ctrlRef.current?.abort();
    };
  }, [fetchState]);

  /* إعادة الجلب فور عودة التبويب إلى الواجهة */
  useEffect(() => {
    const onVis = () => {
      if (document.visibilityState === "visible") void fetchState();
    };
    document.addEventListener("visibilitychange", onVis);
    return () => document.removeEventListener("visibilitychange", onVis);
  }, [fetchState]);

  /* ---------- نداءات الـ API ---------- */
  const submitAnswer = useCallback(
    async (task: TaskDTO, payload: AnswerPayload): Promise<ApiResult> => {
      try {
        const res = await fetch(
          `/api/control/tasks/${encodeURIComponent(task.id)}/answer`,
          {
            method: "POST",
            headers: { "Content-Type": "application/json" },
            body: JSON.stringify(payload),
            cache: "no-store",
          },
        );
        const data = (await res.json().catch(() => ({}))) as {
          ok?: boolean;
          error?: string;
        };
        if (!res.ok || !data.ok) {
          return { ok: false, error: data.error ?? `فشل الإرسال (${res.status})` };
        }
        await fetchState();
        return { ok: true };
      } catch {
        return { ok: false, error: "مشكلة في الاتصال بالموقع" };
      }
    },
    [fetchState],
  );

  const retakeTask = useCallback(
    async (task: TaskDTO): Promise<ApiResult> => {
      try {
        const res = await fetch(
          `/api/control/tasks/${encodeURIComponent(task.id)}/retake`,
          { method: "POST", cache: "no-store" },
        );
        const data = (await res.json().catch(() => ({}))) as {
          ok?: boolean;
          error?: string;
        };
        if (!res.ok || !data.ok) {
          return { ok: false, error: data.error ?? "تعذّر طلب لقطة جديدة" };
        }
        await fetchState();
        return { ok: true };
      } catch {
        return { ok: false, error: "مشكلة في الاتصال بالموقع" };
      }
    },
    [fetchState],
  );

  const current = state?.current ?? null;
  const runnerLive =
    state?.heartbeat != null &&
    Date.now() - new Date(state.heartbeat.updatedAt).getTime() <
      HEARTBEAT_STALE_MS;

  return (
    <div className="flex min-h-screen flex-col bg-[#05060b] text-[#f4f6fb]">
      <HeaderBar
        heartbeat={state?.heartbeat ?? null}
        pending={state?.stats.pending ?? 0}
        connError={connError}
        muted={muted}
        onToggleMute={toggleMute}
      />

      <main className="mx-auto w-full max-w-7xl flex-1 px-4 py-5 sm:px-6 sm:py-6">
        {state === null ? (
          /* الهيكل الأولي أثناء التحميل */
          <div className="grid gap-5 lg:grid-cols-3 lg:gap-6">
            <div className="space-y-5 lg:col-start-2 lg:col-span-2 lg:row-start-1">
              <Skeleton className="h-[520px] w-full rounded-2xl bg-white/[0.05]" />
            </div>
            <aside className="space-y-5 lg:col-start-1 lg:row-start-1">
              <Skeleton className="h-72 w-full rounded-2xl bg-white/[0.05]" />
              <Skeleton className="h-60 w-full rounded-2xl bg-white/[0.05]" />
              <Skeleton className="h-80 w-full rounded-2xl bg-white/[0.05]" />
            </aside>
          </div>
        ) : (
          <div className="grid items-start gap-5 lg:grid-cols-3 lg:gap-6">
            {/* بطاقة المهمة الحالية — عمودان */}
            <div className="lg:col-start-2 lg:col-span-2 lg:row-start-1">
              <AnimatePresence mode="wait">
                {current ? (
                  <motion.div
                    key={current.id}
                    initial={{ opacity: 0, y: 14 }}
                    animate={{ opacity: 1, y: 0 }}
                    exit={{ opacity: 0, y: -10 }}
                    transition={{ duration: 0.25, ease: "easeOut" }}
                  >
                    <TaskCard
                      task={current}
                      queueCount={state.queue.length}
                      onAnswer={(payload) => submitAnswer(current, payload)}
                      onRetake={() => retakeTask(current)}
                    />
                  </motion.div>
                ) : (
                  <motion.div
                    key="empty"
                    initial={{ opacity: 0, y: 14 }}
                    animate={{ opacity: 1, y: 0 }}
                    exit={{ opacity: 0, y: -10 }}
                    transition={{ duration: 0.25, ease: "easeOut" }}
                  >
                    <EmptyState runnerLive={runnerLive} />
                  </motion.div>
                )}
              </AnimatePresence>
            </div>

            {/* الشريط الجانبي — العمود الأيمن */}
            <aside
              className="space-y-5 lg:col-start-1 lg:row-start-1"
              aria-label="لوحة التشغيل والمتابعة"
            >
              <ConnectionCard token={state.token} />
              <PipelineStatus
                heartbeat={state.heartbeat}
                stats={state.stats}
              />
              <HistoryList recent={state.recent} />
            </aside>
          </div>
        )}
      </main>

      <footer className="mt-auto border-t border-white/10 bg-black/30">
        <div className="mx-auto w-full max-w-7xl px-4 py-4 pb-[calc(1rem+env(safe-area-inset-bottom))] text-center text-[11px] leading-relaxed text-zinc-500">
          Owen Control Room — القيادة اليدوية للمحاكي • يعمل الموقع بلا توقف،
          والـ Action يشغّل المحاكي حتى ٦ ساعات
        </div>
      </footer>
    </div>
  );
}
