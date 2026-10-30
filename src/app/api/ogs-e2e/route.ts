import { readdir, readFile, stat } from "fs/promises";
import { NextRequest, NextResponse } from "next/server";

/* /api/ogs-e2e — the OGS 12.41 on-Android evidence feed.
 *
 * The `ogs-1241-e2e` workflow (dump → build → play) commits its evidence to
 * docs/e2e/12.41/ — lobby.png, match.png, status.json, logcat.txt, server.log.
 * This route reads them from the repo checkout so the console renders the
 * REAL emulator screenshots (no mockups, ever):
 *
 *   GET /api/ogs-e2e                  → { available, status, files, updated }
 *   GET /api/ogs-e2e?image=lobby.png  → the PNG (raw bytes)
 *   GET /api/ogs-e2e?file=server.log  → the text log (capped at 256 KiB)
 */

const DIR = "docs/e2e/12.41";
const IMAGES = new Set(["lobby.png", "match.png"]);
const TEXTS = new Set(["server.log", "logcat.txt", "status.json"]);
const MAX_TEXT = 256 * 1024;

export async function GET(req: NextRequest) {
  const image = req.nextUrl.searchParams.get("image");
  const file = req.nextUrl.searchParams.get("file");

  if (image) {
    if (!IMAGES.has(image)) return NextResponse.json({ error: "not an evidence image" }, { status: 400 });
    try {
      const buf = await readFile(`${DIR}/${image}`);
      return new NextResponse(new Uint8Array(buf), {
        headers: { "content-type": "image/png", "cache-control": "no-store" },
      });
    } catch {
      return NextResponse.json({ error: "not yet" }, { status: 404 });
    }
  }

  if (file) {
    if (!TEXTS.has(file)) return NextResponse.json({ error: "not an evidence file" }, { status: 400 });
    try {
      let buf = await readFile(`${DIR}/${file}`);
      if (buf.length > MAX_TEXT) buf = buf.subarray(buf.length - MAX_TEXT);
      return new NextResponse(new Uint8Array(buf), {
        headers: {
          "content-type": file.endsWith(".json") ? "application/json" : "text/plain; charset=utf-8",
          "cache-control": "no-store",
        },
      });
    } catch {
      return NextResponse.json({ error: "not yet" }, { status: 404 });
    }
  }

  // summary
  const files: Record<string, number> = {};
  let status: unknown = null;
  let updated: string | null = null;
  try {
    const entries = await readdir(DIR);
    for (const name of entries) {
      if (name === ".gitkeep") continue;
      try {
        const s = await stat(`${DIR}/${name}`);
        files[name] = s.size;
        if (!updated || s.mtime > new Date(updated)) updated = s.mtime.toISOString();
      } catch {
        /* ignore */
      }
    }
    if (files["status.json"]) {
      status = JSON.parse(await readFile(`${DIR}/status.json`, "utf8"));
    }
  } catch {
    /* dir missing — evidence not landed yet */
  }
  return NextResponse.json(
    {
      available: Object.keys(files).length > 0,
      files,
      status,
      updated,
      note: "OGS 12.41 → Android: MobileDumper-7 SDK dump → libOwenOGS build → shim-injected APK → emulator lobby + match",
    },
    { headers: { "cache-control": "no-store" } },
  );
}
