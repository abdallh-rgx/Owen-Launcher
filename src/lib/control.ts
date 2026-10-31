// ---------------------------------------------------------------------------
// Owen Control Room — server-side helpers (Node runtime only)
// ---------------------------------------------------------------------------
import { randomUUID } from 'crypto';
import { existsSync } from 'fs';
import { mkdir, writeFile } from 'fs/promises';
import path from 'path';
import type { ControlAction, ControlStatus, TaskDTO, TaskMeta } from './control-types';
import type { ControlTask, Prisma } from '@prisma/client';

/** Shots live in <repo>/data/control/shots (never inside public/). */
export const SHOTS_DIR = path.join(process.cwd(), 'data', 'control', 'shots');

/** Shared secret between this site and the GitHub Actions driver. */
export const CONTROL_TOKEN =
  process.env.CONTROL_TOKEN || 'owen-ctl-7f4k9qz2m8';

/** Max accepted base64 payload (~9 MB → ~12 MB base64 chars). */
const MAX_IMAGE_B64 = 12 * 1024 * 1024;

export function checkToken(req: Request): boolean {
  const header =
    req.headers.get('x-owen-token') ||
    req.headers.get('authorization')?.replace(/^Bearer\s+/i, '') ||
    '';
  return header === CONTROL_TOKEN;
}

export function unauthorized() {
  return Response.json(
    { ok: false, error: 'unauthorized: missing/invalid x-owen-token' },
    { status: 401 },
  );
}

export function badRequest(message: string) {
  return Response.json({ ok: false, error: message }, { status: 400 });
}

/** Parse PNG IHDR → { w, h } (adb screencap -p always emits PNG). */
export function pngDims(buf: Buffer): { w: number; h: number } | null {
  const SIG = Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]);
  if (buf.length < 24 || !buf.subarray(0, 8).equals(SIG)) return null;
  const w = buf.readUInt32BE(16);
  const h = buf.readUInt32BE(20);
  if (!w || !h || w > 10000 || h > 10000) return null;
  return { w, h };
}

export interface SavedShot {
  name: string;
  w: number;
  h: number;
}

/** Decode base64 PNG + persist it under SHOTS_DIR, return its file name. */
export async function saveShot(base64: string): Promise<SavedShot> {
  const b64 = base64.replace(/^data:image\/\w+;base64,/, '').trim();
  if (!b64 || b64.length > MAX_IMAGE_B64) {
    throw new Error('image payload missing or too large (>12MB base64)');
  }
  const buf = Buffer.from(b64, 'base64');
  const dims = pngDims(buf);
  if (!dims) throw new Error('image is not a valid PNG');
  if (!existsSync(SHOTS_DIR)) await mkdir(SHOTS_DIR, { recursive: true });
  const name = `${Date.now()}-${randomUUID().slice(0, 8)}.png`;
  await writeFile(path.join(SHOTS_DIR, name), buf);
  return { name, w: dims.w, h: dims.h };
}

/** Strict file-name sanitizer for the shot-serving route. */
export function safeShotName(name: string): string | null {
  if (!/^[A-Za-z0-9_-]{1,120}\.png$/.test(name)) return null;
  return name;
}

function parseMeta(raw: string | null | undefined): TaskMeta | null {
  if (!raw) return null;
  try {
    return JSON.parse(raw) as TaskMeta;
  } catch {
    return null;
  }
}

/** DB row → client DTO (image bytes stay on disk; only URLs travel). */
export function taskDTO(t: ControlTask): TaskDTO {
  return {
    id: t.id,
    seq: t.seq,
    stepKey: t.stepKey,
    title: t.title,
    instruction: t.instruction,
    status: t.status as ControlStatus,
    action: (t.action as ControlAction | null) ?? null,
    x: t.x,
    y: t.y,
    x2: t.x2,
    y2: t.y2,
    description: t.description,
    waitMs: t.waitMs,
    imageUrl: t.imageName
      ? `/api/control/shot/${t.imageName}?v=${t.imageVersion}`
      : null,
    afterImageUrl: t.afterName ? `/api/control/shot/${t.afterName}?v=1` : null,
    imageW: t.imageW,
    imageH: t.imageH,
    imageVersion: t.imageVersion,
    wantsRetake: t.wantsRetake,
    meta: parseMeta(t.meta),
    error: t.error,
    createdAt: t.createdAt.toISOString(),
    answeredAt: t.answeredAt ? t.answeredAt.toISOString() : null,
    appliedAt: t.appliedAt ? t.appliedAt.toISOString() : null,
  };
}

/** Next seq = max(seq) + 1 (SQLite has no autoincrement on non-id columns). */
export async function nextSeq(
  tx: Prisma.TransactionClient,
): Promise<number> {
  const last = await tx.controlTask.aggregate({ _max: { seq: true } });
  return (last._max.seq ?? 0) + 1;
}

export function jsonError(e: unknown) {
  const message = e instanceof Error ? e.message : String(e);
  return Response.json({ ok: false, error: message }, { status: 500 });
}
