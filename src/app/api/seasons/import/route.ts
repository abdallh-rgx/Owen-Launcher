import { NextResponse } from "next/server";
import { db } from "@/lib/db";

export const runtime = "nodejs";

interface SeasonImportBody {
  seasonNumber?: number;
  displayName?: string;
  data?: {
    battleBundleOfferId?: string;
    battlePassOfferId?: string;
    tierOfferId?: string;
    paidRewards?: Array<Record<string, number>>;
    freeRewards?: Array<Record<string, number>>;
  };
}

export async function POST(req: Request) {
  try {
    const body = (await req.json()) as SeasonImportBody;
    const seasonNumber = Number(body?.seasonNumber);
    if (!Number.isInteger(seasonNumber) || seasonNumber < 1 || seasonNumber > 100) {
      return NextResponse.json(
        { error: "seasonNumber must be an integer between 1 and 100" },
        { status: 400 }
      );
    }
    if (!body?.data || typeof body.data !== "object") {
      return NextResponse.json(
        { error: "data is required (owen-pak JSON shape)" },
        { status: 400 }
      );
    }
    const displayName =
      typeof body.displayName === "string" && body.displayName.trim().length > 0
        ? body.displayName.trim()
        : `Season ${seasonNumber}`;

    const record = await db.seasonRecord.upsert({
      where: { seasonNumber },
      update: { displayName, dataJson: JSON.stringify(body.data) },
      create: {
        seasonNumber,
        displayName,
        dataJson: JSON.stringify(body.data),
      },
    });
    return NextResponse.json({
      id: record.id,
      seasonNumber: record.seasonNumber,
      displayName: record.displayName,
    });
  } catch (err) {
    console.error("POST /api/seasons/import failed", err);
    return NextResponse.json({ error: "Failed to import season" }, { status: 500 });
  }
}
