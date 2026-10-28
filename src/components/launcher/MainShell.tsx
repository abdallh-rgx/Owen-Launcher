"use client";

/* Owen Liquid Glass — the main shell: adaptive tab navigation, top bar and
 * the sticky glass status bar. */

import { AnimatePresence, motion } from "framer-motion";
import { Download, Home, Layers, Settings2, Smartphone, User } from "lucide-react";
import type { ReactNode } from "react";
import type { Dict, Lang } from "@/lib/launcher/i18n";
import type { ServerStatus } from "@/lib/launcher/api";
import { SPRING, StatusDot, Wordmark } from "./glass";
import { cn } from "@/lib/utils";

export type ViewId = "home" | "seasons" | "downloads" | "partyhub" | "profile" | "settings";

export function MainShell({
  d,
  lang,
  onLang,
  theme,
  onTheme,
  view,
  onView,
  status,
  online,
  ping,
  children,
}: {
  d: Dict;
  lang: Lang;
  onLang: (l: Lang) => void;
  theme: "dark" | "light";
  onTheme: (t: "dark" | "light") => void;
  view: ViewId;
  onView: (v: ViewId) => void;
  status: ServerStatus | null;
  online: boolean;
  ping: number | null;
  children: ReactNode;
}) {
  const tabs: Array<{ id: ViewId; label: string; icon: ReactNode }> = [
    { id: "home", label: d.home, icon: <Home size={20} /> },
    { id: "seasons", label: d.seasons, icon: <Layers size={20} /> },
    { id: "downloads", label: d.downloads, icon: <Download size={20} /> },
    { id: "partyhub", label: d.partyhub, icon: <Smartphone size={20} /> },
    { id: "profile", label: d.profile, icon: <User size={20} /> },
    { id: "settings", label: d.settings, icon: <Settings2 size={20} /> },
  ];

  return (
    <div className="relative z-10 flex min-h-dvh flex-col">
      {/* top bar */}
      <header className="sticky top-0 z-30 px-4 pt-4 sm:px-6">
        <div className="owen-glass backdrop-blur-[26px] backdrop-saturate-[1.75] mx-auto flex max-w-6xl items-center justify-between rounded-3xl px-4 py-3">
          <div className="flex items-center gap-3">
            <div className="owen-accent-grad flex h-9 w-9 items-center justify-center rounded-xl text-sm font-black text-white">
              O
            </div>
            <Wordmark brand={d.brand} launcher={d.launcher} />
          </div>
          <div className="flex items-center gap-2">
            <button
              type="button"
              onClick={() => onLang(lang === "ar" ? "en" : "ar")}
              className="owen-glass-soft rounded-full px-3.5 py-1.5 text-[12px] font-bold text-[var(--owen-muted)] transition-colors hover:text-[var(--owen-text)]"
              aria-label="language"
            >
              {lang === "ar" ? "EN" : "ع"}
            </button>
            <button
              type="button"
              onClick={() => onTheme(theme === "dark" ? "light" : "dark")}
              className="owen-glass-soft rounded-full px-3.5 py-1.5 text-[12px] font-bold text-[var(--owen-muted)] transition-colors hover:text-[var(--owen-text)]"
              aria-label="theme"
            >
              {theme === "dark" ? "☀" : "☾"}
            </button>
          </div>
        </div>
      </header>

      {/* body: desktop rail + content / mobile stacked */}
      <div className="mx-auto flex w-full max-w-6xl flex-1 gap-6 px-4 pb-36 pt-6 sm:px-6 lg:pb-10">
        {/* desktop rail */}
        <nav className="hidden lg:block" aria-label="primary">
          <div className="owen-glass backdrop-blur-[26px] backdrop-saturate-[1.75] sticky top-24 flex w-[190px] flex-col gap-1.5 p-3">
            {tabs.map((t) => (
              <TabButton key={t.id} t={t} active={view === t.id} onClick={() => onView(t.id)} />
            ))}
          </div>
        </nav>

        {/* content */}
        <main className="min-w-0 flex-1">
          <AnimatePresence mode="wait">
            <motion.div
              key={view}
              initial={{ opacity: 0, y: 26, filter: "blur(8px)", scale: 0.985 }}
              animate={{ opacity: 1, y: 0, filter: "blur(0px)", scale: 1 }}
              exit={{ opacity: 0, y: -18, filter: "blur(8px)", scale: 0.99 }}
              transition={{ ...SPRING, stiffness: 220, damping: 26 }}
            >
              {children}
            </motion.div>
          </AnimatePresence>
        </main>
      </div>

      {/* mobile bottom tab bar */}
      <nav
        className="fixed inset-x-0 bottom-0 z-40 px-4 pb-[max(env(safe-area-inset-bottom),12px)] pt-2 lg:hidden"
        aria-label="primary"
      >
        <div className="owen-glass backdrop-blur-[26px] backdrop-saturate-[1.75] mx-auto flex max-w-md items-center justify-between rounded-[26px] p-2">
          {tabs.map((t) => (
            <button
              key={t.id}
              type="button"
              onClick={() => onView(t.id)}
              className={cn(
                "relative flex flex-1 flex-col items-center gap-1 rounded-2xl py-2 text-[10px] font-bold transition-colors",
                view === t.id ? "text-[var(--owen-text)]" : "text-[var(--owen-faint)]",
              )}
            >
              {view === t.id && (
                <motion.span
                  layoutId="tab-pill"
                  transition={SPRING}
                  className="absolute inset-0 rounded-2xl bg-white/10"
                />
              )}
              <span className="relative z-10">{t.icon}</span>
              <span className="relative z-10">{t.label}</span>
            </button>
          ))}
        </div>
      </nav>

      {/* sticky footer status bar */}
      <footer className="sticky bottom-0 z-30 mt-auto px-4 pb-[max(env(safe-area-inset-bottom),10px)] pt-2 lg:pb-4">
        <div className="owen-glass-soft backdrop-blur-[18px] backdrop-saturate-[1.6] mx-auto flex max-w-6xl flex-wrap items-center justify-between gap-x-4 gap-y-1 rounded-full px-5 py-2 text-[11.5px] font-semibold text-[var(--owen-muted)]">
          <span className="flex items-center gap-2">
            <StatusDot ok={online} />
            {online ? d.serverOnline : d.serverOffline}
          </span>
          <span className="hidden sm:inline" dir="ltr">
            {ping != null ? `${ping}ms` : "…"} · owen-server v{status?.version ?? "?"}
          </span>
          <span dir="ltr" className="tabular-nums">
            {status?.accounts ?? "—"} {d.accountsOnline} · {status?.seasons ?? "—"} {d.seasons}
          </span>
        </div>
      </footer>
    </div>
  );
}

function TabButton({
  t,
  active,
  onClick,
}: {
  t: { id: string; label: string; icon: ReactNode };
  active: boolean;
  onClick: () => void;
}) {
  return (
    <button
      type="button"
      onClick={onClick}
      className={cn(
        "relative flex items-center gap-3 rounded-2xl px-4 py-3 text-[13.5px] font-bold transition-colors",
        active ? "text-[var(--owen-text)]" : "text-[var(--owen-faint)] hover:text-[var(--owen-muted)]",
      )}
    >
      {active && (
        <motion.span
          layoutId="rail-pill"
          transition={SPRING}
          className="owen-accent-grad absolute inset-0 rounded-2xl shadow-[0_10px_30px_rgba(167,139,250,.4)]"
        />
      )}
      <span className={cn("relative z-10", active && "text-white")}>{t.icon}</span>
      <span className="relative z-10">{t.label}</span>
    </button>
  );
}
