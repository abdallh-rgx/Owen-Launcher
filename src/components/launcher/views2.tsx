"use client";

/* Owen Liquid Glass — PartyHub, Profile, Settings and Offline views. */

import { useEffect, useMemo, useState } from "react";
import { AnimatePresence, motion } from "framer-motion";
import {
  AtSign,
  Cpu,
  ExternalLink,
  Languages,
  LogOut,
  Moon,
  Palette,
  RefreshCw,
  Rss,
  ShieldCheck,
  Smartphone,
  Sun,
  WifiOff,
} from "lucide-react";
import {
  api,
  collectCosmetics,
  KIND_COLORS,
  seasonGradient,
  tokenStore,
  type Account,
  type Dict,
  type Lang,
  type McpProfile,
  type SeasonMeta,
  type ServerStatus,
} from "@/lib/launcher/api";
import {
  CountUp,
  GlassButton,
  GlassCard,
  LogoMark,
  Pill,
  ProgressRing,
  SPRING,
  Stagger,
  StaggerItem,
  StatusDot,
} from "./glass";
import { SectionTitle } from "./views";
import { cn } from "@/lib/utils";

/* ============================== PARTYHUB ============================== */

export function PartyHubView({ d, status }: { d: Dict; status: ServerStatus | null }) {
  const [nonce, setNonce] = useState(0);
  const [loaded, setLoaded] = useState(false);
  const live = status?.partyhub === true;

  return (
    <div className="space-y-5">
      <div>
        <SectionTitle title={d.partyhubTitle} icon={<Smartphone size={15} />} />
        <p className="mt-1 text-[12.5px] text-[var(--owen-muted)]">{d.partyhubSub}</p>
      </div>

      <GlassCard className="p-5">
        <div className="mb-4 flex flex-wrap items-center justify-between gap-3">
          <div className="flex items-center gap-2">
            <StatusDot ok={live} />
            <span className="text-[13px] font-bold">{live ? d.liveNow : d.serverOffline}</span>
            <Pill tone="accent">bridge v4</Pill>
          </div>
          <div className="flex gap-2">
            <GlassButton variant="ghost" className="h-9 px-4 text-[12.5px]" onClick={() => setNonce((n) => n + 1)}>
              <RefreshCw size={13} /> {d.reload}
            </GlassButton>
            <GlassButton
              variant="ghost"
              className="h-9 px-4 text-[12.5px]"
              onClick={() => window.open("/partyhub/offline/en-US/", "_blank")}
            >
              <ExternalLink size={13} /> {d.openFull}
            </GlassButton>
          </div>
        </div>

        {/* phone frame */}
        <div className="flex justify-center">
          <motion.div
            initial={{ opacity: 0, y: 30, rotateY: -8 }}
            animate={{ opacity: 1, y: 0, rotateY: 0 }}
            transition={{ ...SPRING, delay: 0.15 }}
            className="owen-glass backdrop-blur-[26px] backdrop-saturate-[1.75] relative w-full max-w-[380px] rounded-[44px] p-3"
          >
            <div className="absolute inset-x-0 top-3 z-10 flex justify-center">
              <div className="h-6 w-28 rounded-b-2xl bg-black/85" />
            </div>
            <div className="relative h-[640px] max-h-[70vh] overflow-hidden rounded-[34px] bg-[#0a0c12]">
              {live ? (
                <iframe
                  key={nonce}
                  src="/partyhub/offline/en-US/"
                  title="Owen PartyHub"
                  onLoad={() => setLoaded(true)}
                  className="h-full w-full border-0"
                  allow="clipboard-read; clipboard-write"
                />
              ) : (
                <div className="flex h-full flex-col items-center justify-center gap-4 p-8 text-center">
                  <WifiOff size={40} className="text-[var(--owen-faint)]" />
                  <p className="text-sm text-[var(--owen-muted)]">{d.offlineSub}</p>
                </div>
              )}
              {!loaded && live && (
                <div className="absolute inset-0 flex items-center justify-center bg-[#0a0c12]">
                  <div
                    className="h-10 w-10 rounded-full border-[3px] border-white/10 border-t-[var(--owen-a)]"
                    style={{ animation: "owen-spin .8s linear infinite" }}
                  />
                </div>
              )}
            </div>
          </motion.div>
        </div>
      </GlassCard>
    </div>
  );
}

/* ============================== PROFILE ============================== */

export function ProfileView({
  d,
  account,
  seasons,
  onLogout,
  refreshKey,
}: {
  d: Dict;
  account: Account;
  seasons: SeasonMeta[];
  onLogout: () => void;
  refreshKey: number;
}) {
  const [mcp, setMcp] = useState<McpProfile | null>(null);

  useEffect(() => {
    api
      .queryProfile(account.accountId, "athena", account.selectedSeason)
      .then(setMcp)
      .catch(() => setMcp(null));
  }, [account.accountId, account.selectedSeason, refreshKey]);

  const profile = mcp?.profileChanges?.[0]?.profile;
  const attrs = (profile?.stats?.attributes || {}) as Record<string, unknown>;
  const items = profile?.items || [];
  const locker = useMemo(() => collectCosmetics([]), []);
  void locker;
  const kinds = useMemo(() => {
    const counts: Record<string, number> = {};
    for (const it of items) {
      const k = (it.templateId.split(":")[0] || "other").replace("Athena", "").toLowerCase() || "other";
      counts[k] = (counts[k] || 0) + 1;
    }
    return Object.entries(counts).sort((a, b) => b[1] - a[1]).slice(0, 6);
  }, [items]);

  const season = seasons.find((s) => s.season === account.selectedSeason);
  const created = new Date(account.created).toLocaleDateString(
    undefined,
    { year: "numeric", month: "long", day: "numeric" },
  );

  return (
    <div className="space-y-5">
      <SectionTitle title={d.myProfile} icon={<AtSign size={15} />} />

      <GlassCard className="p-6">
        <div className="flex flex-wrap items-center gap-6">
          <ProgressRing progress={((account.level % 100) || 1) / 100} size={110}>
            <span className="text-3xl font-black tabular-nums">
              <CountUp value={account.level} />
            </span>
            <span className="text-[10px] font-bold text-[var(--owen-faint)]">{d.level}</span>
          </ProgressRing>
          <div className="min-w-0 flex-1">
            <h3 className="truncate text-2xl font-black">{account.displayName}</h3>
            <p className="text-[13px] text-[var(--owen-muted)]" dir="ltr">
              @{account.username} · {account.email}
            </p>
            <div className="mt-3 flex flex-wrap gap-2">
              <Pill tone={account.battlePassPurchased ? "ok" : "neutral"}>
                <ShieldCheck size={12} />
                {account.battlePassPurchased ? d.battlePassOwned : d.notOwned}
              </Pill>
              <Pill>
                {d.selectedSeason}: {season?.name || `S${account.selectedSeason}`}
              </Pill>
              <Pill>
                {d.memberSince} {created}
              </Pill>
            </div>
          </div>
        </div>
      </GlassCard>

      <div className="grid grid-cols-2 gap-4 sm:grid-cols-3">
        <GlassCard delay={0.05} hover className="p-5 text-center">
          <div className="text-3xl font-black tabular-nums text-emerald-300">
            <CountUp value={account.vbucks} />
          </div>
          <div className="mt-1 text-[11.5px] font-bold text-[var(--owen-muted)]">{d.vbucks}</div>
        </GlassCard>
        <GlassCard delay={0.1} hover className="p-5 text-center">
          <div className="text-3xl font-black tabular-nums text-amber-300">
            <CountUp value={items.length} />
          </div>
          <div className="mt-1 text-[11.5px] font-bold text-[var(--owen-muted)]">{d.locker}</div>
        </GlassCard>
        <GlassCard delay={0.15} hover className="p-5 text-center">
          <div className="text-3xl font-black tabular-nums text-fuchsia-300">
            <CountUp value={Number(attrs.book_level ?? account.level) || 0} />
          </div>
          <div className="mt-1 text-[11.5px] font-bold text-[var(--owen-muted)]">{d.battlePass}</div>
        </GlassCard>
      </div>

      {kinds.length > 0 && (
        <GlassCard className="p-5" delay={0.2}>
          <div className="mb-3 text-[13px] font-bold text-[var(--owen-muted)]">{d.cosmetics}</div>
          <div className="grid grid-cols-2 gap-3 sm:grid-cols-3 lg:grid-cols-6">
            {kinds.map(([kind, count], i) => (
              <motion.div
                key={kind}
                initial={{ opacity: 0, scale: 0.85 }}
                animate={{ opacity: 1, scale: 1 }}
                transition={{ ...SPRING, delay: 0.2 + i * 0.05 }}
                className={cn(
                  "rounded-2xl bg-gradient-to-br p-4 text-center text-white",
                  KIND_COLORS[kind] || "from-zinc-500 to-zinc-700",
                )}
              >
                <div className="text-2xl font-black tabular-nums">{count}</div>
                <div className="text-[10.5px] font-bold opacity-85">{kind}</div>
              </motion.div>
            ))}
          </div>
        </GlassCard>
      )}

      <div className="flex justify-center">
        <GlassButton variant="danger" className="h-12 px-8" onClick={onLogout}>
          <LogOut size={16} /> {d.logout}
        </GlassButton>
      </div>
    </div>
  );
}

/* ============================== SETTINGS ============================== */

const CRATES = [
  ["owen-core", "profiles · manifests · battlepass · ELF"],
  ["owen-server", "LawinServer v2 + Reload · PartyHub · accounts"],
  ["owen-redirecter", "libowen.so — curl + ART hooks"],
  ["owen-detect", "curl_easy_setopt locator"],
  ["owen-hook", "AArch64 trampolines"],
  ["owen-art", "runtime method hooking"],
  ["owen-patcher", "DT_NEEDED injection · APK surgery"],
  ["owen-pak", "UE4 pak reader"],
  ["owen-cli", "the all-in-one tool"],
];

export function SettingsView({
  d,
  lang,
  onLang,
  theme,
  onTheme,
  status,
}: {
  d: Dict;
  lang: Lang;
  onLang: (l: Lang) => void;
  theme: "dark" | "light";
  onTheme: (t: "dark" | "light") => void;
  status: ServerStatus | null;
}) {
  return (
    <div className="space-y-5">
      <SectionTitle title={d.settings} icon={<Palette size={15} />} />

      <Stagger className="space-y-4" gap={0.08}>
        <StaggerItem>
          <GlassCard className="p-5">
            <div className="mb-4 flex items-center gap-2 text-[14px] font-extrabold">
              <Languages size={16} className="text-[var(--owen-a)]" /> {d.language}
            </div>
            <div className="flex gap-2">
              {(["ar", "en"] as Lang[]).map((l) => (
                <button
                  key={l}
                  type="button"
                  onClick={() => onLang(l)}
                  className={cn(
                    "relative flex-1 rounded-2xl py-3 text-[13.5px] font-bold transition-colors",
                    lang === l ? "text-[#0b0616]" : "owen-glass-soft text-[var(--owen-muted)]",
                  )}
                >
                  {lang === l && (
                    <motion.span
                      layoutId="lang-pill"
                      transition={SPRING}
                      className="owen-accent-grad absolute inset-0 rounded-2xl"
                    />
                  )}
                  <span className="relative z-10">{l === "ar" ? "العربية" : "English"}</span>
                </button>
              ))}
            </div>
          </GlassCard>
        </StaggerItem>

        <StaggerItem>
          <GlassCard className="p-5">
            <div className="mb-4 flex items-center gap-2 text-[14px] font-extrabold">
              <Palette size={16} className="text-[var(--owen-a)]" /> {d.appearance}
            </div>
            <div className="flex gap-2">
              {(["dark", "light"] as const).map((t) => (
                <button
                  key={t}
                  type="button"
                  onClick={() => onTheme(t)}
                  className={cn(
                    "relative flex flex-1 items-center justify-center gap-2 rounded-2xl py-3 text-[13.5px] font-bold transition-colors",
                    theme === t ? "text-[#0b0616]" : "owen-glass-soft text-[var(--owen-muted)]",
                  )}
                >
                  {theme === t && (
                    <motion.span
                      layoutId="theme-pill"
                      transition={SPRING}
                      className="owen-accent-grad absolute inset-0 rounded-2xl"
                    />
                  )}
                  <span className="relative z-10 flex items-center gap-2">
                    {t === "dark" ? <Moon size={14} /> : <Sun size={14} />}
                    {t === "dark" ? d.dark : d.light}
                  </span>
                </button>
              ))}
            </div>
          </GlassCard>
        </StaggerItem>

        <StaggerItem>
          <GlassCard className="p-5">
            <div className="mb-4 flex items-center gap-2 text-[14px] font-extrabold">
              <Cpu size={16} className="text-[var(--owen-a)]" /> {d.serverInfo}
            </div>
            <dl className="grid grid-cols-2 gap-x-4 gap-y-3 text-[13px] sm:grid-cols-3">
              <Info2 label={d.backend} value="owen-server" />
              <Info2 label={d.version} value={status?.version ?? "—"} />
              <Info2 label={d.uptime} value={fmtUptime(status?.uptime_seconds)} />
              <Info2 label={d.accountsOnline} value={String(status?.accounts ?? "—")} />
              <Info2 label={d.seasons} value={String(status?.seasons ?? "—")} />
              <Info2 label={d.xmpp} value={String(status?.xmpp_port ?? "—")} />
            </dl>
          </GlassCard>
        </StaggerItem>

        <StaggerItem>
          <GlassCard className="p-5">
            <div className="mb-2 flex items-center gap-2 text-[14px] font-extrabold">
              <Rss size={16} className="text-[var(--owen-a)]" /> {d.redirecter}
            </div>
            <p className="mb-4 text-[12.5px] text-[var(--owen-muted)]">{d.redirecterSub}</p>
            <div className="flex flex-wrap gap-2">
              <Pill tone="ok">curl_easy_setopt hook</Pill>
              <Pill tone="ok">WebView.loadUrl</Pill>
              <Pill tone="ok">DebouncedLimiter</Pill>
              <Pill tone="ok">cleartext unlock</Pill>
              <Pill tone="ok">auto manifest</Pill>
            </div>
          </GlassCard>
        </StaggerItem>

        <StaggerItem>
          <GlassCard className="p-5">
            <div className="mb-2 flex items-center gap-2 text-[14px] font-extrabold">
              <Cpu size={16} className="text-[var(--owen-a)]" /> {d.crates}
            </div>
            <div className="mt-3 space-y-2">
              {CRATES.map(([name, desc], i) => (
                <motion.div
                  key={name}
                  initial={{ opacity: 0, x: 14 }}
                  animate={{ opacity: 1, x: 0 }}
                  transition={{ delay: 0.1 + i * 0.04 }}
                  className="flex flex-wrap items-center justify-between gap-2 rounded-xl bg-white/[.04] px-3.5 py-2.5"
                >
                  <code className="font-mono text-[12.5px] font-bold text-[var(--owen-a)]" dir="ltr">
                    {name}
                  </code>
                  <span className="text-[11.5px] text-[var(--owen-faint)]" dir="ltr">
                    {desc}
                  </span>
                </motion.div>
              ))}
            </div>
          </GlassCard>
        </StaggerItem>

        <StaggerItem>
          <GlassCard className="p-5">
            <div className="mb-2 flex items-center gap-3">
              <LogoMark size={40} />
              <div>
                <div className="text-[15px] font-extrabold">{d.about}</div>
                <div className="owen-text-grad text-[11px] font-bold tracking-[3px]">RUST · SAFE · 1:1</div>
              </div>
            </div>
            <p className="text-[13px] leading-relaxed text-[var(--owen-muted)]">{d.aboutBody}</p>
          </GlassCard>
        </StaggerItem>
      </Stagger>
    </div>
  );
}

function Info2({ label, value }: { label: string; value: string }) {
  return (
    <div className="rounded-xl bg-white/[.04] px-3.5 py-2.5">
      <dt className="text-[10.5px] font-bold text-[var(--owen-faint)]">{label}</dt>
      <dd className="mt-0.5 truncate font-bold" dir="ltr">
        {value}
      </dd>
    </div>
  );
}

function fmtUptime(secs: number | undefined): string {
  if (!secs) return "—";
  const h = Math.floor(secs / 3600);
  const m = Math.floor((secs % 3600) / 60);
  const s = secs % 60;
  return `${h ? h + "h " : ""}${m}m ${s}s`.trim();
}

/* ============================== OFFLINE ============================== */

export function OfflineView({ d, onRetry }: { d: Dict; onRetry: () => void }) {
  const [count, setCount] = useState(8);
  useEffect(() => {
    const t = setInterval(
      () =>
        setCount((c) => {
          if (c <= 1) {
            onRetry();
            return 8;
          }
          return c - 1;
        }),
      1000,
    );
    return () => clearInterval(t);
  }, [onRetry]);
  return (
    <div className="relative z-10 flex min-h-dvh items-center justify-center p-6">
      <GlassCard className="max-w-md p-8 text-center">
        <motion.div
          initial={{ scale: 0.6, opacity: 0 }}
          animate={{ scale: 1, opacity: 1 }}
          transition={SPRING}
          className="mx-auto mb-5 flex h-20 w-20 items-center justify-center rounded-3xl bg-rose-400/12"
        >
          <WifiOff size={38} className="text-rose-300" />
        </motion.div>
        <h2 className="text-2xl font-black">{d.offlineTitle}</h2>
        <p className="mt-2 text-[13.5px] leading-relaxed text-[var(--owen-muted)]">{d.offlineSub}</p>
        <div className="mt-6 flex items-center justify-center gap-3">
          <GlassButton variant="ghost" className="h-11 px-6" onClick={() => setCount(8)}>
            <RefreshCw size={15} /> {d.retryNow}
          </GlassButton>
          <Pill>
            {d.retrying} {count}s
          </Pill>
        </div>
      </GlassCard>
    </div>
  );
}
