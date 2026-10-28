/*
 * partyhub-web — serves the patched Fortnite "shippedweb" PartyHub web and
 * bridges its raw Kairos GraphQL dialect to a Voltronite backend.
 *
 * WHY AN ADAPTER
 * --------------
 * The Kairos web bundle POSTs raw graphql documents:
 *     {"query":"query { Account { myAccount { ... } }, Fortnite { ... } }", "variables":{}}
 * with NO operationName and NO Authorization header (in the real app the
 * native layer owns auth). Voltronite's /partyhub/graphql switches on
 * body.operationName and requires a Bearer token. This service:
 *   1. receives the raw document same-origin on any SPA route (./graphql),
 *   2. fingerprints the query text and maps it to a Voltronite operation,
 *   3. obtains a real access token from Voltronite's oauth endpoint
 *      (grant_type=password, cached, refreshed on 401),
 *   4. forwards {operationName, query, variables} + Authorization to
 *      /partyhub/graphql  ->  the backend receives and answers the request,
 *   5. applies the tiny compatibility transforms the old client needs
 *      (kairos lightswitch UP, alias reshaping) and returns {data: ...}.
 * Operations Voltronite does not implement (friends lists, eula body,
 * character profile, friend mutations) are answered locally with neutral
 * defaults so the PartyHub UI renders cleanly.
 *
 * ROUTING (when served behind the Owen web console at /partyhub/...)
 *   GET  /*                    static file from the patched copy
 *   GET  /offline/:locale/...  SPA fallback -> index.html
 *   POST <any path ending in /graphql>  -> the adapter above
 */

import { serve } from "bun";
import path from "node:path";
import { existsSync, readFileSync } from "node:fs";

const ROOT = process.env.OWEN_WEB_ROOT ?? "/home/z/my-project/download/shippedweb_party";
const ENTRY = path.join(ROOT, "offline", "entry");
const PORT = Number(process.env.PORT ?? 8091);

/* Voltronite backend (unmodified). */
const VOLTRONITE = process.env.VOLTRONITE_URL ?? "http://127.0.0.1:3551";
const OWEN_USER = process.env.OWEN_USER ?? "OwenTester";
const OWEN_ACCOUNT_ID = Buffer.from(OWEN_USER, "utf8").toString("hex");

const MIME: Record<string, string> = {
  ".html": "text/html; charset=utf-8",
  ".js": "application/javascript; charset=utf-8",
  ".css": "text/css; charset=utf-8",
  ".json": "application/json",
  ".png": "image/png",
  ".jpg": "image/jpeg",
  ".gif": "image/gif",
  ".svg": "image/svg+xml",
  ".map": "application/json",
  ".ico": "image/x-icon",
};

/* ------------------------------------------------------------------ *
 * Voltronite access tokens (cached per user; password grant)          *
 * ------------------------------------------------------------------ */
const tokenCache = new Map<string, { token: string; at: number }>();

async function getToken(user: string, force = false): Promise<string> {
  const key = user || OWEN_USER;
  const hit = tokenCache.get(key);
  if (!force && hit && Date.now() - hit.at < 24 * 3600_000) return hit.token;
  const res = await fetch(`${VOLTRONITE}/account/api/oauth/token`, {
    method: "POST",
    headers: { "content-type": "application/x-www-form-urlencoded" },
    body: `grant_type=password&username=${encodeURIComponent(key)}`,
  });
  if (!res.ok) throw new Error(`oauth token failed: HTTP ${res.status}`);
  const json: any = await res.json();
  tokenCache.set(key, { token: json.access_token, at: Date.now() });
  return json.access_token;
}

/* ------------------------------------------------------------------ *
 * Query fingerprinting -> Voltronite operation mapping                *
 * ------------------------------------------------------------------ */
interface Mapping {
  /** Voltronite operationName (forwarded with a real Bearer token). */
  op?: string;
  /** pick/reshape data for the raw client query (default: identity). */
  extract?: (data: any) => any;
  /** local answer when Voltronite has no such operation. */
  local?: any;
  label: string;
}

const norm = (q: string) => q.replace(/\s+/g, " ").trim();

function mapQuery(query: string): Mapping {
  const q = norm(query);
  const isMutation = /^mutation\b/.test(q);

  if (!isMutation) {
    /* LightSwitch: fortnite + kairos service status (+ social ban dates).
     * Voltronite answers "kairosStatus: DOWN" which would park the hub on the
     * LightSwitch warning screen — the old Kairos client needs it UP. */
    if (q.includes("fortniteStatus") && q.includes("kairosStatus"))
      return {
        label: "GetServiceStatus(fortnite+kairos)",
        op: "GetStatusPerService",
        extract: (d) => {
          try {
            if (d?.LightSwitch?.kairosStatus) d.LightSwitch.kairosStatus.status = "UP";
            /* the client reads SocialBan.socialBan.starts_at (absent -> not banned) */
            if (d?.SocialBan && !("socialBan" in d.SocialBan)) d.SocialBan.socialBan = null;
          } catch {}
          return d;
        },
      };

    /* ContentControl voice permission. */
    if (q.includes("canUseVoiceChat"))
      return {
        label: "canVoiceChat",
        op: "GetStatusPerService",
        extract: (d) => ({ ContentControl: d?.ContentControl }),
      };

    /* Full account query (the one the PartyHub app initialization sends). */
    if (q.includes("myAccount") && q.includes("externalAuths"))
      return { label: "GetMyAccount", op: "GetMyAccount" };

    /* The built-in mock's identity probe. */
    if (q.replace(/\s/g, "") === "query{Account{myAccount{id}}}")
      return { label: "identity-probe", op: "GetMyAccount" };

    if (q.includes("hasAccountAccepted"))
      return { label: "EulaAccepted", op: "EulaAccepted" };

    if (q.includes("getLatestAndConvert"))
      return {
        label: "GetEula(local)",
        local: { data: { Eula: { getLatestAndConvert: { accepted: true, body: "", key: "fortnite", locale: "en-US", title: "", version: 1 } } } },
      };

    if (q.includes("myAvailableSetting"))
      return { label: "myAvailableSetting(local)", local: { data: { UserSettings: { myAvailableSetting: null } } } };

    if (q.includes("mySetting"))
      return { label: "GetMySetting", op: "GetMySetting" };

    if (q.includes("setting(key") && q.includes("accountIds"))
      return { label: "GetAccountSettings", op: "GetAccountSettings" };

    /* Friends + PartySettings notification settings. */
    if (q.includes("notificationSettings"))
      return { label: "GetNotificationSettings", op: "GetNotificationSettings" };

    /* Single-service lightswitch probe (bypass). */
    if (q.includes("allowedActions"))
      return {
        label: "serviceStatus(local)",
        local: { data: { LightSwitch: { serviceStatus: { allowedActions: ["PLAY"], status: "UP", banned: false } } } },
      };

    if (q.includes("getMySocialBan") && !q.includes("fortniteStatus"))
      return { label: "getMySocialBanSummary", op: "getMySocialBanSummary" };

    /* Friends. */
    if (q.includes("summary(displayNames"))
      return {
        label: "friends-summary(local)",
        local: { data: { Friends: { summary: { friends: [], incoming: [], outgoing: [], blocklist: [], suggestions: [] } } } },
      };
    if (q.includes("blockList"))
      return { label: "blockList(local)", local: { data: { Friends: { blockList: [] } } } };
    if (q.includes("friend(friendId"))
      return { label: "getFriend(local)", local: { data: { Friends: { friend: null } } } };

    /* Account lookups. */
    if (q.includes("accounts(accountIds"))
      return { label: "accounts(local)", local: { data: { Account: { accounts: [] } } } };
    if (q.includes("friendshipStatus"))
      return { label: "account(local)", local: { data: { Account: { account: null } } } };
    if (q.includes("unlinkedDieselPlatforms"))
      return { label: "unlinkedPlatforms(local)", local: { data: { Account: { myAccount: { unlinkedDieselPlatforms: [] } } } } };

    /* Fortnite profile: powers the PartyHub avatar. */
    if (q.includes("myProfile") && q.includes("character"))
      return {
        label: "myProfile-character(local)",
        local: {
          data: {
            Fortnite: {
              myProfile: {
                id: OWEN_ACCOUNT_ID,
                character: {
                  templateId: "CID_029_Athena_Commando_F_Halloween",
                  attributes: { favorite: false, item_seen: true, level: 1, variants: [] },
                },
                backpack: { templateId: "" },
              },
            },
          },
        },
      };
  } else {
    if (q.includes("updateSetting"))
      return { label: "UpdateUserSetting", op: "UpdateUserSetting" };
    if (q.includes("acceptEula"))
      return { label: "AcceptEula(local)", local: { data: { Eula: { acceptEula: { accepted: true } } } } };
    if (q.includes("setNotificationSettings"))
      return {
        label: "setNotificationSettings(local)",
        local: {
          data: q.includes("PartySettings")
            ? { PartySettings: { setNotificationSettings: { offline: null, success: true } } }
            : { Friends: { setNotificationSettings: { offline: null, success: true } } },
        },
      };
    if (q.includes("reportPlayer"))
      return {
        label: "reportPlayer(local)",
        local: { data: { Fortnite: { reportPlayer: { success: true, status: "OK", errorCode: "", errorMessage: "" } } } },
      };
    return { label: "generic-mutation(local)", local: { data: { success: true } } };
  }

  return { label: "unknown(local)", local: { data: {} } };
}

/* ------------------------------------------------------------------ *
 * Forward to Voltronite                                              *
 * ------------------------------------------------------------------ */
async function forward(op: string, body: any, depth = 0): Promise<any> {
  const token = await getToken(OWEN_USER, depth > 0);
  const res = await fetch(`${VOLTRONITE}/partyhub/graphql`, {
    method: "POST",
    headers: { "content-type": "application/json", authorization: `Bearer ${token}` },
    body: JSON.stringify({
      operationName: op,
      query: typeof body?.query === "string" ? body.query : `query ${op}`,
      variables: body?.variables ?? {},
    }),
  });
  if (res.status === 401 && depth === 0) return forward(op, body, 1);
  const json: any = await res.json().catch(() => ({}));
  if (!res.ok) throw new Error(`voltronite HTTP ${res.status}`);
  return json;
}

async function handleGraphql(bodyJson: any, log: string[]): Promise<any> {
  if (Array.isArray(bodyJson)) {
    const out: any[] = [];
    for (const item of bodyJson) out.push(await handleGraphql(item, log));
    return out;
  }
  const query = String(bodyJson?.query ?? "");
  const mapping = mapQuery(query);
  log.push(`${mapping.label}${mapping.op ? ` -> ${VOLTRONITE}/partyhub/graphql` : ""}`);
  if (mapping.op) {
    try {
      const json = await forward(mapping.op, bodyJson);
      const data = mapping.extract ? mapping.extract(json?.data) : json?.data;
      return { data };
    } catch (err) {
      console.log(`  !! forward failed (${mapping.op}): ${(err as Error).message} — local fallback`);
      return mapping.local ?? { data: {} };
    }
  }
  return mapping.local ?? { data: {} };
}

/* ------------------------------------------------------------------ *
 * Recent-request ring buffer for the web console                     *
 * ------------------------------------------------------------------ */
interface Hit {
  time: string;
  method: string;
  path: string;
  op: string;
  via: "voltronite" | "local" | "-";
  detail: string;
}
const hits: Hit[] = [];
const pushHit = (h: Hit) => {
  hits.push(h);
  if (hits.length > 200) hits.shift();
};

/* ------------------------------------------------------------------ *
 * Server                                                             *
 * ------------------------------------------------------------------ */
const server = serve({
  port: PORT,
  async fetch(req) {
    const url = new URL(req.url);
    const p = decodeURIComponent(url.pathname);
    const method = req.method;
    const ts = new Date().toISOString().slice(11, 19);

    /* ---- graphql adapter (same-origin on every SPA route) ---- */
    if (
      method === "POST" &&
      (p === "/owen-graphql" ||
        p.endsWith("/owen-graphql") ||
        p.endsWith("/graphql") ||
        p === "/graphql" ||
        p.endsWith("/api/graphql"))
    ) {
      /* /owen-graphql is the v3 bridge relay: owen-bridge.js already mapped
       * the raw query to a Voltronite operation and tells us the backend +
       * user via headers — forward verbatim with a real Bearer token. */
      if (p === "/owen-graphql" || p.endsWith("/owen-graphql")) {
        const bodyText = await req.text();
        let bodyJson: any = null;
        try { bodyJson = JSON.parse(bodyText); } catch {}
        const backend = (req.headers.get("x-owen-backend") || VOLTRONITE).replace(/\/+$/, "");
        const user = req.headers.get("x-owen-user") || OWEN_USER;
        const op = String(bodyJson?.operationName ?? "?");
        try {
          let token = await getToken(user);
          let voltronite: any = null;
          for (let attempt = 0; attempt < 2; attempt++) {
            const res = await fetch(`${backend}/partyhub/graphql`, {
              method: "POST",
              headers: { "content-type": "application/json", authorization: `Bearer ${token}` },
              body: JSON.stringify(bodyJson ?? {}),
            });
            if (res.status === 401 && attempt === 0) {
              token = await getToken(user, true);
              continue;
            }
            voltronite = await res.json().catch(() => ({}));
            if (!res.ok) throw new Error(`voltronite HTTP ${res.status}`);
            break;
          }
          console.log(`[${ts}] RELAY   ${op}  user=${user}  -> ${backend}/partyhub/graphql`);
          pushHit({
            time: ts,
            method: "POST",
            path: p,
            op: `${op} (v3 relay)`,
            via: "voltronite",
            detail: `user=${user} -> ${backend}`,
          });
          return new Response(JSON.stringify(voltronite), {
            headers: { "content-type": "application/json", "access-control-allow-origin": "*" },
          });
        } catch (err) {
          console.log(`  !! relay failed (${op}): ${(err as Error).message}`);
          pushHit({
            time: ts,
            method: "POST",
            path: p,
            op: `${op} (v3 relay FAILED)`,
            via: "voltronite",
            detail: `${err instanceof Error ? err.message : err} (bridge will fall back locally)`,
          });
          return new Response(JSON.stringify({ errors: [{ message: String(err) }] }), {
            status: 502,
            headers: { "content-type": "application/json", "access-control-allow-origin": "*" },
          });
        }
      }

      const bodyText = await req.text();
      let bodyJson: any = null;
      try {
        bodyJson = JSON.parse(bodyText);
      } catch {}
      const queries: string[] = Array.isArray(bodyJson) ? bodyJson.map((b: any) => norm(String(b?.query ?? ""))) : [norm(String(bodyJson?.query ?? ""))];
      const log: string[] = [];
      const resp = await handleGraphql(bodyJson, log);
      const ops = log.join(" | ");
      console.log(`[${ts}] GRAPHQL ${ops}`);
      for (const q of queries) console.log(`           q: ${q.slice(0, 110)}${q.length > 110 ? "..." : ""}`);
      pushHit({
        time: ts,
        method: "POST",
        path: p,
        op: ops,
        via: ops.includes("->") ? "voltronite" : "local",
        detail: queries.join(" ; ").slice(0, 160),
      });
      return new Response(JSON.stringify(resp), {
        headers: { "content-type": "application/json", "access-control-allow-origin": "*" },
      });
    }

    if (method === "OPTIONS") {
      return new Response(null, {
        status: 204,
        headers: {
          "access-control-allow-origin": "*",
          "access-control-allow-methods": "GET, POST, OPTIONS",
          "access-control-allow-headers": "*",
        },
      });
    }

    /* ---- status endpoint for the web console ---- */
    if (method === "GET" && (p === "/__status" || p.endsWith("/__status"))) {
      return new Response(
        JSON.stringify(
          {
            ok: true,
            root: ROOT,
            voltronite: VOLTRONITE,
            user: OWEN_USER,
            accountId: OWEN_ACCOUNT_ID,
            backendReachable: tokenCache.size > 0,
            hits: hits.slice(-60).reverse(),
          },
          null,
          2,
        ),
        { headers: { "content-type": "application/json", "access-control-allow-origin": "*" } },
      );
    }

    /* ---- convenience redirect ---- */
    if (p === "/" || p === "") {
      return new Response(null, { status: 302, headers: { location: "/offline/en-US/" } });
    }

    /* ---- static files ---- */
    const candidates = [
      path.join(ROOT, p.replace(/\.\./g, "")),
      /* relative assets from nested SPA paths (/offline/en-US/main.js -> entry/main.js) */
      path.join(ENTRY, path.basename(p)),
    ];
    const file = candidates.find((f) => existsSync(f) && !f.endsWith("/"));
    if (file) {
      pushHit({ time: ts, method, path: p, op: path.basename(file), via: "-", detail: "static" });
      const ext = path.extname(file).toLowerCase();
      return new Response(readFileSync(file), {
        headers: { "content-type": MIME[ext] ?? "application/octet-stream" },
      });
    }

    /* ---- SPA fallback: /offline/:locale/... and every app route ---- */
    pushHit({ time: ts, method, path: p, op: "index.html", via: "-", detail: "spa" });
    return new Response(readFileSync(path.join(ENTRY, "index.html")), {
      headers: { "content-type": "text/html; charset=utf-8" },
    });
  },
});

console.log(`partyhub-web on :${server.port} root=${ROOT}`);
console.log(`voltronite at   : ${VOLTRONITE} (user ${OWEN_USER} / account ${OWEN_ACCOUNT_ID})`);
console.log(`open            : /partyhub/offline/en-US/ (via the web console proxy)`);
console.log(`                : /offline/en-US/             (direct)`);
