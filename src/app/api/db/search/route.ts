import { NextRequest, NextResponse } from "next/server";
import { searchFunctions, type SearchMode } from "@/lib/fndb";

export const dynamic = "force-dynamic";

export async function GET(req: NextRequest) {
  const q = req.nextUrl.searchParams.get("q") ?? "";
  const mode = (req.nextUrl.searchParams.get("mode") ?? "contains") as SearchMode;
  const limit = Math.min(
    Math.max(parseInt(req.nextUrl.searchParams.get("limit") ?? "60", 10) || 60, 1),
    300
  );
  if (!q.trim()) {
    return NextResponse.json({ hits: [], total: 0, q, mode });
  }
  const validMode: SearchMode[] = ["contains", "prefix", "exact"];
  const m: SearchMode = validMode.includes(mode) ? mode : "contains";
  const t0 = Date.now();
  const { hits, total } = searchFunctions(q, m, limit);
  return NextResponse.json({
    q,
    mode: m,
    total,
    returned: hits.length,
    ms: Date.now() - t0,
    hits,
  });
}
