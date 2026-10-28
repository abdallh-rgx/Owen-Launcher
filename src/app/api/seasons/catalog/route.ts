import { NextResponse } from "next/server";
import { readFile } from "fs/promises";
import path from "path";

export const runtime = "nodejs";

const CATALOG_PATH = path.join(
  process.cwd(),
  "data",
  "owen",
  "battlepass-catalog-21plus.json"
);

/**
 * GET /api/seasons/catalog
 *
 * Serves the generated storefront catalog entries (BRSeason21Plus) that pair
 * with the Season21+ battle pass files. Merge these entries into the
 * catalog.json your backend serves so in-game battle pass purchases resolve
 * to the matching offerIds.
 */
export async function GET() {
  try {
    const text = await readFile(CATALOG_PATH, "utf-8");
    return new NextResponse(text, {
      status: 200,
      headers: {
        "Content-Type": "application/json",
        "Content-Disposition": 'attachment; filename="battlepass-catalog-21plus.json"',
      },
    });
  } catch {
    return NextResponse.json(
      { error: "Catalog file not generated yet" },
      { status: 404 }
    );
  }
}
