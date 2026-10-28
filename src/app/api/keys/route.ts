import { NextResponse } from "next/server";
import { db } from "@/lib/db";

export const runtime = "nodejs";

/** Alphabet for generated keys (no ambiguous I, O, 0, 1). */
const ALPHABET = "ABCDEFGHJKLMNPQRSTUVWXYZ23456789";

function generateKey(): string {
  const bytes = new Uint8Array(20);
  crypto.getRandomValues(bytes);
  const chars = Array.from(bytes, (b) => ALPHABET[b % ALPHABET.length]);
  return [0, 4, 8, 12, 16].map((i) => chars.slice(i, i + 4).join("")).join("-");
}

const ROLES = ["admin", "moderator", "player"];

export async function GET() {
  try {
    const rows = await db.keyEntry.findMany({ orderBy: { createdAt: "desc" } });
    return NextResponse.json(
      rows.map((k) => ({
        id: k.id,
        key: k.key,
        role: k.role,
        note: k.note,
        revoked: k.revoked,
        lastUsedAt: k.lastUsedAt,
        createdAt: k.createdAt,
      }))
    );
  } catch (err) {
    console.error("GET /api/keys failed", err);
    return NextResponse.json({ error: "Failed to list keys" }, { status: 500 });
  }
}

export async function POST(req: Request) {
  try {
    const body = await req.json().catch(() => ({}));
    const role =
      typeof body?.role === "string" && ROLES.includes(body.role)
        ? body.role
        : "player";
    const note = typeof body?.note === "string" ? body.note : "";
    const key = generateKey();
    const entry = await db.keyEntry.create({ data: { key, role, note } });
    return NextResponse.json({
      id: entry.id,
      key: entry.key,
      role: entry.role,
      note: entry.note,
      revoked: entry.revoked,
      lastUsedAt: entry.lastUsedAt,
      createdAt: entry.createdAt,
    });
  } catch (err) {
    console.error("POST /api/keys failed", err);
    return NextResponse.json({ error: "Failed to create key" }, { status: 500 });
  }
}

export async function PATCH(req: Request) {
  try {
    const body = await req.json();
    if (typeof body?.id !== "string") {
      return NextResponse.json({ error: "id is required" }, { status: 400 });
    }
    const patch: Record<string, unknown> = {};
    if (typeof body.role === "string" && ROLES.includes(body.role)) patch.role = body.role;
    if (typeof body.note === "string") patch.note = body.note;
    if (typeof body.revoked === "boolean") patch.revoked = body.revoked;
    if (body.lastUsed === true) patch.lastUsedAt = new Date();
    const entry = await db.keyEntry.update({ where: { id: body.id }, data: patch });
    return NextResponse.json({
      id: entry.id,
      key: entry.key,
      role: entry.role,
      note: entry.note,
      revoked: entry.revoked,
      lastUsedAt: entry.lastUsedAt,
      createdAt: entry.createdAt,
    });
  } catch (err) {
    console.error("PATCH /api/keys failed", err);
    return NextResponse.json({ error: "Failed to update key" }, { status: 500 });
  }
}

export async function DELETE(req: Request) {
  try {
    const { searchParams } = new URL(req.url);
    const id = searchParams.get("id");
    if (!id) {
      return NextResponse.json({ error: "Query param id is required" }, { status: 400 });
    }
    await db.keyEntry.delete({ where: { id } });
    return NextResponse.json({ ok: true });
  } catch (err) {
    console.error("DELETE /api/keys failed", err);
    return NextResponse.json({ error: "Failed to delete key" }, { status: 500 });
  }
}
