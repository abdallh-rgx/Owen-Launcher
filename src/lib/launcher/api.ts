/* Owen Launcher — API client for the Owen Rust backend (owen-server).
 * Same-origin via the Next.js rewrite: /owen-api/* → 127.0.0.1:3010/*.
 */

export interface Account {
  accountId: string;
  username: string;
  displayName: string;
  email: string;
  created: string;
  level: number;
  vbucks: number;
  battlePassPurchased: boolean;
  banned: boolean;
  itemsOwned: number;
  selectedSeason: number;
}

export interface SeasonMeta {
  season: number;
  chapter: number;
  chapterSeason: number;
  name: string;
  version: string;
  startDate: string;
  endDate: string;
  image: string | null;
  paidTiers: number;
  freeTiers: number;
  active: boolean;
}

export interface SeasonsResponse {
  current: number;
  seasons: SeasonMeta[];
}

export interface ManifestEntry {
  version: string;
  cl: string;
  releaseString: string;
  manifestHash: string;
  label: string;
  date: string;
  cdn: string;
}

export interface ServerStatus {
  server: string;
  version: string;
  backend_url: string;
  uptime_seconds: number;
  accounts: number;
  registered: number;
  seasons: number;
  manifests: number;
  partyhub: boolean;
  xmpp_port: number;
}

export interface TokenResponse {
  access_token: string;
  refresh_token: string;
  expires_in: number;
  account_id: string;
  displayName: string;
}

export interface McpProfile {
  profileRevision: number;
  profileChanges: Array<{
    changeType: string;
    profile?: {
      items?: Array<{ templateId: string; quantity: number; attributes?: Record<string, unknown> }>;
      stats?: { attributes?: Record<string, unknown> };
    };
  }>;
  serverTime: string;
}

export class ApiError extends Error {
  code?: string;
  constructor(message: string, code?: string) {
    super(message);
    this.code = code;
  }
}

const BASE = "/owen-api";
const TOKEN_KEY = "owen:web-token";
const ME_KEY = "owen:web-me";

export const tokenStore = {
  get(): string {
    try {
      return localStorage.getItem(TOKEN_KEY) || "";
    } catch {
      return "";
    }
  },
  set(t: string) {
    try {
      localStorage.setItem(TOKEN_KEY, t);
    } catch {
      /* ignore */
    }
  },
  clear() {
    try {
      localStorage.removeItem(TOKEN_KEY);
      localStorage.removeItem(ME_KEY);
    } catch {
      /* ignore */
    }
  },
  cacheMe(a: Account | null) {
    try {
      if (a) localStorage.setItem(ME_KEY, JSON.stringify(a));
      else localStorage.removeItem(ME_KEY);
    } catch {
      /* ignore */
    }
  },
  cachedMe(): Account | null {
    try {
      const raw = localStorage.getItem(ME_KEY);
      return raw ? (JSON.parse(raw) as Account) : null;
    } catch {
      return null;
    }
  },
};

async function call<T>(
  method: string,
  path: string,
  opts: {
    body?: unknown;
    auth?: boolean;
    form?: Record<string, string>;
    headers?: Record<string, string>;
  } = {},
): Promise<T> {
  const headers: Record<string, string> = { ...opts.headers };
  let body: string | undefined;
  if (opts.form) {
    headers["Content-Type"] = "application/x-www-form-urlencoded";
    body = new URLSearchParams(opts.form).toString();
  } else if (opts.body !== undefined) {
    headers["Content-Type"] = "application/json";
    body = JSON.stringify(opts.body);
  }
  if (opts.auth !== false && tokenStore.get()) {
    headers["Authorization"] = "Bearer " + tokenStore.get();
  }
  let res: Response;
  try {
    // NOTE: "/owen-api/" (trailing slash) triggers a pointless 308 — use
    // the bare prefix for the root path.
    const url = BASE + (path === "/" ? "" : path);
    res = await fetch(url, { method, headers, body, cache: "no-store" });
  } catch {
    throw new ApiError("network", "errors.com.owen.network");
  }
  const text = await res.text();
  let json: Record<string, unknown> = {};
  try {
    json = text ? JSON.parse(text) : {};
  } catch {
    /* non-JSON */
  }
  if (!res.ok) {
    throw new ApiError(
      (json.message as string) ||
        (json.error_description as string) ||
        (json.error as string) ||
        `HTTP ${res.status}`,
      (json.errorCode as string) || undefined,
    );
  }
  return json as T;
}

export const api = {
  status: () => call<ServerStatus>("GET", "/"),
  lightswitch: () =>
    call<Array<{ serviceInstanceId: string; status: string }>>(
      "GET",
      "/lightswitch/api/service/bulk/status?serviceId=fortnite",
    ),
  register: (data: {
    email: string;
    username: string;
    displayName: string;
    password: string;
  }) =>
    call<TokenResponse & Account>("POST", "/api/launcher/register", {
      body: data,
      auth: false,
    }),
  login: (login: string, password: string) =>
    call<TokenResponse>("POST", "/account/api/oauth/token", {
      form: { grant_type: "password", username: login, password, token_type: "eg1" },
      auth: false,
    }),
  verify: () =>
    call<{ accountId: string; displayName: string }>(
      "GET",
      "/account/api/oauth/verify",
    ),
  me: () => call<Account>("GET", "/api/launcher/me"),
  setSeason: (season: number) =>
    call<Account>("POST", "/api/launcher/me/season", { body: { season } }),
  logout: () => call<unknown>("POST", "/api/launcher/logout"),
  seasons: () => call<SeasonsResponse>("GET", "/api/launcher/seasons"),
  seasonDetail: (n: number) =>
    call<{
      meta: SeasonMeta & { paidTiers: number; freeTiers: number };
      battlepass: BattlepassFile | null;
    }>("GET", `/api/launcher/seasons/${n}`),
  manifests: () => call<ManifestEntry[]>("GET", "/api/launcher/manifests"),
  queryProfile: (accountId: string, profileId = "athena", season?: number) =>
    call<McpProfile>(
      "POST",
      `/fortnite/api/game/v2/profile/${accountId}/client/QueryProfile?profileId=${profileId}`,
      {
        body: { profileId },
        headers: season ? { "X-Season": String(season) } : {},
      },
    ),
};

/* ------------------------- battlepass parsing ------------------------- */

export interface BattlepassFile {
  battleBundleOfferId?: string;
  battlePassOfferId?: string;
  tierOfferId?: string;
  paidRewards?: Array<Record<string, number>>;
  freeRewards?: Array<Record<string, number>>;
  [k: string]: unknown;
}

export function countTiers(arr?: Array<Record<string, number>>): number {
  return (arr || []).filter((t) => Object.keys(t).length > 0).length;
}

export function collectCosmetics(
  arr?: Array<Record<string, number>>,
  maxTier = Infinity,
): Array<{ templateId: string; quantity: number; tier: number; kind: string }> {
  const out: Array<{ templateId: string; quantity: number; tier: number; kind: string }> = [];
  (arr || []).forEach((tierMap, idx) => {
    if (idx + 1 > maxTier) return;
    for (const [templateId, quantity] of Object.entries(tierMap || {})) {
      out.push({ templateId, quantity, tier: idx + 1, kind: kindOf(templateId) });
    }
  });
  return out;
}

export function kindOf(templateId: string): string {
  const t = templateId.split(":")[0] || "";
  const map: Record<string, string> = {
    AthenaCharacter: "outfit",
    AthenaBackpack: "backpack",
    AthenaPickaxe: "pickaxe",
    AthenaGlider: "glider",
    AthenaSkyDiveContrail: "trail",
    AthenaItemWrap: "wrap",
    AthenaDance: "emote",
    AthenaLoadingScreen: "loading",
    AthenaMusicPack: "music",
    Emoji: "emoji",
    Token: "token",
    ChallengeBundle: "bundle",
    ChallengeBundleSchedule: "schedule",
    HomeBanner: "banner",
    Currency: "currency",
    "Cosmetic.VariantToken": "variant",
  };
  return map[t] || t.toLowerCase() || "other";
}

export const KIND_COLORS: Record<string, string> = {
  outfit: "from-violet-500 to-fuchsia-500",
  backpack: "from-emerald-500 to-teal-500",
  pickaxe: "from-amber-500 to-orange-500",
  glider: "from-sky-400 to-cyan-500",
  trail: "from-rose-500 to-pink-500",
  wrap: "from-indigo-400 to-violet-500",
  emote: "from-lime-400 to-emerald-500",
  loading: "from-slate-400 to-slate-600",
  music: "from-fuchsia-400 to-purple-600",
  emoji: "from-yellow-400 to-amber-500",
  token: "from-teal-300 to-emerald-400",
  banner: "from-zinc-400 to-zinc-600",
  currency: "from-emerald-300 to-green-500",
  variant: "from-pink-300 to-rose-400",
  schedule: "from-cyan-300 to-sky-400",
  bundle: "from-orange-300 to-amber-500",
};

export function seasonGradient(season: number): string {
  // Deterministic per-season gradient (no blue/indigo bases).
  const hue = (season * 47) % 360;
  const hue2 = (hue + 60) % 360;
  return `linear-gradient(135deg, hsl(${hue} 65% 42%), hsl(${hue2} 70% 30%))`;
}
