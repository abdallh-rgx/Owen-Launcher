import { NextRequest, NextResponse } from "next/server";
import { resolveDownloadPath } from "@/lib/fndb";

export const dynamic = "force-dynamic";

const CONTENT_TYPES: Record<string, string> = {
  ".json": "application/json; charset=utf-8",
  ".md": "text/markdown; charset=utf-8",
  ".cpp": "text/plain; charset=utf-8",
  ".h": "text/plain; charset=utf-8",
};

export async function GET(req: NextRequest) {
  const group = req.nextUrl.searchParams.get("group") ?? "db";
  const file = req.nextUrl.searchParams.get("file") ?? "";
  const p = resolveDownloadPath(group, file);
  if (!p) {
    return NextResponse.json({ error: "not_found" }, { status: 404 });
  }
  const fs = await import("fs");
  const buf = fs.readFileSync(p);
  const ext = p.slice(p.lastIndexOf("."));
  const inline = req.nextUrl.searchParams.get("inline") === "1";
  return new NextResponse(new Uint8Array(buf), {
    headers: {
      "Content-Type": CONTENT_TYPES[ext] ?? "application/octet-stream",
      "Content-Disposition": `${inline ? "inline" : "attachment"}; filename="${file}"`,
      "Cache-Control": "public, max-age=3600",
    },
  });
}
