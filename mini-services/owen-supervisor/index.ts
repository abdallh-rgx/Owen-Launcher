/*
 * owen-supervisor — on-demand supervisor + verbatim reverse proxy for the
 * real Owen Rust backend (owen-launcher-rs/target/release/owen-server).
 *
 * WHY
 * ---
 * The Next.js app proxies `/owen-api/*` and `/partyhub/*` to
 * http://127.0.0.1:3010 (next.config.ts). This service owns :3010 and
 * keeps the heavy Rust server OFF until the first request actually needs
 * it (lazy hosting), then proxies every byte verbatim to 127.0.0.1:3011.
 *
 * ROUTES
 *   GET  /api/launcher/hosting/status   supervisor-owned: {"running", "detail"}
 *   POST /api/launcher/hosting/start    spawn + wait for health (<=10s)
 *   POST /api/launcher/hosting/stop     SIGTERM (<=3s) then SIGKILL
 *   GET  /api/launcher/status           lazy-start + 200 JSON (synthesized
 *                                       here: the Rust backend has no such
 *                                       route — it 404s `no_route` — so the
 *                                       supervisor answers it by merging the
 *                                       backend's `/` index payload)
 *   *    everything else                lazy-start if needed, then proxy
 *                                       method+path+query+body verbatim.
 *
 * RULES honored: port 3010/3011 are HARDCODED (no PORT env); all fetches to
 * the backend are server-side absolute http://127.0.0.1:3011 (allowed for a
 * server-side service); stdout logging for `bun --hot` keep-alive.
 */

import { existsSync } from "node:fs";

/* ------------------------------------------------------------------ *
 * configuration (hardcoded — never from env)                          *
 * ------------------------------------------------------------------ */

const SUPERVISOR_PORT = 3010; // hardcoded by contract (next.config.ts)
const BACKEND_PORT = 3011; // hardcoded by contract
const BACKEND_ORIGIN = `http://127.0.0.1:${BACKEND_PORT}`;

const BACKEND_BIN = "/home/z/my-project/owen-launcher-rs/target/release/owen-server";
const BACKEND_CWD = "/home/z/my-project/owen-launcher-rs";
const BACKEND_DATA_DIR = "/home/z/my-project/owen-launcher-rs/data/owen";

/* The Rust server's ServerOptions (crates/owen-server/src/lib.rs) supports
 * `--partyhub-dir` for the static Kairos bundle root (served under
 * /partyhub/*, SPA fallback offline/entry/index.html). Pass it only when a
 * sensible bundle actually exists on disk. */
const PARTYHUB_DIR = "/home/z/my-project/download/shippedweb_party";

/* Health probe: the backend's `/` index route returns 200 JSON as soon as
 * it is listening. NOTE the backend has no /api/launcher/status route (its
 * fallback 404s), so the supervisor probes `/` and answers
 * /api/launcher/status itself. */
const HEALTH_PATH = "/";
const HEALTH_TIMEOUT_MS = 10_000;
const HEALTH_POLL_MS = 250;
const HEALTH_FETCH_TIMEOUT_MS = 1_500;
const STOP_GRACE_MS = 3_000;

function backendArgs(): string[] {
  const args = [
    "--port",
    String(BACKEND_PORT),
    "--bind",
    "127.0.0.1",
    "--data-dir",
    BACKEND_DATA_DIR,
  ];
  if (existsSync(PARTYHUB_DIR)) args.push("--partyhub-dir", PARTYHUB_DIR);
  return args;
}

/* ------------------------------------------------------------------ *
 * state (on globalThis so `bun --hot` reloads keep the child alive)   *
 * ------------------------------------------------------------------ */

type BackendChild = Bun.Subprocess<"ignore", "pipe", "pipe">;

type SupState = {
  child: BackendChild | null;
  starting: Promise<boolean> | null;
};

const g = globalThis as typeof globalThis & { __owenSupervisor?: SupState };
const S: SupState = (g.__owenSupervisor ??= { child: null, starting: null });

function log(msg: string) {
  console.log(`[owen-supervisor ${new Date().toISOString()}] ${msg}`);
}

/* ------------------------------------------------------------------ *
 * child process management                                            *
 * ------------------------------------------------------------------ */

function isRunning(): boolean {
  return !!S.child && S.child.exitCode === null && S.child.signalCode === null;
}

function childDetail(): string {
  return isRunning()
    ? `127.0.0.1:${BACKEND_PORT} · pid ${S.child!.pid}`
    : "stopped";
}

function spawnBackend(): BackendChild {
  const args = backendArgs();
  log(`spawning owen-server: ${BACKEND_BIN} ${args.join(" ")} (cwd ${BACKEND_CWD})`);
  const child = Bun.spawn({
    cmd: [BACKEND_BIN, ...args],
    cwd: BACKEND_CWD,
    stdin: "ignore",
    stdout: "pipe",
    stderr: "pipe",
    /* minimal env: no PORT-ish variables can leak in */
    env: {
      PATH: process.env.PATH ?? "/usr/local/bin:/usr/bin:/bin",
      HOME: process.env.HOME ?? "/home/z",
    },
  });

  /* stream backend stdout/stderr into our log */
  const pump = async (stream: ReadableStream<Uint8Array> | "pipe", tag: string) => {
    if (typeof stream === "string") return;
    try {
      for await (const chunk of stream) {
        for (const line of new TextDecoder().decode(chunk).split("\n")) {
          if (line.trim()) log(`[owen-server:${tag}] ${line}`);
        }
      }
    } catch {
      /* stream closed */
    }
  };
  void pump(child.stdout, "out");
  void pump(child.stderr, "err");

  /* unexpected-death watcher: mark stopped, next request respawns */
  void (async () => {
    await child.exited;
    if (S.child === child) {
      S.child = null;
      const why = child.signalCode ? `signal=${child.signalCode}` : `code=${child.exitCode}`;
      log(`owen-server pid ${child.pid} exited (${why}) — marked stopped; next request respawns`);
    }
  })();

  return child;
}

async function backendHealthy(): Promise<boolean> {
  try {
    const res = await fetch(`${BACKEND_ORIGIN}${HEALTH_PATH}`, {
      method: "GET",
      cache: "no-store",
      signal: AbortSignal.timeout(HEALTH_FETCH_TIMEOUT_MS),
    });
    return res.status >= 200 && res.status < 300;
  } catch {
    return false;
  }
}

/* Per-request guard: concurrent first requests share one spawn+health-wait.
 * NOTE: joining an in-flight `S.starting` must come BEFORE the isRunning()
 * fast path — the process is alive during boot but not yet listening, so
 * latecomers must wait for the health poll too. */
async function ensureRunning(): Promise<boolean> {
  if (S.starting) return S.starting; /* a spawn+health wait is in flight → join it */
  if (isRunning()) return true; /* already up → proxy immediately, no waiting */

  /* single-threaded JS: S.starting is null here (checked above) */
  S.starting = (async () => {
    const t0 = Date.now();
    const child = spawnBackend();
    S.child = child;
    const deadline = t0 + HEALTH_TIMEOUT_MS;
    while (Date.now() < deadline) {
      if (child.exitCode !== null || child.signalCode !== null) break; // died during boot
      if (await backendHealthy()) {
        log(`owen-server healthy after ${Date.now() - t0}ms (pid ${child.pid})`);
        return true;
      }
      await Bun.sleep(HEALTH_POLL_MS);
    }
    log(`owen-server did NOT become healthy within ${HEALTH_TIMEOUT_MS}ms`);
    return false;
  })().finally(() => {
    S.starting = null;
  });
  return S.starting;
}

async function stopBackend(): Promise<void> {
  const child = S.child;
  S.child = null;
  if (!child || child.exitCode !== null || child.signalCode !== null) return;
  log(`stopping owen-server pid ${child.pid} (SIGTERM, grace ${STOP_GRACE_MS}ms)`);
  child.kill("SIGTERM");
  const deadline = Date.now() + STOP_GRACE_MS;
  while ((child.exitCode === null && child.signalCode === null) && Date.now() < deadline) {
    await Bun.sleep(100);
  }
  if (child.exitCode === null && child.signalCode === null) {
    log(`owen-server pid ${child.pid} ignored SIGTERM — SIGKILL`);
    child.kill("SIGKILL");
    await child.exited.catch(() => {});
  }
  log("owen-server stopped");
}

/* If a previous supervisor was SIGKILLed its child may still own :3011 —
 * reap orphans at boot so a fresh spawn can bind. */
function backendOrphanPids(): number[] {
  try {
    const r = Bun.spawnSync(["pgrep", "-f", `owen-server .*--port ${BACKEND_PORT}`]);
    return r.stdout
      .toString()
      .split("\n")
      .map((s) => Number(s.trim()))
      .filter((n) => Number.isInteger(n) && n > 0 && n !== process.pid);
  } catch {
    return [];
  }
}

async function reapOrphans() {
  if (S.child) return; /* hot reload — child handle survived */
  if (!(await backendHealthy())) return; /* port free */
  const pids = backendOrphanPids();
  if (!pids.length) return;
  log(`orphan owen-server on :${BACKEND_PORT} (pids ${pids.join(",")}) — reaping`);
  for (const pid of pids) {
    try {
      process.kill(pid, "SIGTERM");
    } catch {}
  }
  const deadline = Date.now() + STOP_GRACE_MS;
  while (Date.now() < deadline && (await backendHealthy())) await Bun.sleep(150);
  if (await backendHealthy()) {
    for (const pid of pids) {
      try {
        process.kill(pid, "SIGKILL");
      } catch {}
    }
    await Bun.sleep(250);
  }
  log("orphans reaped");
}

/* ------------------------------------------------------------------ *
 * proxy                                                               *
 * ------------------------------------------------------------------ */

const HOP_BY_HOP = new Set([
  "host",
  "connection",
  "keep-alive",
  "transfer-encoding",
  "upgrade",
  "proxy-authorization",
  "proxy-authenticate",
  "te",
  "trailer",
  "content-length", // recomputed by the proxy
]);

async function proxyVerbatim(req: Request): Promise<Response> {
  const url = new URL(req.url);
  const target = `${BACKEND_ORIGIN}${url.pathname}${url.search}`;
  const headers = new Headers();
  req.headers.forEach((v, k) => {
    if (!HOP_BY_HOP.has(k.toLowerCase())) headers.set(k, v);
  });
  const hasBody = !(req.method === "GET" || req.method === "HEAD");
  const body = hasBody ? await req.arrayBuffer() : undefined;
  const upstream = await fetch(target, {
    method: req.method,
    headers,
    body,
    redirect: "manual", // pass 3xx (e.g. /partyhub → /partyhub/offline/en-US/) through
  });
  const outHeaders = new Headers();
  upstream.headers.forEach((v, k) => {
    if (!HOP_BY_HOP.has(k.toLowerCase())) outHeaders.set(k, v);
  });
  return new Response(upstream.body, { status: upstream.status, headers: outHeaders });
}

/* ------------------------------------------------------------------ *
 * responses                                                           *
 * ------------------------------------------------------------------ */

const JSON_HEADERS = { "content-type": "application/json", "cache-control": "no-store" };

function json(status: number, body: unknown, extra?: HeadersInit): Response {
  return new Response(JSON.stringify(body), { status, headers: { ...JSON_HEADERS, ...extra } });
}

function notHealthy503(): Response {
  return json(503, {
    running: false,
    errorCode: "errors.com.owen.server.stopped",
    message: "Owen Server did not come up",
  });
}

/* Supervisor-synthesized /api/launcher/status: lazy-starts the backend and
 * merges the backend's `/` index payload with hosting info. */
async function launcherStatus(): Promise<Response> {
  const t0 = Date.now();
  const ok = await ensureRunning();
  const took = Date.now() - t0;
  if (!ok) return notHealthy503();
  try {
    const res = await fetch(`${BACKEND_ORIGIN}/`, {
      headers: { accept: "application/json" },
      cache: "no-store",
    });
    let backend: Record<string, unknown> = {};
    try {
      backend = (await res.json()) as Record<string, unknown>;
    } catch {
      /* keep {} */
    }
    return json(
      200,
      { running: true, detail: childDetail(), ...backend },
      { "x-owen-autostart-ms": String(took) },
    );
  } catch {
    return notHealthy503();
  }
}

/* ------------------------------------------------------------------ *
 * server                                                              *
 * ------------------------------------------------------------------ */

const server = Bun.serve({
  port: SUPERVISOR_PORT, // hardcoded — never env PORT
  async fetch(req): Promise<Response> {
    const url = new URL(req.url);
    const pathname = url.pathname;
    const method = req.method;

    /* ---- hosting API (supervisor-owned, never proxied) ---- */
    if (pathname === "/api/launcher/hosting/status" && method === "GET") {
      return json(200, { running: isRunning(), detail: childDetail() });
    }
    if (pathname === "/api/launcher/hosting/start" && method === "POST") {
      const ok = await ensureRunning();
      if (!ok) {
        return json(504, {
          running: false,
          error: `owen-server did not become healthy within ${HEALTH_TIMEOUT_MS}ms`,
        });
      }
      return json(200, { running: true, detail: childDetail() });
    }
    if (pathname === "/api/launcher/hosting/stop" && method === "POST") {
      await stopBackend();
      return json(200, { running: false, detail: "stopped" });
    }

    /* ---- synthesized backend status (backend itself has no route) ---- */
    if (pathname === "/api/launcher/status" && method === "GET") {
      return launcherStatus();
    }

    /* ---- everything else: LAZY auto-start + verbatim proxy ---- */
    const ok = await ensureRunning();
    if (!ok) return notHealthy503();
    try {
      return await proxyVerbatim(req);
    } catch (e) {
      /* child died mid-request (or connect refused) — mark stopped, 503 */
      if (isRunning()) {
        log(`proxy error for ${method} ${pathname}: ${e}`);
      } else {
        log(`backend died while proxying ${method} ${pathname} — 503`);
      }
      return notHealthy503();
    }
  },
});

log(`owen-supervisor listening on http://127.0.0.1:${server.port} → ${BACKEND_ORIGIN} (lazy)`);
void reapOrphans();
