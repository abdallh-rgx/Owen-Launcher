"use client";

/* Owen Launcher — Liquid Glass (iOS 26 style).
 *
 * The complete launcher experience: boot splash → authentication with real
 * accounts (email + username + password, PBKDF2-verified by the Owen Rust
 * backend) → the main launcher with seasons, downloads, the native PartyHub,
 * profile and settings — all wired live to owen-server via /owen-api.
 */

import { useCallback, useEffect, useMemo, useState } from "react";
import { AnimatePresence, animate, motion, useMotionValue } from "framer-motion";
import { toast } from "sonner";
import {
  api,
  tokenStore,
  type Account,
  type ManifestEntry,
  type SeasonMeta,
  type ServerStatus,
} from "@/lib/launcher/api";
import { tr, type Lang } from "@/lib/launcher/i18n";
import { AuroraBackground, BootSplash } from "@/components/launcher/glass";
import { AuthView } from "@/components/launcher/AuthView";
import { MainShell, type ViewId } from "@/components/launcher/MainShell";
import { DownloadsView, HomeView, SeasonsView } from "@/components/launcher/views";
import { OfflineView, PartyHubView, ProfileView, SettingsView } from "@/components/launcher/views2";
import { OgsEvidence } from "@/components/launcher/OgsEvidence";

type Phase = "boot" | "auth" | "main" | "offline";

export default function Page() {
  const [phase, setPhase] = useState<Phase>("boot");
  const [lang, setLang] = useState<Lang>("ar");
  const [theme, setTheme] = useState<"dark" | "light">("dark");
  const [view, setView] = useState<ViewId>("home");
  const [account, setAccount] = useState<Account | null>(null);
  const [status, setStatus] = useState<ServerStatus | null>(null);
  const [seasons, setSeasons] = useState<SeasonMeta[]>([]);
  const [manifests, setManifests] = useState<ManifestEntry[]>([]);
  const [ping, setPing] = useState<number | null>(null);
  const [refreshKey, setRefreshKey] = useState(0);
  const [seasonBusy, setSeasonBusy] = useState(false);
  /* The main-shell entry blur is driven by a hand-owned motion value so it can
   * END at "none": a lingering `filter: blur(0px)` on this wrapper creates a
   * containing block that breaks the mobile bottom tab bar's position:fixed. */
  const mainBlur = useMotionValue("blur(10px)");

  const d = useMemo(() => tr(lang), [lang]);

  /* ---------- theme + direction ---------- */
  useEffect(() => {
    try {
      const saved = localStorage.getItem("owen:web-lang") as Lang | null;
      if (saved === "ar" || saved === "en") setLang(saved);
      const savedTheme = localStorage.getItem("owen:web-theme");
      if (savedTheme === "light" || savedTheme === "dark") setTheme(savedTheme);
    } catch {
      /* ignore */
    }
  }, []);

  useEffect(() => {
    document.documentElement.lang = lang;
    document.documentElement.dir = lang === "ar" ? "rtl" : "ltr";
    try {
      localStorage.setItem("owen:web-lang", lang);
    } catch {
      /* ignore */
    }
  }, [lang]);

  useEffect(() => {
    document.documentElement.setAttribute("data-owen-theme", theme);
    try {
      localStorage.setItem("owen:web-theme", theme);
    } catch {
      /* ignore */
    }
  }, [theme]);

  /* ---------- data loading ---------- */
  const loadPublic = useCallback(async (): Promise<boolean> => {
    const t0 = performance.now();
    try {
      const [st, ss, mf] = await Promise.all([
        api.status(),
        api.seasons().then((r) => r.seasons).catch(() => [] as SeasonMeta[]),
        api.manifests().catch(() => [] as ManifestEntry[]),
      ]);
      setPing(Math.round(performance.now() - t0));
      setStatus(st);
      setSeasons(ss);
      setManifests(mf);
      return true;
    } catch {
      return false;
    }
  }, []);

  const refreshAccount = useCallback(async () => {
    if (!tokenStore.get()) return;
    try {
      const me = await api.me();
      setAccount(me);
      tokenStore.cacheMe(me);
    } catch {
      /* keep cached */
    }
  }, []);

  useEffect(() => {
    let cancelled = false;
    (async () => {
      const ok = await loadPublic();
      if (cancelled) return;
      if (!ok) {
        setPhase("offline");
        return;
      }
      // session restore happens inside AuthView's boot check
      setPhase((p) => (p === "boot" ? "auth" : p));
    })();
    return () => {
      cancelled = true;
    };
  }, [loadPublic]);

  /* periodic status ping */
  useEffect(() => {
    const iv = setInterval(async () => {
      const t0 = performance.now();
      try {
        const st = await api.status();
        setPing(Math.round(performance.now() - t0));
        setStatus(st);
      } catch {
        setPing(null);
      }
    }, 30000);
    return () => clearInterval(iv);
  }, []);

  /* ---------- main-shell entry blur ---------- */
  useEffect(() => {
    if (phase !== "main") return;
    mainBlur.set("blur(10px)");
    const controls = animate(mainBlur, "blur(0px)", {
      duration: 0.55,
      ease: [0.34, 1.56, 0.64, 1],
      onComplete: () => mainBlur.set("none"),
    });
    return () => controls.stop();
  }, [phase, mainBlur]);

  /* ---------- handlers ---------- */
  const onAuthed = useCallback(
    (acc: Account, _token: string) => {
      setAccount(acc);
      setPhase("main");
      setView("home");
      setRefreshKey((k) => k + 1);
    },
    [],
  );

  const onPlay = useCallback(async () => {
    if (!account) return;
    setSeasonBusy(true);
    try {
      const me = await api.setSeason(account.selectedSeason);
      setAccount(me);
      toast(`${d.playNow} — ${me.displayName} · ${d.seasonWord} ${me.selectedSeason}`);
    } catch {
      toast(d.errServer);
    } finally {
      setSeasonBusy(false);
    }
  }, [account, d]);

  const onSeasonSelect = useCallback(
    async (n: number) => {
      try {
        const me = await api.setSeason(n);
        setAccount(me);
        toast(`${d.seasonActive}: ${d.seasonWord} ${n}`);
      } catch {
        toast(d.errServer);
      }
    },
    [d],
  );

  const onLogout = useCallback(async () => {
    try {
      await api.logout();
    } catch {
      /* ignore */
    }
    tokenStore.clear();
    setAccount(null);
    setPhase("auth");
  }, []);

  const onRetry = useCallback(async () => {
    const ok = await loadPublic();
    if (ok) setPhase(account ? "main" : "auth");
  }, [account, loadPublic]);

  /* ---------- render ---------- */
  const online = phase !== "offline";

  return (
    <div
      className="relative min-h-dvh overflow-x-hidden"
      style={{ background: "var(--owen-bg)", color: "var(--owen-text)" }}
    >
      <AuroraBackground />

      <AnimatePresence mode="wait">
        {phase === "boot" && (
          <motion.div key="boot" exit={{ opacity: 0 }}>
            <div className="flex min-h-dvh items-center justify-center">
              <BootSplash label={d.loading} />
            </div>
          </motion.div>
        )}

        {phase === "offline" && (
          <motion.div key="offline" initial={{ opacity: 0 }} animate={{ opacity: 1 }} exit={{ opacity: 0 }}>
            <OfflineView d={d} onRetry={onRetry} />
          </motion.div>
        )}

        {phase === "auth" && (
          <motion.div key="auth" initial={{ opacity: 0 }} animate={{ opacity: 1 }} exit={{ opacity: 0, scale: 1.03, filter: "blur(8px)" }}>
            <AuthView d={d} lang={lang} onAuthed={onAuthed} serverUp />
          </motion.div>
        )}

        {phase === "main" && account && (
          <motion.div
            key="main"
            initial={{ opacity: 0, scale: 1.02 }}
            animate={{ opacity: 1, scale: 1 }}
            transition={{ duration: 0.55, ease: [0.34, 1.56, 0.64, 1] }}
            style={{ filter: mainBlur }}
          >
            <MainShell
              d={d}
              lang={lang}
              onLang={setLang}
              theme={theme}
              onTheme={setTheme}
              view={view}
              onView={setView}
              status={status}
              online={online}
              ping={ping}
            >
              <AnimatePresence mode="wait">
                <motion.div
                  key={refreshKey}
                  initial={{ opacity: 0 }}
                  animate={{ opacity: 1 }}
                >
                  {view === "home" && (
                    <HomeView
                      d={d}
                      account={account}
                      status={status}
                      seasons={seasons}
                      activeSeason={account.selectedSeason}
                      onPlay={onPlay}
                      seasonDetailBusy={seasonBusy}
                    />
                  )}
                  {view === "seasons" && (
                    <SeasonsView
                      d={d}
                      seasons={seasons}
                      activeSeason={account.selectedSeason}
                      onSelect={onSeasonSelect}
                      refreshKey={refreshKey}
                    />
                  )}
                  {view === "downloads" && <DownloadsView d={d} manifests={manifests} />}
                  {view === "partyhub" && <PartyHubView d={d} status={status} />}
                  {view === "ogs" && <OgsEvidence />}
                  {view === "profile" && (
                    <ProfileView
                      d={d}
                      account={account}
                      seasons={seasons}
                      onLogout={onLogout}
                      refreshKey={refreshKey}
                    />
                  )}
                  {view === "settings" && (
                    <SettingsView
                      d={d}
                      lang={lang}
                      onLang={setLang}
                      theme={theme}
                      onTheme={setTheme}
                      status={status}
                    />
                  )}
                </motion.div>
              </AnimatePresence>
            </MainShell>
          </motion.div>
        )}
      </AnimatePresence>
    </div>
  );
}
