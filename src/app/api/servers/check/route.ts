import { NextResponse } from "next/server";
import { db } from "@/lib/db";

export const runtime = "nodejs";

/**
 * POST /api/servers/check { id }
 * Server-side health check: fetches
 * `${baseUrl}/fortnite/api/lightswitch/api/service/bulk/status`
 * with a 5 second timeout. Failures are caught and reported gracefully.
 */
export async function POST(req: Request) {
  try {
    const body = await req.json().catch(() => ({}));
    if (typeof body?.id !== "string") {
      return NextResponse.json({ error: "id is required" }, { status: 400 });
    }
    const server = await db.serverProfile.findUnique({ where: { id: body.id } });
    if (!server) {
      return NextResponse.json({ error: "Server not found" }, { status: 404 });
    }

    const base = server.baseUrl.replace(/\/+$/, "");
    const url = `${base}/fortnite/api/lightswitch/api/service/bulk/status`;
    const started = Date.now();
    const controller = new AbortController();
    const timer = setTimeout(() => controller.abort(), 5000);
    try {
      const res = await fetch(url, {
        signal: controller.signal,
        headers: { "User-Agent": "Owen-Launcher-Console/1.0" },
        cache: "no-store",
      });
      const elapsedMs = Date.now() - started;
      let preview = "";
      try {
        const text = await res.text();
        preview = text.slice(0, 200);
      } catch {
        preview = "";
      }
      return NextResponse.json({
        ok: res.ok,
        status: res.status,
        statusText: res.statusText,
        elapsedMs,
        url,
        preview,
      });
    } finally {
      clearTimeout(timer);
    }
  } catch (err) {
    const aborted = err instanceof Error && err.name === "AbortError";
    console.error("POST /api/servers/check failed", err);
    return NextResponse.json(
      {
        ok: false,
        error: aborted ? "Timed out after 5 seconds" : "Connection failed",
        detail: err instanceof Error ? err.message : String(err),
      },
      { status: 502 }
    );
  }
}
