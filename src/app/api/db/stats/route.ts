import { NextResponse } from "next/server";
import { getStats } from "@/lib/fndb";

export const dynamic = "force-dynamic";

export async function GET() {
  try {
    const stats = getStats();
    return NextResponse.json(stats);
  } catch (e) {
    return NextResponse.json(
      { error: "stats_unavailable", detail: String(e) },
      { status: 500 }
    );
  }
}
