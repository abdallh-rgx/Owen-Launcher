import { NextResponse } from "next/server";
import { readdir, readFile } from "fs/promises";
import path from "path";
import { db } from "@/lib/db";

export const runtime = "nodejs";

interface SeasonData {
  battleBundleOfferId?: string;
  battlePassOfferId?: string;
  tierOfferId?: string;
  paidRewards?: Array<Record<string, number>>;
  freeRewards?: Array<Record<string, number>>;
}

const BATTLEPASS_DIR = path.join(process.cwd(), "data", "owen", "battlepass");

function countItems(rows: Array<Record<string, number>> | undefined): number {
  return (rows ?? []).reduce((acc, row) => acc + Object.keys(row).length, 0);
}

/**
 * Sync the SeasonRecord table from data/owen/battlepass/*.json — upserts
 * every file present (idempotent), so newly dropped/generated season files
 * appear without a manual reset.
 */
async function syncSeasons() {
  let files: string[];
  try {
    files = await readdir(BATTLEPASS_DIR);
  } catch (err) {
    console.error("Battlepass data directory unreadable", err);
    return;
  }

  for (const file of files) {
    const m = /^Season(\d+)\.json$/i.exec(file);
    if (!m) continue;
    const seasonNumber = Number(m[1]);
    try {
      const text = await readFile(path.join(BATTLEPASS_DIR, file), "utf-8");
      const data = JSON.parse(text) as SeasonData;
      await db.seasonRecord.upsert({
        where: { seasonNumber },
        update: { dataJson: text, displayName: `Season ${seasonNumber}` },
        create: {
          seasonNumber,
          displayName: `Season ${seasonNumber}`,
          dataJson: text,
        },
      });
      void data;
    } catch (err) {
      console.error(`Failed to sync ${file}`, err);
    }
  }
}

export async function GET() {
  try {
    await syncSeasons();
    const rows = await db.seasonRecord.findMany({ orderBy: { seasonNumber: "asc" } });
    return NextResponse.json(
      rows.map((r) => {
        let parsed: SeasonData = {};
        try {
          parsed = JSON.parse(r.dataJson) as SeasonData;
        } catch {
          parsed = {};
        }
        return {
          id: r.id,
          seasonNumber: r.seasonNumber,
          displayName: r.displayName,
          paidCount: countItems(parsed.paidRewards),
          freeCount: countItems(parsed.freeRewards),
          hasBundle: Boolean(parsed.battleBundleOfferId),
          updatedAt: r.updatedAt,
        };
      })
    );
  } catch (err) {
    console.error("GET /api/seasons failed", err);
    return NextResponse.json({ error: "Failed to list seasons" }, { status: 500 });
  }
}
