import { NextRequest, NextResponse } from "next/server";
import { getVersionDB, VERSIONS } from "@/lib/fndb";

export const dynamic = "force-dynamic";

export async function GET(req: NextRequest) {
  const ver = req.nextUrl.searchParams.get("ver") ?? "";
  if (!VERSIONS.includes(ver as (typeof VERSIONS)[number])) {
    return NextResponse.json(
      { error: "invalid_version", versions: VERSIONS },
      { status: 400 }
    );
  }
  const q = (req.nextUrl.searchParams.get("q") ?? "").trim().toLowerCase();
  const page = Math.max(parseInt(req.nextUrl.searchParams.get("page") ?? "1", 10) || 1, 1);
  const pageSize = Math.min(
    Math.max(parseInt(req.nextUrl.searchParams.get("pageSize") ?? "100", 10) || 100, 10),
    500
  );

  const db = getVersionDB(ver);
  let names = Object.keys(db).sort();
  if (q) names = names.filter((n) => n.toLowerCase().includes(q));

  const total = names.length;
  const start = (page - 1) * pageSize;
  const slice = names.slice(start, start + pageSize).map((n) => ({
    name: n,
    thunk: db[n].thunk,
    orig: db[n].orig ?? null,
  }));

  return NextResponse.json({
    ver,
    page,
    pageSize,
    total,
    totalPages: Math.ceil(total / pageSize),
    functions: slice,
  });
}
