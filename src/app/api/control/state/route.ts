// GET /api/control/state — one-shot snapshot for the Control Room UI.
import { db } from '@/lib/db';
import { CONTROL_TOKEN, taskDTO } from '@/lib/control';
import { recordOrigin } from '@/lib/live-store';
import type { SignalDTO, SignalType, StateDTO } from '@/lib/control-types';

export const dynamic = 'force-dynamic';

export async function GET(req: Request) {
  recordOrigin(req);
  try {
    const [hb, pending, answered, applied, cancelled, failed, total, signals] =
      await Promise.all([
        db.controlHeartbeat.findUnique({ where: { id: 1 } }),
        db.controlTask.count({ where: { status: 'PENDING' } }),
        db.controlTask.count({ where: { status: 'ANSWERED' } }),
        db.controlTask.count({ where: { status: 'APPLIED' } }),
        db.controlTask.count({ where: { status: 'CANCELLED' } }),
        db.controlTask.count({ where: { status: 'FAILED' } }),
        db.controlTask.count(),
        db.controlSignal.findMany({ orderBy: { seq: 'desc' }, take: 12 }),
      ]);

    const pendings = await db.controlTask.findMany({
      where: { status: 'PENDING' },
      orderBy: [{ seq: 'asc' }],
    });

    const recent = await db.controlTask.findMany({
      orderBy: [{ seq: 'desc' }],
      take: 15,
    });

    const signalDTOs: SignalDTO[] = signals.reverse().map((s) => ({
      id: s.id,
      seq: s.seq,
      type: s.type as SignalType,
      note: s.note,
      status: s.status as 'PENDING' | 'ACK',
      result: s.result,
      createdAt: s.createdAt.toISOString(),
      ackedAt: s.ackedAt ? s.ackedAt.toISOString() : null,
    }));

    const state: StateDTO = {
      serverTime: new Date().toISOString(),
      token: CONTROL_TOKEN,
      heartbeat: hb
        ? {
            state: hb.state,
            phase: hb.phase,
            device: hb.device,
            message: hb.message,
            logTail: hb.logTail,
            updatedAt: hb.updatedAt.toISOString(),
          }
        : null,
      stats: { pending, answered, applied, cancelled, failed, total },
      current: pendings.length ? taskDTO(pendings[0]) : null,
      queue: pendings.slice(1).map(taskDTO),
      recent: recent.map(taskDTO),
      signals: signalDTOs,
    };

    return Response.json(state, {
      headers: { 'Cache-Control': 'no-store' },
    });
  } catch (e) {
    return Response.json(
      { error: e instanceof Error ? e.message : 'state failed' },
      { status: 500 },
    );
  }
}
