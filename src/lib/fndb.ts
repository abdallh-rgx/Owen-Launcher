import fs from "fs";
import path from "path";

export const VERSIONS = [
  "5.41",
  "7.40",
  "9.41",
  "13.40",
  "17.50",
  "18.40",
  "19.10",
  "21.30",
  "22.40",
  "23.50",
  "30.00",
] as const;

export type Version = (typeof VERSIONS)[number];

export interface FuncEntry {
  thunk: string;
  orig?: string;
}

type ByName = Record<string, Partial<Record<Version, FuncEntry>>>;

const DATA_DIR = path.join(process.cwd(), "data", "db");
const ERBIUM_DIR = path.join(process.cwd(), "data", "erbium");

let byNameCache: ByName | null = null;
let perVersionCache: Map<string, Record<string, FuncEntry>> | null = null;
let statsCache: Record<string, unknown> | null = null;

export function getByName(): ByName {
  if (!byNameCache) {
    const raw = fs.readFileSync(path.join(DATA_DIR, "by-name.json"), "utf8");
    byNameCache = JSON.parse(raw) as ByName;
  }
  return byNameCache;
}

export function getVersionDB(ver: string): Record<string, FuncEntry> {
  if (!VERSIONS.includes(ver as Version)) throw new Error("bad version");
  if (!perVersionCache) perVersionCache = new Map();
  let db = perVersionCache.get(ver);
  if (!db) {
    const raw = fs.readFileSync(
      path.join(DATA_DIR, `natives-v${ver}.json`),
      "utf8"
    );
    db = JSON.parse(raw) as Record<string, FuncEntry>;
    perVersionCache.set(ver, db);
  }
  return db;
}

export function getStats() {
  if (statsCache) return statsCache;
  const byName = getByName();
  const perVersion: Record<string, number> = {};
  for (const ver of VERSIONS) {
    try {
      const db = getVersionDB(ver);
      perVersion[ver] = Object.keys(db).length;
    } catch {
      perVersion[ver] = 0;
    }
  }
  const unique = Object.keys(byName).length;
  const inAll = Object.values(byName).filter(
    (v) => Object.keys(v).length === VERSIONS.length
  ).length;
  statsCache = { perVersion, unique, inAll, versions: VERSIONS };
  return statsCache;
}

const WHITELIST_DIRS: Record<string, string> = {
  db: DATA_DIR,
  erbium: ERBIUM_DIR,
};

export function resolveDownloadPath(
  group: string,
  file: string
): string | null {
  const dir = WHITELIST_DIRS[group];
  if (!dir) return null;
  if (!/^[A-Za-z0-9._-]+$/.test(file)) return null;
  const p = path.join(dir, file);
  if (!p.startsWith(dir)) return null;
  if (!fs.existsSync(p) || !fs.statSync(p).isFile()) return null;
  return p;
}

export type SearchMode = "contains" | "prefix" | "exact";

export interface SearchHit {
  name: string;
  versions: Partial<Record<Version, FuncEntry>>;
}

export function searchFunctions(
  q: string,
  mode: SearchMode = "contains",
  limit = 60
): { hits: SearchHit[]; total: number } {
  const byName = getByName();
  const query = q.trim();
  if (!query) return { hits: [], total: 0 };
  const names = Object.keys(byName);
  let matched: string[];
  if (mode === "exact") {
    matched = names.filter((n) => n === query);
  } else if (mode === "prefix") {
    const lower = query.toLowerCase();
    matched = names.filter((n) => n.toLowerCase().startsWith(lower));
  } else {
    const lower = query.toLowerCase();
    matched = names.filter((n) => n.toLowerCase().includes(lower));
  }
  matched.sort();
  const hits = matched.slice(0, limit).map((n) => ({
    name: n,
    versions: byName[n],
  }));
  return { hits, total: matched.length };
}
