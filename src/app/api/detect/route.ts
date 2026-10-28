import { NextResponse } from "next/server";
import { createWriteStream } from "fs";
import { readFile, unlink } from "fs/promises";
import { tmpdir } from "os";
import { join } from "path";
import { Readable } from "stream";
import { pipeline } from "stream/promises";
import { db } from "@/lib/db";
import { scanImage, readElfSegments, offToVa, hookOffset, type Candidate } from "@/lib/scanner";

export const runtime = "nodejs";
export const maxDuration = 300;

/** 400 MB upload cap. */
const MAX_BYTES = 400 * 1024 * 1024;

function hex(n: number | null): string | null {
  return n === null ? null : "0x" + n.toString(16).toUpperCase().padStart(8, "0");
}

function serializeCandidate(c: Candidate, va: number | null) {
  return {
    score: c.score,
    variadic: c.variadic,
    funcStart: hex(c.funcStart),
    cbzX0: hex(c.cbzOff),
    bl: hex(c.blOff),
    blTarget: hex(c.blTarget),
    movW02b: hex(c.movOff),
    gap: "0x" + c.gap.toString(16).toUpperCase(),
    va: hex(va),
  };
}

export async function POST(req: Request) {
  const tempPath = join(tmpdir(), `owen-scan-${Date.now()}.so`);
  try {
    const { searchParams } = new URL(req.url);
    const name = searchParams.get("name") || "library.so";
    const contentLength = Number(req.headers.get("content-length") || "0");
    if (contentLength > MAX_BYTES) {
      return NextResponse.json(
        { error: "File exceeds the 400 MB limit" },
        { status: 413 }
      );
    }

    if (!req.body) {
      return NextResponse.json({ error: "Empty request body" }, { status: 400 });
    }

    // Stream the raw binary body to a temp file (never buffer in memory as a
    // string) to avoid default body-size limits and buffering issues.
    let received = 0;
    const webStream = req.body as unknown as ReadableStream<Uint8Array>;
    const counter = new TransformStream<Uint8Array, Uint8Array>({
      transform(chunk, controller) {
        received += chunk.byteLength;
        if (received > MAX_BYTES) {
          controller.error(new Error("File exceeds the 400 MB limit"));
          return;
        }
        controller.enqueue(chunk);
      },
    });
    const nodeStream = Readable.fromWeb(
      webStream.pipeThrough(counter) as Parameters<typeof Readable.fromWeb>[0]
    );
    await pipeline(nodeStream, createWriteStream(tempPath));

    const buf = await readFile(tempPath);
    const data = new Uint8Array(buf.buffer, buf.byteOffset, buf.byteLength);
    if (data.length < 64) {
      return NextResponse.json(
        { error: "File is too small to be an ELF64 shared object" },
        { status: 400 }
      );
    }

    const segments = readElfSegments(data);
    if (!segments) {
      return NextResponse.json(
        { error: "Not an ELF64 little-endian shared object" },
        { status: 400 }
      );
    }

    const result = scanImage(data);
    const withVa = result.candidates.map((c) => ({
      raw: c,
      va: offToVa(segments, hookOffset(c)),
    }));
    const best = withVa.length > 0 ? withVa[0] : null;

    if (best) {
      await db.scanRecord
        .create({
          data: {
            fileName: name,
            bestVa: best.va === null ? null : "0x" + best.va.toString(16).toUpperCase(),
            score: best.raw.score,
            variadic: best.raw.variadic,
          },
        })
        .catch((err) => console.error("Failed to persist ScanRecord", err));
    }

    return NextResponse.json({
      fileName: name,
      sizeBytes: buf.length,
      elapsedMs: result.elapsedMs,
      candidates: withVa.map((x) => serializeCandidate(x.raw, x.va)),
      best: best ? serializeCandidate(best.raw, best.va) : null,
    });
  } catch (err) {
    console.error("POST /api/detect failed", err);
    const message =
      err instanceof Error && err.message.includes("400 MB")
        ? err.message
        : "Scan failed";
    return NextResponse.json({ error: message }, { status: err instanceof Error && err.message.includes("400 MB") ? 413 : 500 });
  } finally {
    await unlink(tempPath).catch(() => {});
  }
}
