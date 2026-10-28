"use client";

/* Owen Liquid Glass — the design primitives. */

import { motion, useInView, useMotionValue, useSpring, useTransform } from "framer-motion";
import { useEffect, useRef, type ReactNode, type InputHTMLAttributes } from "react";
import { cn } from "@/lib/utils";

export const SPRING = { type: "spring" as const, stiffness: 260, damping: 26 };
export const SOFT_SPRING = { type: "spring" as const, stiffness: 180, damping: 24 };

/* ---------------------------------- aurora ---------------------------------- */

export function AuroraBackground() {
  return (
    <div aria-hidden className="pointer-events-none fixed -inset-[20%] z-0 opacity-85 blur-[90px] saturate-125">
      <div
        className="blob absolute -left-[8%] -top-[12%] h-[44vmax] w-[44vmax] rounded-full mix-blend-screen"
        style={{
          background: "radial-gradient(circle at 35% 35%, rgba(139,92,246,.5), transparent 60%)",
          animation: "owen-drift1 26s ease-in-out infinite alternate",
        }}
      />
      <div
        className="absolute -right-[10%] top-[6%] h-[38vmax] w-[38vmax] rounded-full mix-blend-screen"
        style={{
          background: "radial-gradient(circle at 60% 40%, rgba(236,72,153,.4), transparent 60%)",
          animation: "owen-drift2 31s ease-in-out infinite alternate",
        }}
      />
      <div
        className="absolute -bottom-[16%] left-[16%] h-[34vmax] w-[34vmax] rounded-full mix-blend-screen"
        style={{
          background: "radial-gradient(circle at 50% 50%, rgba(45,212,191,.32), transparent 60%)",
          animation: "owen-drift3 37s ease-in-out infinite alternate",
        }}
      />
      <div
        className="absolute -bottom-[6%] right-[12%] h-[26vmax] w-[26vmax] rounded-full mix-blend-screen"
        style={{
          background: "radial-gradient(circle at 50% 50%, rgba(245,158,11,.22), transparent 60%)",
          animation: "owen-drift1 29s ease-in-out infinite alternate-reverse",
        }}
      />
    </div>
  );
}

/* ---------------------------------- logo ---------------------------------- */

export function LogoMark({ size = 52, className }: { size?: number; className?: string }) {
  return (
    <div
      className={cn("owen-accent-grad flex flex-none items-center justify-center", className)}
      style={{
        width: size,
        height: size,
        borderRadius: size * 0.33,
        boxShadow: "0 12px 34px rgba(167,139,250,.45), inset 0 1px 0 rgba(255,255,255,.5)",
        animation: "owen-pulse-glow 3.2s ease-in-out infinite",
      }}
    >
      <svg viewBox="0 0 24 24" fill="none" style={{ width: size * 0.58, height: size * 0.58 }}>
        <path
          d="M12 2L21 8.2 17.4 21H6.6L3 8.2 12 2Z"
          stroke="#fff"
          strokeWidth="1.7"
          strokeLinejoin="round"
        />
        <path
          d="M12 2v19M3.3 8.6 12 12l8.7-3.4M6.6 21 12 12l5.4 9"
          stroke="#fff"
          strokeWidth="1"
          opacity=".75"
        />
      </svg>
    </div>
  );
}

export function Wordmark({ brand, launcher }: { brand: string; launcher: string }) {
  return (
    <div className="leading-[1.08]">
      <b className="block text-2xl font-extrabold tracking-wide">{brand}</b>
      <span className="owen-text-grad text-[11.5px] font-bold tracking-[3.5px]">{launcher}</span>
    </div>
  );
}

/* ---------------------------------- cards ---------------------------------- */

export function GlassCard({
  children,
  className,
  hover = false,
  delay = 0,
  ...rest
}: {
  children: ReactNode;
  className?: string;
  hover?: boolean;
  delay?: number;
} & React.ComponentProps<typeof motion.div>) {
  return (
    <motion.div
      initial={{ opacity: 0, y: 24, filter: "blur(6px)" }}
      animate={{ opacity: 1, y: 0, filter: "blur(0px)" }}
      transition={{ ...SOFT_SPRING, delay }}
      whileHover={hover ? { y: -5, transition: SPRING } : undefined}
      whileTap={hover ? { scale: 0.98 } : undefined}
      className={cn(
        "owen-glass backdrop-blur-[26px] backdrop-saturate-[1.75]",
        className,
      )}
      {...rest}
    >
      {children}
    </motion.div>
  );
}

/* ---------------------------------- buttons ---------------------------------- */

type BtnVariant = "primary" | "ghost" | "danger";

export function GlassButton({
  children,
  variant = "primary",
  className,
  shine = false,
  ...rest
}: {
  children: ReactNode;
  variant?: BtnVariant;
  shine?: boolean;
  className?: string;
} & React.ComponentProps<typeof motion.button>) {
  const base =
    "relative inline-flex items-center justify-center gap-2 rounded-full font-bold outline-none transition-colors disabled:cursor-default disabled:opacity-55";
  const styles: Record<BtnVariant, string> = {
    primary:
      "owen-accent-grad text-white shadow-[0_14px_42px_rgba(167,139,250,.45),inset_0_1px_0_rgba(255,255,255,.4)]",
    ghost:
      "owen-glass-soft backdrop-blur-[18px] backdrop-saturate-[1.6] text-[var(--owen-text)] hover:border-white/25",
    danger: "bg-rose-500/15 border border-rose-400/40 text-rose-300",
  };
  return (
    <motion.button
      whileHover={{ y: -2 }}
      whileTap={{ scale: 0.96 }}
      transition={SPRING}
      className={cn(base, styles[variant], shine && "owen-shine", className)}
      {...rest}
    >
      {children}
    </motion.button>
  );
}

/* ---------------------------------- inputs ---------------------------------- */

export function GlassInput({
  label,
  dir,
  ...rest
}: { label: string } & InputHTMLAttributes<HTMLInputElement>) {
  return (
    <div className="relative mb-3.5">
      <input
        {...rest}
        dir={dir}
        placeholder=" "
        className="peer w-full rounded-2xl border border-[var(--owen-line-soft)] bg-white/5 px-4 py-3.5 text-[15px] text-[var(--owen-text)] caret-[var(--owen-a)] outline-none transition-all focus:border-[rgba(167,139,250,.65)] focus:bg-white/[.08] focus:shadow-[0_0_0_4px_rgba(167,139,250,.14)]"
      />
      <label className="pointer-events-none absolute start-4 top-3.5 text-[15px] text-[var(--owen-faint)] transition-all peer-focus:-top-2 peer-focus:rounded-md peer-focus:bg-[#0b0d16] peer-focus:px-1.5 peer-focus:text-[11px] peer-focus:font-bold peer-focus:text-[var(--owen-a)] peer-[:not(:placeholder-shown)]:-top-2 peer-[:not(:placeholder-shown)]:rounded-md peer-[:not(:placeholder-shown)]:bg-[#0b0d16] peer-[:not(:placeholder-shown)]:px-1.5 peer-[:not(:placeholder-shown)]:text-[11px] peer-[:not(:placeholder-shown)]:font-bold peer-[:not(:placeholder-shown)]:text-[var(--owen-a)]">
        {label}
      </label>
    </div>
  );
}

/* ---------------------------------- pills ---------------------------------- */

export function Pill({
  children,
  tone = "neutral",
  className,
}: {
  children: ReactNode;
  tone?: "neutral" | "ok" | "err" | "accent";
  className?: string;
}) {
  const tones = {
    neutral: "bg-white/8 border-white/15 text-[var(--owen-muted)]",
    ok: "bg-emerald-400/12 border-emerald-300/35 text-emerald-300",
    err: "bg-rose-400/12 border-rose-300/35 text-rose-300",
    accent: "bg-[rgba(167,139,250,.14)] border-[rgba(167,139,250,.4)] text-[var(--owen-a)]",
  } as const;
  return (
    <span
      className={cn(
        "inline-flex items-center gap-1.5 rounded-full border px-3 py-1 text-[11px] font-bold leading-5",
        tones[tone],
        className,
      )}
    >
      {children}
    </span>
  );
}

export function StatusDot({ ok, className }: { ok: boolean; className?: string }) {
  return (
    <span className={cn("relative flex h-2.5 w-2.5", className)}>
      {ok && (
        <span className="absolute inline-flex h-full w-full animate-ping rounded-full bg-emerald-400 opacity-60" />
      )}
      <span
        className={cn(
          "relative inline-flex h-2.5 w-2.5 rounded-full",
          ok ? "bg-emerald-400" : "bg-rose-400",
        )}
      />
    </span>
  );
}

/* ---------------------------------- counters ---------------------------------- */

export function CountUp({
  value,
  className,
}: {
  value: number;
  className?: string;
}) {
  const ref = useRef<HTMLSpanElement>(null);
  const inView = useInView(ref, { once: true });
  const mv = useMotionValue(0);
  const spring = useSpring(mv, { stiffness: 90, damping: 24 });
  const rounded = useTransform(spring, (v) => Math.round(v).toLocaleString("en"));
  useEffect(() => {
    if (inView) mv.set(value);
  }, [inView, value, mv]);
  return (
    <span ref={ref} className={className}>
      <motion.span>{rounded}</motion.span>
    </span>
  );
}

/* ---------------------------------- ring ---------------------------------- */

export function ProgressRing({
  progress,
  size = 92,
  stroke = 8,
  children,
}: {
  progress: number; // 0..1
  size?: number;
  stroke?: number;
  children?: ReactNode;
}) {
  const r = (size - stroke) / 2;
  const c = 2 * Math.PI * r;
  return (
    <div className="relative" style={{ width: size, height: size }}>
      <svg width={size} height={size} className="-rotate-90">
        <circle
          cx={size / 2}
          cy={size / 2}
          r={r}
          fill="none"
          stroke="rgba(255,255,255,.09)"
          strokeWidth={stroke}
        />
        <motion.circle
          cx={size / 2}
          cy={size / 2}
          r={r}
          fill="none"
          stroke="url(#owen-grad)"
          strokeWidth={stroke}
          strokeLinecap="round"
          strokeDasharray={c}
          initial={{ strokeDashoffset: c }}
          animate={{ strokeDashoffset: c * (1 - Math.min(1, Math.max(0, progress))) }}
          transition={{ ...SOFT_SPRING, delay: 0.2 }}
        />
        <defs>
          <linearGradient id="owen-grad" x1="0%" y1="0%" x2="100%" y2="100%">
            <stop offset="0%" stopColor="#a78bfa" />
            <stop offset="100%" stopColor="#f472b6" />
          </linearGradient>
        </defs>
      </svg>
      <div className="absolute inset-0 flex flex-col items-center justify-center">{children}</div>
    </div>
  );
}

/* ---------------------------------- banner ---------------------------------- */

export function GlassBanner({
  kind,
  children,
}: {
  kind: "err" | "ok";
  children: ReactNode;
}) {
  return (
    <motion.div
      initial={{ opacity: 0, x: 0 }}
      animate={{ opacity: 1, x: [0, -7, 6, -3, 0] }}
      transition={{ duration: 0.5 }}
      className={cn(
        "mb-3.5 rounded-2xl border px-4 py-3 text-[13.5px] leading-relaxed backdrop-blur-xl",
        kind === "err"
          ? "border-rose-300/40 bg-rose-400/10 text-rose-300"
          : "border-emerald-300/40 bg-emerald-400/10 text-emerald-300",
      )}
      role={kind === "err" ? "alert" : "status"}
    >
      {children}
    </motion.div>
  );
}

/* ---------------------------------- boot splash ---------------------------------- */

export function BootSplash({ label }: { label: string }) {
  return (
    <motion.div
      exit={{ opacity: 0, scale: 1.06, filter: "blur(10px)" }}
      transition={{ duration: 0.5, ease: [0.34, 1.56, 0.64, 1] }}
      className="fixed inset-0 z-50 flex flex-col items-center justify-center gap-8"
    >
      <motion.div
        initial={{ scale: 0.4, opacity: 0, filter: "blur(14px)" }}
        animate={{ scale: 1, opacity: 1, filter: "blur(0px)" }}
        transition={{ ...SPRING, delay: 0.1 }}
      >
        <LogoMark size={96} />
      </motion.div>
      <motion.div
        initial={{ opacity: 0, y: 16 }}
        animate={{ opacity: 1, y: 0 }}
        transition={{ delay: 0.35 }}
        className="text-center"
      >
        <div className="text-3xl font-black tracking-wide">OWEN</div>
        <div className="owen-text-grad mt-1 text-xs font-bold tracking-[6px]">LAUNCHER</div>
      </motion.div>
      <motion.div
        initial={{ opacity: 0 }}
        animate={{ opacity: 1 }}
        transition={{ delay: 0.6 }}
        className="flex items-center gap-3"
      >
        <div className="h-[3px] w-40 overflow-hidden rounded-full bg-white/10">
          <motion.div
            className="owen-accent-grad h-full rounded-full"
            initial={{ width: "8%" }}
            animate={{ width: ["8%", "82%", "100%"] }}
            transition={{ duration: 1.5, ease: "easeInOut" }}
          />
        </div>
        <span className="text-xs text-[var(--owen-faint)]">{label}</span>
      </motion.div>
    </motion.div>
  );
}

/* ---------------------------------- stagger ---------------------------------- */

export function Stagger({
  children,
  delay = 0,
  gap = 0.06,
  className,
}: {
  children: ReactNode;
  delay?: number;
  gap?: number;
  className?: string;
}) {
  return (
    <motion.div
      className={className}
      initial="hidden"
      animate="show"
      variants={{
        hidden: {},
        show: { transition: { staggerChildren: gap, delayChildren: delay } },
      }}
    >
      {children}
    </motion.div>
  );
}

export function StaggerItem({ children, className }: { children: ReactNode; className?: string }) {
  return (
    <motion.div
      className={className}
      variants={{
        hidden: { opacity: 0, y: 22, filter: "blur(5px)" },
        show: { opacity: 1, y: 0, filter: "blur(0px)", transition: SOFT_SPRING },
      }}
    >
      {children}
    </motion.div>
  );
}
