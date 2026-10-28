import { NextResponse } from "next/server";
import { db } from "@/lib/db";

export const runtime = "nodejs";

/**
 * GET /api/status — aggregated counts + config presence + latest scans
 * for the Dashboard overview cards.
 */
export async function GET() {
  try {
    const [config, rules, servers, seasons, keys, scans] = await Promise.all([
      db.redirecterConfig.findUnique({ where: { id: 1 } }),
      db.redirectRule.count(),
      db.serverProfile.count(),
      db.seasonRecord.count(),
      db.keyEntry.count(),
      db.scanRecord.findMany({
        orderBy: { createdAt: "desc" },
        take: 5,
      }),
    ]);

    return NextResponse.json({
      redirecter: {
        configured: Boolean(
          config &&
            (() => {
              try {
                const parsed = JSON.parse(config.configJson) as { server?: { base_url?: string } };
                return Boolean(parsed.server?.base_url);
              } catch {
                return false;
              }
            })()
        ),
        updatedAt: config?.updatedAt ?? null,
      },
      rules,
      servers,
      seasons,
      keys,
      scans: scans.map((s) => ({
        id: s.id,
        fileName: s.fileName,
        bestVa: s.bestVa,
        score: s.score,
        variadic: s.variadic,
        createdAt: s.createdAt,
      })),
    });
  } catch (err) {
    console.error("GET /api/status failed", err);
    return NextResponse.json({ error: "Failed to load status" }, { status: 500 });
  }
}
