import { NextResponse } from "next/server";

export const runtime = "nodejs";

/**
 * GET /api/partyhub-status — proxy to the partyhub-web mini-service (:8091)
 * status endpoint. Reports the Voltronite backend target, the demo account and
 * the recent graphql request log (which entries were forwarded to Voltronite
 * and which were answered by the local compatibility layer).
 */
export async function GET() {
  try {
    const res = await fetch("http://127.0.0.1:8091/__status", {
      cache: "no-store",
      signal: AbortSignal.timeout(3000),
    });
    if (!res.ok) {
      return NextResponse.json({ ok: false, error: `upstream ${res.status}` }, { status: 200 });
    }
    const data = await res.json();
    return NextResponse.json(data);
  } catch (err) {
    return NextResponse.json(
      { ok: false, error: err instanceof Error ? err.message : "unreachable" },
      { status: 200 },
    );
  }
}
