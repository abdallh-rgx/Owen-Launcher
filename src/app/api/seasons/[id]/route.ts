import { NextResponse } from "next/server";
import { db } from "@/lib/db";

export const runtime = "nodejs";

/**
 * GET /api/seasons/[id] — full season payload (the stored JSON, verbatim, so
 * the client can export it in the same owen-pak / Reload-Backend shape).
 */
export async function GET(
  _req: Request,
  { params }: { params: Promise<{ id: string }> }
) {
  try {
    const { id } = await params;
    const numeric = Number(id);
    const record = Number.isInteger(numeric) && numeric > 0
      ? await db.seasonRecord.findUnique({ where: { seasonNumber: numeric } })
      : await db.seasonRecord.findUnique({ where: { id } });
    if (!record) {
      return NextResponse.json({ error: "Season not found" }, { status: 404 });
    }
    let data: unknown = {};
    try {
      data = JSON.parse(record.dataJson);
    } catch {
      data = {};
    }
    return NextResponse.json({
      id: record.id,
      seasonNumber: record.seasonNumber,
      displayName: record.displayName,
      updatedAt: record.updatedAt,
      data,
    });
  } catch (err) {
    console.error("GET /api/seasons/[id] failed", err);
    return NextResponse.json({ error: "Failed to load season" }, { status: 500 });
  }
}
