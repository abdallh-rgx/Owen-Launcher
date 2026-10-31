// POST /api/control/heartbeat — the runner refreshes its liveness record
// every ~20s (state, phase, device, message, logcat tail). Token-protected.
import { db } from '@/lib/db';
import { checkToken, jsonError, unauthorized } from '@/lib/control';

export const dynamic = 'force-dynamic';

export async function POST(req: Request) {
  if (!checkToken(req)) return unauthorized();
  try {
    const body = (await req.json()) as {
      state?: string;
      phase?: string;
      device?: string;
      message?: string;
      logTail?: string;
    };

    const data = {
      state: (body.state || 'RUN').slice(0, 60),
      phase: (body.phase || '').slice(0, 200),
      device: (body.device || '').slice(0, 200),
      message: (body.message || '').slice(0, 500),
      logTail: (body.logTail || '').slice(-6000),
    };

    const hb = await db.controlHeartbeat.upsert({
      where: { id: 1 },
      update: data,
      create: { id: 1, ...data },
    });

    return Response.json({
      ok: true,
      updatedAt: hb.updatedAt.toISOString(),
    });
  } catch (e) {
    return jsonError(e);
  }
}
