"use client";

/* Owen Liquid Glass — the authentication view (login + registration). */

import { useMemo, useRef, useState } from "react";
import { AnimatePresence, motion } from "framer-motion";
import { toast } from "sonner";
import {
  ApiError,
  api,
  tokenStore,
  type Account,
} from "@/lib/launcher/api";
import type { Dict, Lang } from "@/lib/launcher/i18n";
import { BootSplash, GlassBanner, GlassButton, GlassCard, GlassInput, LogoMark, SPRING, Wordmark } from "./glass";
import { cn } from "@/lib/utils";

type Mode = "login" | "reg";

export function AuthView({
  d,
  lang,
  onAuthed,
  serverUp,
}: {
  d: Dict;
  lang: Lang;
  onAuthed: (account: Account, token: string) => void;
  serverUp: boolean;
}) {
  const [mode, setMode] = useState<Mode>("login");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const [booting, setBooting] = useState(true);

  // restore session?
  useMemo(() => {
    (async () => {
      const token = tokenStore.get();
      if (token) {
        try {
          await api.verify();
          const me = await api.me();
          tokenStore.cacheMe(me);
          onAuthed(me, token);
          return;
        } catch {
          tokenStore.clear();
        }
      }
      setTimeout(() => setBooting(false), 900);
    })();
    return null;
  }, [onAuthed]);

  const lUser = useRef<HTMLInputElement>(null);
  const lPass = useRef<HTMLInputElement>(null);
  const [rEmail, setREmail] = useState("");
  const [rUser, setRUser] = useState("");
  const [rName, setRName] = useState("");
  const [rPass, setRPass] = useState("");
  const [rPass2, setRPass2] = useState("");

  const friendly = (e: unknown): string => {
    if (e instanceof ApiError) {
      if (e.code === "errors.com.owen.network") return d.errServer;
      if (e.code?.includes("taken")) return d.errTaken;
      if (e.code === "invalid_grant" || e.message === "invalid_grant") return d.errLogin;
      return e.message;
    }
    return d.errServer;
  };

  async function doLogin(ev: React.FormEvent) {
    ev.preventDefault();
    setError("");
    const u = lUser.current?.value.trim() || "";
    const p = lPass.current?.value || "";
    if (!u) return setError(d.vEnterUser);
    if (!p) return setError(d.vEnterPass);
    setBusy(true);
    try {
      const tok = await api.login(u, p);
      tokenStore.set(tok.access_token);
      const me = await api.me();
      tokenStore.cacheMe(me);
      onAuthed(me, tok.access_token);
    } catch (e) {
      setError(friendly(e));
    } finally {
      setBusy(false);
    }
  }

  async function doRegister(ev: React.FormEvent) {
    ev.preventDefault();
    setError("");
    if (!/^[^@\s]+@[^@\s]+\.[^@\s]{2,}$/.test(rEmail)) return setError(d.vEmail);
    if (!/^[a-zA-Z0-9_.]{3,16}$/.test(rUser)) return setError(d.vUser);
    const dn = rName.trim() || rUser;
    if (dn.length < 3 || dn.length > 32) return setError(d.vName);
    if (rPass.length < 8) return setError(d.vWeak);
    if (rPass !== rPass2) return setError(d.vMatch);
    setBusy(true);
    try {
      const res = await api.register({
        email: rEmail,
        username: rUser,
        displayName: dn,
        password: rPass,
      });
      if (res.access_token) {
        tokenStore.set(res.access_token);
        const me = await api.me();
        tokenStore.cacheMe(me);
        onAuthed(me, res.access_token);
      }
    } catch (e) {
      setError(friendly(e));
    } finally {
      setBusy(false);
    }
  }

  if (booting) return <BootSplash label={d.loading} />;

  return (
    <div className="relative z-10 flex min-h-dvh items-center justify-center p-5">
      <GlassCard className="w-full max-w-[430px] px-8 pb-7 pt-9" delay={0}>
        <div className="mb-7 flex items-center gap-4">
          <LogoMark />
          <div>
            <Wordmark brand={d.brand} launcher={d.launcher} />
            <p className="mt-1.5 text-[12.5px] text-[var(--owen-muted)]">{d.tagline}</p>
          </div>
        </div>

        {/* tabs */}
        <div className="relative mb-6 flex rounded-full border border-[var(--owen-line-soft)] bg-white/5 p-1">
          {(["login", "reg"] as Mode[]).map((m) => (
            <button
              key={m}
              type="button"
              onClick={() => {
                setMode(m);
                setError("");
              }}
              className={cn(
                "relative z-10 flex-1 rounded-full py-2.5 text-[13.5px] font-bold transition-colors",
                mode === m ? "text-[#0b0616]" : "text-[var(--owen-muted)]",
              )}
            >
              {m === "login" ? d.login : d.createAccount}
              {mode === m && (
                <motion.span
                  layoutId="auth-pill"
                  transition={SPRING}
                  className="owen-accent-grad absolute inset-0 -z-10 rounded-full shadow-[0_6px_22px_rgba(167,139,250,.5)]"
                />
              )}
            </button>
          ))}
        </div>

        <AnimatePresence>
          {error && (
            <motion.div key="err" initial={{ opacity: 0 }} animate={{ opacity: 1 }} exit={{ opacity: 0 }}>
              <GlassBanner kind="err">{error}</GlassBanner>
            </motion.div>
          )}
          {!serverUp && !error && (
            <motion.div key="off" initial={{ opacity: 0 }} animate={{ opacity: 1 }}>
              <GlassBanner kind="err">{d.errServer}</GlassBanner>
            </motion.div>
          )}
        </AnimatePresence>

        <AnimatePresence mode="wait">
          {mode === "login" ? (
            <motion.form
              key="login"
              initial={{ opacity: 0, x: lang === "ar" ? -26 : 26, filter: "blur(4px)" }}
              animate={{ opacity: 1, x: 0, filter: "blur(0px)" }}
              exit={{ opacity: 0, x: lang === "ar" ? 26 : -26, filter: "blur(4px)" }}
              transition={SPRING}
              onSubmit={doLogin}
              noValidate
            >
              <GlassInput
                ref={lUser}
                label={d.usernameOrEmail}
                type="text"
                autoComplete="username"
                autoCapitalize="none"
                spellCheck={false}
                dir="ltr"
              />
              <GlassInput
                ref={lPass}
                label={d.password}
                type="password"
                autoComplete="current-password"
                dir="ltr"
              />
              <GlassButton type="submit" shine disabled={busy} className="mt-2 h-[54px] w-full text-base">
                {busy ? "…" : d.signIn}
              </GlassButton>
              <p className="mt-4 text-center text-[13px] text-[var(--owen-faint)]">
                {d.noAccount}{" "}
                <button
                  type="button"
                  className="font-bold text-[var(--owen-a)] transition-colors hover:text-[var(--owen-b)]"
                  onClick={() => setMode("reg")}
                >
                  {d.createAccount}
                </button>
              </p>
            </motion.form>
          ) : (
            <motion.form
              key="reg"
              initial={{ opacity: 0, x: lang === "ar" ? -26 : 26, filter: "blur(4px)" }}
              animate={{ opacity: 1, x: 0, filter: "blur(0px)" }}
              exit={{ opacity: 0, x: lang === "ar" ? 26 : -26, filter: "blur(4px)" }}
              transition={SPRING}
              onSubmit={doRegister}
              noValidate
            >
              <GlassInput
                label={d.email}
                type="email"
                autoComplete="email"
                dir="ltr"
                value={rEmail}
                onChange={(e) => setREmail(e.target.value)}
              />
              <GlassInput
                label={d.username}
                type="text"
                autoComplete="username"
                autoCapitalize="none"
                spellCheck={false}
                dir="ltr"
                value={rUser}
                onChange={(e) => setRUser(e.target.value)}
              />
              <GlassInput
                label={d.displayName}
                type="text"
                value={rName}
                onChange={(e) => setRName(e.target.value)}
              />
              <GlassInput
                label={d.password}
                type="password"
                autoComplete="new-password"
                dir="ltr"
                value={rPass}
                onChange={(e) => setRPass(e.target.value)}
              />
              <GlassInput
                label={d.confirmPassword}
                type="password"
                autoComplete="new-password"
                dir="ltr"
                value={rPass2}
                onChange={(e) => setRPass2(e.target.value)}
              />
              <p className="-mt-1 mb-3 px-1 text-[11.5px] leading-relaxed text-[var(--owen-faint)]">
                {d.passwordHint}
              </p>
              <GlassButton type="submit" shine disabled={busy} className="h-[54px] w-full text-base">
                {busy ? "…" : d.register}
              </GlassButton>
              <p className="mt-4 text-center text-[13px] text-[var(--owen-faint)]">
                {d.haveAccount}{" "}
                <button
                  type="button"
                  className="font-bold text-[var(--owen-a)] transition-colors hover:text-[var(--owen-b)]"
                  onClick={() => setMode("login")}
                >
                  {d.login}
                </button>
              </p>
            </motion.form>
          )}
        </AnimatePresence>
      </GlassCard>
      <span className="sr-only" aria-live="polite">
        {busy ? d.loading : ""}
      </span>
      <ToastShim />
    </div>
  );
}

function ToastShim() {
  // keeps the sonner dependency referenced for future use without extra UI
  return null;
}
void toast;
