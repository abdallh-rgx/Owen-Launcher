// POST /api/control/signals/[id]/ack — the driver confirms a signal and
// reports what it did with it (shown live on the site).
import { db } from '@/lib/db';
import { checkToken, jsonError, unauthorized } from '@/lib/control';
import { liveStore, notifyLive } from '@/lib/live-store';

export const dynamic = 'force-dynamic';

export async function POST(
  req: Request,
  ctx: { params: Promise<{ id: string }> },
) {
  if (!checkToken(req)) return unauthorized();
  try {
    const { id } = await ctx.params;
    const body = (await req.json().catch(() => ({}))) as { result?: string };
    const updated = await db.controlSignal.update({
      where: { id },
      data: {
        status: 'ACK',
        result: String(body.result ?? '').slice(0, 500) || 'تم الاستلام',
        ackedAt: new Date(),
      },
    });
    liveStore.signalsVersion += 1;
    notifyLive();
    return Response.json({ ok: true, id: updated.id });
  } catch (e) {
    return jsonError(e);
  }
}
