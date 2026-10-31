// POST /api/control/live/frame — the driver pushes a live screen frame.
// Body: { image: base64 (jpeg preferred), w, h, deviceW, deviceH, app }
import { badRequest, checkToken, jsonError, unauthorized } from '@/lib/control';
import { pushFrame } from '@/lib/live-store';

export const dynamic = 'force-dynamic';

const MAX_B64 = 4 * 1024 * 1024; // ~3 MB image

export async function POST(req: Request) {
  if (!checkToken(req)) return unauthorized();
  try {
    const body = (await req.json()) as {
      image?: string;
      w?: number;
      h?: number;
      deviceW?: number;
      deviceH?: number;
      app?: string;
    };
    const b64 = (body.image || '').replace(/^data:image\/\w+;base64,/, '').trim();
    if (!b64) return badRequest('image (base64) is required');
    if (b64.length > MAX_B64) return badRequest('frame too large (>4MB base64)');

    const buf = Buffer.from(b64, 'base64');
    if (buf.length < 64) return badRequest('frame payload too small');

    const mime = buf[0] === 0x89 ? 'image/png' : 'image/jpeg';
    const w = clampInt(body.w, 1, 5000, 400);
    const h = clampInt(body.h, 1, 10000, 800);
    const deviceW = clampInt(body.deviceW, 1, 5000, 1080);
    const deviceH = clampInt(body.deviceH, 1, 10000, 2280);
    const app = String(body.app ?? '').slice(0, 120);

    const frame = pushFrame(
      `data:${mime};base64,${b64}`,
      w,
      h,
      deviceW,
      deviceH,
      app,
    );
    return Response.json({ ok: true, seq: frame.seq });
  } catch (e) {
    return jsonError(e);
  }
}

function clampInt(v: unknown, min: number, max: number, dflt: number): number {
  const n = Number(v);
  if (!Number.isFinite(n)) return dflt;
  return Math.min(max, Math.max(min, Math.round(n)));
}
