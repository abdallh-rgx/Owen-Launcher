import { NextResponse } from "next/server";
import { db } from "@/lib/db";

export const runtime = "nodejs";

const BACKENDS = ["lawin-v2", "reload-backend", "custom"];
const FEATURES = [
  "profiles",
  "matchmaking",
  "storefront",
  "cloudstorage",
  "xmpp",
  "lightswitch",
  "contentpages",
  "keychain",
  "manifest",
];

function parseFeatures(csv: string): string[] {
  return csv
    .split(",")
    .map((s) => s.trim())
    .filter((s) => s.length > 0);
}

export async function GET() {
  try {
    const rows = await db.serverProfile.findMany({ orderBy: { createdAt: "asc" } });
    return NextResponse.json(
      rows.map((s) => ({
        id: s.id,
        name: s.name,
        baseUrl: s.baseUrl,
        backend: s.backend,
        features: parseFeatures(s.features),
      }))
    );
  } catch (err) {
    console.error("GET /api/servers failed", err);
    return NextResponse.json({ error: "Failed to list servers" }, { status: 500 });
  }
}

export async function POST(req: Request) {
  try {
    const body = await req.json();
    const name = typeof body?.name === "string" ? body.name.trim() : "";
    const baseUrl = typeof body?.baseUrl === "string" ? body.baseUrl.trim() : "";
    if (!name || !baseUrl) {
      return NextResponse.json(
        { error: "Name and base URL are required" },
        { status: 400 }
      );
    }
    const backend =
      typeof body?.backend === "string" && BACKENDS.includes(body.backend)
        ? body.backend
        : "lawin-v2";
    const features = Array.isArray(body?.features)
      ? body.features.filter(
          (f: unknown): f is string => typeof f === "string" && FEATURES.includes(f)
        )
      : [];
    const server = await db.serverProfile.create({
      data: { name, baseUrl, backend, features: features.join(",") },
    });
    return NextResponse.json({
      id: server.id,
      name: server.name,
      baseUrl: server.baseUrl,
      backend: server.backend,
      features: parseFeatures(server.features),
    });
  } catch (err) {
    console.error("POST /api/servers failed", err);
    return NextResponse.json({ error: "Failed to create server" }, { status: 500 });
  }
}

export async function PATCH(req: Request) {
  try {
    const body = await req.json();
    if (typeof body?.id !== "string") {
      return NextResponse.json({ error: "id is required" }, { status: 400 });
    }
    const patch: Record<string, unknown> = {};
    if (typeof body.name === "string" && body.name.trim()) patch.name = body.name.trim();
    if (typeof body.baseUrl === "string" && body.baseUrl.trim()) patch.baseUrl = body.baseUrl.trim();
    if (typeof body.backend === "string" && BACKENDS.includes(body.backend)) patch.backend = body.backend;
    if (Array.isArray(body.features)) {
      const features = body.features.filter(
        (f: unknown): f is string => typeof f === "string" && FEATURES.includes(f)
      );
      patch.features = features.join(",");
    }
    const server = await db.serverProfile.update({ where: { id: body.id }, data: patch });
    return NextResponse.json({
      id: server.id,
      name: server.name,
      baseUrl: server.baseUrl,
      backend: server.backend,
      features: parseFeatures(server.features),
    });
  } catch (err) {
    console.error("PATCH /api/servers failed", err);
    return NextResponse.json({ error: "Failed to update server" }, { status: 500 });
  }
}

export async function DELETE(req: Request) {
  try {
    const { searchParams } = new URL(req.url);
    const id = searchParams.get("id");
    if (!id) {
      return NextResponse.json({ error: "Query param id is required" }, { status: 400 });
    }
    await db.serverProfile.delete({ where: { id } });
    return NextResponse.json({ ok: true });
  } catch (err) {
    console.error("DELETE /api/servers failed", err);
    return NextResponse.json({ error: "Failed to delete server" }, { status: 500 });
  }
}
