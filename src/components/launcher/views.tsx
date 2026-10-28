"use client";

/* Owen Liquid Glass — Home, Seasons and Downloads views. */

import { useEffect, useMemo, useRef, useState } from "react";
import { AnimatePresence, motion } from "framer-motion";
import {
  Calendar,
  Check,
  Copy,
  Download as DownloadIcon,
  Gift,
  Layers,
  Play,
  Server,
  Sparkles,
  Trophy,
  Zap,
} from "lucide-react";
import { toast } from "sonner";
import {
  api,
  collectCosmetics,
  countTiers,
  KIND_COLORS,
  seasonGradient,
  type Account,
  type Dict,
  type Lang,
  type ManifestEntry,
  type SeasonMeta,
  type ServerStatus,
} from "@/lib/launcher/api";
import {
  CountUp,
  GlassButton,
  GlassCard,
  Pill,
  SPRING,
  Stagger,
  StaggerItem,
} from "./glass";
import { cn } from "@/lib/utils";

/* ============================== HOME ============================== */

export function HomeView({
  d,
  account,
  status,
  seasons,
  activeSeason,
  onPlay,
  seasonDetailBusy,
}: {
  d: Dict;
  account: Account | null;
  status: ServerStatus | null;
  seasons: SeasonMeta[];
  activeSeason: number;
  onPlay: () => void;
  seasonDetailBusy: boolean;
}) {
  const hero =
    seasons.find((s) => s.season === activeSeason) || seasons[seasons.length - 1] || null;
  const others = seasons.filter((s) => s.season !== hero?.season).slice(-8).reverse();

  return (
    <div className="space-y-5">
      {/* hero */}
      <GlassCard className="relative overflow-hidden p-0" hover>
        <div className="relative h-[240px] sm:h-[280px]" style={{ background: hero ? seasonGradient(hero.season) : undefined }}>
          {hero?.image && (
            // eslint-disable-next-line @next/next/no-img-element
            <img
              src={hero.image}
              alt={`${d.chapter} ${hero.chapter} — ${d.seasonWord} ${hero.chapterSeason}`}
              className="absolute inset-0 h-full w-full object-cover opacity-70"
            />
          )}
          <div className="absolute inset-0 bg-gradient-to-t from-[rgba(5,6,11,.92)] via-[rgba(5,6,11,.35)] to-transparent" />
          <div className="owen-glass absolute inset-x-4 bottom-4 top-auto rounded-3xl border-white/15 px-5 py-4 sm:inset-x-6">
            <div className="flex flex-wrap items-center gap-2">
              <Pill tone="accent">
                <Sparkles size={12} />
                {d.activeSeason}
              </Pill>
              {hero?.active && <Pill tone="ok">{d.seasonActive}</Pill>}
              <Pill>{hero?.version}</Pill>
            </div>
            <h1 className="mt-2 text-[26px] font-black leading-tight sm:text-3xl">
              {hero?.name || `${d.chapter} ${hero?.chapter}`}
              <span className="owen-text-grad"> {hero ? `• ${d.seasonWord} ${hero.chapterSeason}` : ""}</span>
            </h1>
            <div className="mt-1 flex items-center gap-2 text-[12.5px] text-[var(--owen-muted)]">
              <Calendar size={13} />
              <span dir="ltr">{hero?.startDate} → {hero?.endDate}</span>
            </div>
            <div className="mt-4 flex flex-wrap items-center gap-3">
              <GlassButton
                shine
                disabled={seasonDetailBusy}
                onClick={onPlay}
                className="h-[52px] px-9 text-base"
              >
                <Play size={19} fill="currentColor" />
                {d.playNow}
              </GlassButton>
              <div className="flex items-center gap-2 text-[12px] text-[var(--owen-muted)]">
                <Trophy size={14} className="text-amber-300" />
                {d.battlePass}: {hero ? `${countTiersLike(hero)} ${d.tiers}` : "—"}
              </div>
            </div>
          </div>
        </div>
      </GlassCard>

      {/* quick stats */}
      <div className="grid grid-cols-2 gap-4 sm:grid-cols-4">
        <StatTile icon={<Zap size={16} />} label={d.level} value={account?.level ?? 0} delay={0} tone="from-violet-500 to-fuchsia-500" />
        <StatTile icon={<Sparkles size={16} />} label={d.vbucks} value={account?.vbucks ?? 0} delay={0.06} tone="from-emerald-400 to-teal-500" />
        <StatTile icon={<Layers size={16} />} label={d.items} value={account?.itemsOwned ?? 0} delay={0.12} tone="from-amber-400 to-orange-500" />
        <StatTile icon={<Server size={16} />} label={d.accountsOnline} value={status?.accounts ?? 0} delay={0.18} tone="from-rose-400 to-pink-500" />
      </div>

      {/* seasons strip */}
      <div>
        <SectionTitle title={d.allSeasons} icon={<Layers size={15} />} />
        <div className="owen-no-scrollbar -mx-1 flex gap-3 overflow-x-auto px-1 pb-2">
          {others.map((s, i) => (
            <SeasonChip key={s.season} s={s} delay={i * 0.04} />
          ))}
        </div>
      </div>
    </div>
  );
}

function countTiersLike(s: SeasonMeta): number {
  return s.paidTiers || s.freeTiers || 100;
}

function StatTile({
  icon,
  label,
  value,
  delay,
  tone,
}: {
  icon: React.ReactNode;
  label: string;
  value: number;
  delay: number;
  tone: string;
}) {
  return (
    <GlassCard delay={delay} hover className="p-4">
      <div className={cn("mb-2 inline-flex rounded-xl bg-gradient-to-br p-2 text-white", tone)}>{icon}</div>
      <div className="text-2xl font-black tabular-nums">
        <CountUp value={value} />
      </div>
      <div className="text-[11.5px] font-semibold text-[var(--owen-muted)]">{label}</div>
    </GlassCard>
  );
}

function SeasonChip({ s, delay }: { s: SeasonMeta; delay: number }) {
  return (
    <motion.div
      initial={{ opacity: 0, y: 18 }}
      animate={{ opacity: 1, y: 0 }}
      transition={{ ...SPRING, delay }}
      whileHover={{ y: -4 }}
      className="owen-glass-soft backdrop-blur-[18px] backdrop-saturate-[1.6] relative h-[76px] w-[130px] flex-none overflow-hidden"
    >
      <div className="absolute inset-0 opacity-45" style={{ background: seasonGradient(s.season) }} />
      {s.image && (
        // eslint-disable-next-line @next/next/no-img-element
        <img src={s.image} alt="" className="absolute inset-0 h-full w-full object-cover opacity-40" />
      )}
      <div className="absolute inset-0 flex flex-col justify-end p-2.5">
        <span className="text-[10px] font-bold text-white/70" dir="ltr">v{s.season}.0</span>
        <span className="truncate text-[12.5px] font-extrabold text-white">
          {s.name || `S${s.season}`}
        </span>
      </div>
    </motion.div>
  );
}

/* ============================== SEASONS ============================== */

export function SeasonsView({
  d,
  seasons,
  activeSeason,
  onSelect,
  refreshKey,
}: {
  d: Dict;
  seasons: SeasonMeta[];
  activeSeason: number;
  onSelect: (n: number) => void;
  refreshKey: number;
}) {
  const [selected, setSelected] = useState<number | null>(null);
  const [detail, setDetail] = useState<Awaited<ReturnType<typeof api.seasonDetail>> | null>(null);
  const level = 42; // display tier gating hint (account level fetched by parent)
  const scroller = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (selected == null) return;
    let cancelled = false;
    api
      .seasonDetail(selected)
      .then((d) => {
        if (!cancelled) setDetail(d);
      })
      .catch(() => {
        if (!cancelled)
          setDetail({
            meta: { season: selected, paidTiers: 0, freeTiers: 0 } as Awaited<
              ReturnType<typeof api.seasonDetail>
            >["meta"],
            battlepass: null,
          });
      });
    return () => {
      cancelled = true;
    };
  }, [selected, refreshKey]);

  /* derived — no setState in the effect body */
  const loadingDetail = selected != null && detail?.meta?.season !== selected;

  const cosmetics = useMemo(() => {
    if (!detail?.battlepass) return [];
    return collectCosmetics(detail.battlepass.paidRewards, level);
  }, [detail, level]);

  return (
    <div className="space-y-5">
      <SectionTitle title={d.allSeasons} icon={<Layers size={15} />} />

      {/* horizontal snap carousel */}
      <div
        ref={scroller}
        className="owen-scroll -mx-1 flex snap-x snap-mandatory gap-4 overflow-x-auto px-1 pb-3"
      >
        {[...seasons].reverse().map((s, i) => {
          const active = s.season === activeSeason;
          return (
            <motion.button
              key={s.season}
              type="button"
              initial={{ opacity: 0, y: 24 }}
              animate={{ opacity: 1, y: 0 }}
              transition={{ ...SPRING, delay: Math.min(i * 0.035, 0.5) }}
              whileHover={{ y: -6 }}
              whileTap={{ scale: 0.97 }}
              onClick={() => setSelected(s.season)}
              className={cn(
                "owen-glass backdrop-blur-[26px] backdrop-saturate-[1.75] relative h-[190px] w-[150px] flex-none snap-center overflow-hidden p-0 text-start",
                selected === s.season && "ring-2 ring-[var(--owen-a)] ring-offset-2 ring-offset-[var(--owen-bg)]",
              )}
            >
              <div className="absolute inset-0 opacity-55" style={{ background: seasonGradient(s.season) }} />
              {s.image && (
                // eslint-disable-next-line @next/next/no-img-element
                <img src={s.image} alt="" className="absolute inset-0 h-full w-full object-cover opacity-45" />
              )}
              <div className="absolute inset-0 bg-gradient-to-t from-[rgba(5,6,11,.9)] to-transparent" />
              {active && (
                <span className="absolute end-2 top-2 z-10">
                  <Pill tone="ok">
                    <Check size={11} />
                    {d.seasonActive}
                  </Pill>
                </span>
              )}
              <div className="absolute inset-x-0 bottom-0 p-3">
                <div className="text-[10.5px] font-bold text-white/65" dir="ltr">
                  {d.chapter} {s.chapter} · {d.seasonWord} {s.chapterSeason}
                </div>
                <div className="truncate text-[15px] font-black text-white">
                  {s.name || `Season ${s.season}`}
                </div>
                <div className="text-[10.5px] text-white/60" dir="ltr">
                  {s.startDate}
                </div>
              </div>
            </motion.button>
          );
        })}
      </div>

      {/* detail panel */}
      <AnimatePresence mode="wait">
        {selected != null && (
          <motion.div
            key={selected}
            initial={{ opacity: 0, y: 26, filter: "blur(6px)" }}
            animate={{ opacity: 1, y: 0, filter: "blur(0px)" }}
            exit={{ opacity: 0, y: -18, filter: "blur(6px)" }}
            transition={SPRING}
          >
            <GlassCard className="p-5">
              {loadingDetail ? (
                <div className="flex h-40 items-center justify-center">
                  <div
                    className="h-10 w-10 rounded-full border-[3px] border-white/10 border-t-[var(--owen-a)]"
                    style={{ animation: "owen-spin .8s linear infinite" }}
                  />
                </div>
              ) : detail ? (
                <div className="space-y-4">
                  <div className="flex flex-wrap items-center justify-between gap-3">
                    <div>
                      <h3 className="text-xl font-black">
                        {detail.meta.name || `Season ${detail.meta.season}`}{" "}
                        <span className="owen-text-grad">v{detail.meta.season}</span>
                      </h3>
                      <p className="text-[12.5px] text-[var(--owen-muted)]" dir="ltr">
                        {detail.meta.startDate} → {detail.meta.endDate}
                      </p>
                    </div>
                    <div className="flex gap-2">
                      <Pill tone="accent">
                        <Gift size={12} /> {d.paidRewards}: {detail.meta.paidTiers}
                      </Pill>
                      <Pill>
                        {d.freeRewards}: {detail.meta.freeTiers}
                      </Pill>
                    </div>
                  </div>

                  {activeSeason !== selected && (
                    <GlassButton
                      variant={activeSeason === selected ? "ghost" : "primary"}
                      onClick={() => onSelect(selected)}
                      shine
                      className="h-11 px-7"
                    >
                      <Check size={16} />
                      {d.setActive}
                    </GlassButton>
                  )}

                  <div>
                    <div className="mb-2.5 flex items-center justify-between">
                      <span className="text-[13px] font-bold text-[var(--owen-muted)]">
                        {d.cosmetics} · {d.yourLevel} {level}
                      </span>
                      <span className="text-[12px] text-[var(--owen-faint)]">
                        {cosmetics.length} {d.items}
                      </span>
                    </div>
                    <div className="owen-scroll grid max-h-[320px] grid-cols-2 gap-2.5 overflow-y-auto pe-1 sm:grid-cols-3 lg:grid-cols-4">
                      {cosmetics.map((c, i) => (
                        <motion.div
                          key={`${c.templateId}-${i}`}
                          initial={{ opacity: 0, scale: 0.9 }}
                          animate={{ opacity: 1, scale: 1 }}
                          transition={{ delay: Math.min(i * 0.015, 0.6), ...SPRING }}
                          className="owen-glass-soft flex items-center gap-2.5 p-2.5"
                        >
                          <div
                            className={cn(
                              "flex h-9 w-9 flex-none rounded-xl bg-gradient-to-br text-white",
                              KIND_COLORS[c.kind] || "from-zinc-500 to-zinc-700",
                            )}
                          >
                            <span className="m-auto text-[10px] font-black">
                              {c.tier}
                            </span>
                          </div>
                          <div className="min-w-0">
                            <div className="truncate text-[11.5px] font-bold" dir="ltr">
                              {c.templateId.split(":").pop()?.replace(/_/g, " ")}
                            </div>
                            <div className="text-[10px] text-[var(--owen-faint)]">{c.kind}</div>
                          </div>
                        </motion.div>
                      ))}
                    </div>
                  </div>
                </div>
              ) : (
                <p className="py-10 text-center text-[var(--owen-muted)]">—</p>
              )}
            </GlassCard>
          </motion.div>
        )}
      </AnimatePresence>
    </div>
  );
}

/* ============================== DOWNLOADS ============================== */

export function DownloadsView({ d, manifests }: { d: Dict; manifests: ManifestEntry[] }) {
  const [filter, setFilter] = useState<string>("all");
  const [jobs, setJobs] = useState<Record<string, number>>({});

  const majors = useMemo(
    () => Array.from(new Set(manifests.map((m) => m.version.split(".")[0]))).sort(),
    [manifests],
  );
  const filtered = useMemo(
    () => (filter === "all" ? manifests : manifests.filter((m) => m.version.startsWith(filter + "."))),
    [manifests, filter],
  );

  useEffect(() => {
    if (Object.keys(jobs).length === 0) return;
    const iv = setInterval(() => {
      setJobs((prev) => {
        const next: Record<string, number> = {};
        let changed = false;
        for (const [k, v] of Object.entries(prev)) {
          if (v >= 100) {
            next[k] = 100;
          } else {
            next[k] = Math.min(100, v + 3 + Math.random() * 9);
            changed = true;
          }
        }
        return changed || Object.values(next).some((v) => v < 100) ? next : prev;
      });
    }, 220);
    return () => clearInterval(iv);
  }, [jobs]);

  return (
    <div className="space-y-5">
      <div>
        <SectionTitle title={d.gameVersions} icon={<DownloadIcon size={15} />} />
        <p className="mt-1 text-[12.5px] text-[var(--owen-muted)]">{d.gameVersionsSub}</p>
      </div>

      <div className="flex flex-wrap gap-2">
        {["all", ...majors].map((m) => (
          <button
            key={m}
            type="button"
            onClick={() => setFilter(m)}
            className={cn(
              "relative rounded-full px-4 py-1.5 text-[12.5px] font-bold transition-colors",
              filter === m ? "text-[#0b0616]" : "owen-glass-soft text-[var(--owen-muted)]",
            )}
          >
            {filter === m && (
              <motion.span
                layoutId="dl-chip"
                transition={SPRING}
                className="owen-accent-grad absolute inset-0 rounded-full"
              />
            )}
            <span className="relative z-10" dir="ltr">
              {m === "all" ? "All" : `v${m}.x`}
            </span>
          </button>
        ))}
      </div>

      <Stagger className="space-y-3" gap={0.05}>
        {filtered.map((m, i) => {
          const key = m.releaseString + "@" + i;
          const progress = jobs[key];
          const done = progress != null && progress >= 100;
          return (
            <StaggerItem key={key}>
              <GlassCard className="p-4" hover>
                <div className="flex flex-wrap items-center gap-4">
                  <div className="owen-accent-grad flex h-12 w-12 flex-none items-center justify-center rounded-2xl text-[15px] font-black text-white shadow-[0_8px_24px_rgba(167,139,250,.4)]" dir="ltr">
                    {m.version}
                  </div>
                  <div className="min-w-0 flex-1">
                    <div className="flex flex-wrap items-center gap-2">
                      <span className="text-[15px] font-extrabold" dir="ltr">
                        {m.label}
                      </span>
                      <Pill tone="ok">
                        <Check size={11} /> {d.verified}
                      </Pill>
                      <span className="text-[11px] text-[var(--owen-faint)]" dir="ltr">
                        {m.date}
                      </span>
                    </div>
                    <button
                      type="button"
                      className="group mt-1 flex max-w-full items-center gap-1.5 text-start"
                      onClick={() => {
                        navigator.clipboard?.writeText(m.manifestHash).then(() => toast(d.copied));
                      }}
                    >
                      <code className="truncate font-mono text-[11px] text-[var(--owen-faint)] transition-colors group-hover:text-[var(--owen-a)]" dir="ltr">
                        {m.manifestHash}
                      </code>
                      <Copy size={11} className="flex-none text-[var(--owen-faint)]" />
                    </button>
                  </div>
                  <GlassButton
                    variant={done ? "ghost" : "primary"}
                    shine={!done && progress == null}
                    className="h-10 flex-none px-5 text-[13px]"
                    onClick={() => !done && setJobs((p) => ({ ...p, [key]: p[key] ?? 0 }))}
                  >
                    {done ? (
                      <>
                        <Check size={14} /> {d.downloadDone}
                      </>
                    ) : progress != null ? (
                      <>
                        <DownloadIcon size={14} className="animate-bounce" /> {d.downloading}
                      </>
                    ) : (
                      <>
                        <DownloadIcon size={14} /> {d.download}
                      </>
                    )}
                  </GlassButton>
                </div>
                <AnimatePresence>
                  {progress != null && !done && (
                    <motion.div
                      initial={{ opacity: 0, height: 0 }}
                      animate={{ opacity: 1, height: "auto" }}
                      exit={{ opacity: 0, height: 0 }}
                      className="overflow-hidden"
                    >
                      <div className="mt-3 h-2 overflow-hidden rounded-full bg-white/10">
                        <motion.div
                          className="owen-accent-grad h-full rounded-full"
                          animate={{ width: `${progress}%` }}
                          transition={{ ease: "linear", duration: 0.2 }}
                        />
                      </div>
                      <div className="mt-1 flex justify-between text-[10.5px] text-[var(--owen-faint)]">
                        <span dir="ltr">{Math.round(progress)}%</span>
                        <span dir="ltr">{(12 + Math.random() * 6).toFixed(1)} MB/s</span>
                      </div>
                    </motion.div>
                  )}
                </AnimatePresence>
              </GlassCard>
            </StaggerItem>
          );
        })}
      </Stagger>
    </div>
  );
}

/* ============================== shared ============================== */

export function SectionTitle({ title, icon }: { title: string; icon?: React.ReactNode }) {
  return (
    <h2 className="flex items-center gap-2 text-[15px] font-extrabold text-[var(--owen-text)]">
      <span className="owen-accent-grad flex h-7 w-7 items-center justify-center rounded-xl text-white">
        {icon}
      </span>
      {title}
    </h2>
  );
}

export type { Lang };
