// POST /api/control/tasks/[id]/retake — the human asks the driver for a
// fresh screenshot (screen moved / stale frame). Public.
import { db } from '@/lib/db';
import { badRequest, jsonError, taskDTO } from '@/lib/control';

export const dynamic = 'force-dynamic';

export async function POST(
  _req: Request,
  ctx: { params: Promise<{ id: string }> },
) {
  try {
    const { id } = await ctx.params;
    const task = await db.controlTask.findUnique({ where: { id } });
    if (!task) return badRequest('task not found');
    if (task.status !== 'PENDING') {
      return badRequest(`task already ${task.status}`);
    }

    const updated = await db.controlTask.update({
      where: { id },
      data: { wantsRetake: true },
    });

    return Response.json({ ok: true, task: taskDTO(updated) });
  } catch (e) {
    return jsonError(e);
  }
}
