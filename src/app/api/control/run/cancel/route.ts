// POST /api/control/run/cancel — stop the in-progress control-room run.
import { jsonError } from '@/lib/control';
import { cancelRun, latestRun } from '@/lib/gh';

export const dynamic = 'force-dynamic';

export async function POST() {
  try {
    const run = await latestRun(true);
    if (!run) return Response.json({ ok: false, error: 'لا يوجد تشغيل' }, { status: 404 });
    if (run.status !== 'in_progress' && run.status !== 'queued' && run.status !== 'waiting') {
      return Response.json(
        { ok: false, error: 'التشغيل منتهٍ أصلاً' },
        { status: 409 },
      );
    }
    await cancelRun(run.id);
    return Response.json({ ok: true, cancelled: run.number });
  } catch (e) {
    return jsonError(e);
  }
}
