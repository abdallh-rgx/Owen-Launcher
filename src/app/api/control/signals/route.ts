// POST /api/control/signals — the operator sends a manual signal from the site
// (download_done | start_sdk | restart_app | crash_report | note).
// GET — recent signals (oldest→newest, last 12) for the UI.
import { db } from '@/lib/db';
import { badRequest, jsonError } from '@/lib/control';
import { liveStore, notifyLive } from '@/lib/live-store';
import { Prisma } from '@prisma/client';

export const dynamic = 'force-dynamic';

export const SIGNAL_TYPES = [
  'download_done',
  'start_sdk',
  'restart_app',
  'crash_report',
  'note',
] as const;

const MAX_PENDING = 10;

export async function GET() {
  try {
    const signals = await db.controlSignal.findMany({
      orderBy: { seq: 'desc' },
      take: 12,
    });
    return Response.json(
      { signals: signals.reverse() },
      { headers: { 'Cache-Control': 'no-store' } },
    );
  } catch (e) {
    return jsonError(e);
  }
}

export async function POST(req: Request) {
  try {
    const body = (await req.json()) as { type?: string; note?: string };
    const type = String(body.type ?? '');
    if (!(SIGNAL_TYPES as readonly string[]).includes(type)) {
      return badRequest(`type must be one of: ${SIGNAL_TYPES.join(', ')}`);
    }
    const note = String(body.note ?? '').slice(0, 500);

    const created = await db.$transaction(async (tx) => {
      // block duplicate pending signals of the same type + spam cap
      const pendingSame = await tx.controlSignal.count({
        where: { type, status: 'PENDING' },
      });
      if (pendingSame > 0 && type !== 'note') return null;
      const pendingAll = await tx.controlSignal.count({
        where: { status: 'PENDING' },
      });
      if (pendingAll >= MAX_PENDING) return null;

      const last = await tx.controlSignal.aggregate({ _max: { seq: true } });
      return tx.controlSignal.create({
        data: { seq: (last._max.seq ?? 0) + 1, type, note },
      });
    });

    if (!created) {
      return Response.json(
        { ok: false, error: 'يوجد إشارة مماثلة قيد الانتظار بالفعل' },
        { status: 409 },
      );
    }

    liveStore.signalsVersion += 1;
    notifyLive();
    return Response.json({ ok: true, id: created.id, seq: created.seq });
  } catch (e) {
    if (e instanceof Prisma.PrismaClientKnownRequestError) {
      return Response.json({ ok: false, error: 'إشارة مكررة' }, { status: 409 });
    }
    return jsonError(e);
  }
}
