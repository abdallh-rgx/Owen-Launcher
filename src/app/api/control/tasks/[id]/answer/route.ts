// POST /api/control/tasks/[id]/answer — the human submits the button
// coordinates + description + wait. Public (the browser user has no token).
import { db } from '@/lib/db';
import { badRequest, jsonError, taskDTO } from '@/lib/control';
import { CONTROL_ACTIONS, type ControlAction } from '@/lib/control-types';

export const dynamic = 'force-dynamic';

export async function POST(
  req: Request,
  ctx: { params: Promise<{ id: string }> },
) {
  try {
    const { id } = await ctx.params;
    const body = (await req.json()) as {
      action?: string;
      x?: number;
      y?: number;
      x2?: number;
      y2?: number;
      description?: string;
      waitMs?: number;
    };

    const task = await db.controlTask.findUnique({ where: { id } });
    if (!task) return badRequest('task not found');
    if (task.status !== 'PENDING') {
      return badRequest(`task already ${task.status}`);
    }

    const action = body.action as ControlAction | undefined;
    if (!action || !CONTROL_ACTIONS.includes(action)) {
      return badRequest(`action must be one of ${CONTROL_ACTIONS.join('|')}`);
    }

    const num = (v: unknown): number | null =>
      typeof v === 'number' && Number.isFinite(v) ? v : null;
    const clampCoord = (v: number | null) =>
      v === null ? null : Math.min(12000, Math.max(0, Math.round(v)));

    let x = clampCoord(num(body.x));
    let y = clampCoord(num(body.y));
    const x2 = clampCoord(num(body.x2));
    const y2 = clampCoord(num(body.y2));

    // A swipe needs both points; a tap needs one; anything else needs none.
    if (action === 'tap' && (x === null || y === null)) {
      return badRequest('tap requires x and y (click the screenshot first)');
    }
    if (action === 'swipe') {
      if (x === null || y === null || x2 === null || y2 === null) {
        return badRequest('swipe requires two points (start + end)');
      }
    }

    let waitMs = num(body.waitMs);
    if (waitMs !== null) {
      waitMs = Math.min(600_000, Math.max(0, Math.round(waitMs)));
    }

    const updated = await db.controlTask.update({
      where: { id },
      data: {
        status: 'ANSWERED',
        action,
        x: action === 'tap' || action === 'swipe' ? x : null,
        y: action === 'tap' || action === 'swipe' ? y : null,
        x2: action === 'swipe' ? x2 : null,
        y2: action === 'swipe' ? y2 : null,
        description:
          typeof body.description === 'string'
            ? body.description.slice(0, 1000)
            : null,
        waitMs,
        wantsRetake: false,
        answeredAt: new Date(),
      },
    });

    return Response.json({ ok: true, task: taskDTO(updated) });
  } catch (e) {
    return jsonError(e);
  }
}
