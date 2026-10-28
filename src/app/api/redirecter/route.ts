import { NextResponse } from "next/server";
import { db } from "@/lib/db";

export const runtime = "nodejs";

/**
 * The exact owen.json shape consumed by libowen.so at runtime.
 */
export interface OwenConfig {
  version: 1;
  server: {
    base_url: string;
    xmpp_host: string;
    xmpp_port: number;
    login_path: string;
    graphql_path: string;
  };
  redirect: {
    domains: string[];
    extra_rules: Array<{ match: string; replace_path: string }>;
  };
  manifest: { mode: "auto" | "off" };
  webview: {
    login_url: string;
    force_cleartext: boolean;
    partyhub: "auto" | "off";
  };
  detect: {
    strategy: "auto" | "symbols-only" | "signature-only";
    min_score: number;
  };
  hooks: { curl: boolean; art_webview: boolean };
}

export const DEFAULT_CONFIG: OwenConfig = {
  version: 1,
  server: {
    base_url: "",
    xmpp_host: "",
    xmpp_port: 5222,
    login_path: "/login",
    graphql_path: "/partyhub/graphql",
  },
  redirect: {
    domains: [
      "game-social.epicgames.com",
      "ol.epicgames.com",
      "ol.epicgames.net",
      "on.epicgames.com",
      "ak.epicgames.com",
      "epicgames.dev",
    ],
    extra_rules: [],
  },
  manifest: { mode: "auto" },
  webview: {
    login_url: "",
    force_cleartext: true,
    partyhub: "auto",
  },
  detect: { strategy: "auto", min_score: 6 },
  hooks: { curl: true, art_webview: true },
};

function sanitize(raw: unknown): OwenConfig {
  const d = DEFAULT_CONFIG;
  if (typeof raw !== "object" || raw === null) return d;
  const r = raw as Record<string, unknown>;
  const server = (r.server ?? {}) as Record<string, unknown>;
  const redirect = (r.redirect ?? {}) as Record<string, unknown>;
  const manifest = (r.manifest ?? {}) as Record<string, unknown>;
  const webview = (r.webview ?? {}) as Record<string, unknown>;
  const detect = (r.detect ?? {}) as Record<string, unknown>;
  const hooks = (r.hooks ?? {}) as Record<string, unknown>;

  const str = (v: unknown, fallback: string) =>
    typeof v === "string" ? v : fallback;
  const num = (v: unknown, fallback: number) =>
    typeof v === "number" && Number.isFinite(v) ? v : fallback;
  const bool = (v: unknown, fallback: boolean) =>
    typeof v === "boolean" ? v : fallback;
  const oneOf = <T extends string>(v: unknown, allowed: T[], fallback: T): T =>
    typeof v === "string" && (allowed as string[]).includes(v) ? (v as T) : fallback;

  const domains = Array.isArray(redirect.domains)
    ? redirect.domains.filter((x): x is string => typeof x === "string")
    : d.redirect.domains;

  const extraRules = Array.isArray(redirect.extra_rules)
    ? redirect.extra_rules
        .filter(
          (x): x is { match: string; replace_path: string } =>
            typeof x === "object" &&
            x !== null &&
            typeof (x as Record<string, unknown>).match === "string" &&
            typeof (x as Record<string, unknown>).replace_path === "string"
        )
        .map((x) => ({ match: x.match, replace_path: x.replace_path }))
    : [];

  return {
    version: 1,
    server: {
      base_url: str(server.base_url, d.server.base_url),
      xmpp_host: str(server.xmpp_host, d.server.xmpp_host),
      xmpp_port: num(server.xmpp_port, d.server.xmpp_port),
      login_path: str(server.login_path, d.server.login_path),
      graphql_path: str(server.graphql_path, d.server.graphql_path),
    },
    redirect: { domains, extra_rules: extraRules },
    manifest: { mode: oneOf(manifest.mode, ["auto", "off"], "auto") },
    webview: {
      login_url: str(webview.login_url, d.webview.login_url),
      force_cleartext: bool(webview.force_cleartext, true),
      partyhub: oneOf(webview.partyhub, ["auto", "off"], "auto"),
    },
    detect: {
      strategy: oneOf(detect.strategy, ["auto", "symbols-only", "signature-only"], "auto"),
      min_score: num(detect.min_score, 6),
    },
    hooks: {
      curl: bool(hooks.curl, true),
      art_webview: bool(hooks.art_webview, true),
    },
  };
}

export async function GET() {
  try {
    const row = await db.redirecterConfig.findUnique({ where: { id: 1 } });
    const config = row ? sanitize(JSON.parse(row.configJson)) : DEFAULT_CONFIG;
    const rules = await db.redirectRule.findMany({ orderBy: { createdAt: "asc" } });
    return NextResponse.json({
      config,
      rules: rules.map((r) => ({
        id: r.id,
        match: r.match,
        replacePath: r.replacePath,
        enabled: r.enabled,
      })),
    });
  } catch (err) {
    console.error("GET /api/redirecter failed", err);
    return NextResponse.json({ error: "Failed to load redirecter config" }, { status: 500 });
  }
}

export async function PUT(req: Request) {
  try {
    const body = await req.json();
    const config = sanitize(body);
    await db.redirecterConfig.upsert({
      where: { id: 1 },
      update: { configJson: JSON.stringify(config) },
      create: { id: 1, configJson: JSON.stringify(config) },
    });
    // Derive RedirectRule rows from extra_rules (replace-all semantics).
    await db.redirectRule.deleteMany({});
    if (config.redirect.extra_rules.length > 0) {
      await db.redirectRule.createMany({
        data: config.redirect.extra_rules.map((r) => ({
          match: r.match,
          replacePath: r.replace_path,
          enabled: true,
        })),
      });
    }
    return NextResponse.json({ ok: true, config });
  } catch (err) {
    console.error("PUT /api/redirecter failed", err);
    return NextResponse.json({ error: "Failed to save redirecter config" }, { status: 500 });
  }
}
