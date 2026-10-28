import { NextResponse } from "next/server";
import { db } from "@/lib/db";

export const runtime = "nodejs";

export async function GET() {
  try {
    const rules = await db.redirectRule.findMany({ orderBy: { createdAt: "asc" } });
    return NextResponse.json(
      rules.map((r) => ({
        id: r.id,
        match: r.match,
        replacePath: r.replacePath,
        enabled: r.enabled,
      }))
    );
  } catch (err) {
    console.error("GET /api/rules failed", err);
    return NextResponse.json({ error: "Failed to list rules" }, { status: 500 });
  }
}

export async function POST(req: Request) {
  try {
    const body = await req.json();
    const match = typeof body?.match === "string" ? body.match.trim() : "";
    const replacePath =
      typeof body?.replacePath === "string" ? body.replacePath.trim() : "";
    if (!match || !replacePath) {
      return NextResponse.json(
        { error: "Both match pattern and replace path are required" },
        { status: 400 }
      );
    }
    const rule = await db.redirectRule.create({
      data: { match, replacePath, enabled: body?.enabled !== false },
    });
    return NextResponse.json({
      id: rule.id,
      match: rule.match,
      replacePath: rule.replacePath,
      enabled: rule.enabled,
    });
  } catch (err) {
    console.error("POST /api/rules failed", err);
    return NextResponse.json({ error: "Failed to create rule" }, { status: 500 });
  }
}

export async function DELETE(req: Request) {
  try {
    const { searchParams } = new URL(req.url);
    const id = searchParams.get("id");
    if (!id) {
      return NextResponse.json({ error: "Query param id is required" }, { status: 400 });
    }
    await db.redirectRule.delete({ where: { id } });
    return NextResponse.json({ ok: true });
  } catch (err) {
    console.error("DELETE /api/rules failed", err);
    return NextResponse.json({ error: "Failed to delete rule" }, { status: 500 });
  }
}
