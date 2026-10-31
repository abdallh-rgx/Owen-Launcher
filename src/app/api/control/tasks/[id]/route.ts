// GET    /api/control/tasks/[id] — driver polls for the human's answer.
// PATCH  /api/control/tasks/[id] — driver applies results / uploads a fresh
//         screenshot (retake) / marks APPLIED|FAILED|CANCELLED. Token-protected.
import { db } from '@/lib/db';
import {
  badRequest,
  checkToken,
  jsonError,
  saveShot,
  taskDTO,
  unauthorized,
} from '@/lib/control';

export const dynamic = 'force-dynamic';

type Ctx = { params: Promise<{ id: string }> };

export async function GET(req: Request, ctx: Ctx) {
  if (!checkToken(req)) return unauthorized();
  try {
    const { id } = await ctx.params;
    const task = await db.controlTask.findUnique({ where: { id } });
    if (!task) return badRequest('task not found');
    return Response.json(
      { ok: true, task: taskDTO(task) },
      { headers: { 'Cache-Control': 'no-store' } },
    );
  } catch (e) {
    return jsonError(e);
  }
}

export async function PATCH(req: Request, ctx: Ctx) {
  if (!checkToken(req)) return unauthorized();
  try {
    const { id } = await ctx.params;
    const body = (await req.json()) as {
      status?: string;
      error?: string;
      image?: string;
      afterImage?: string;
      clearRetake?: boolean;
      wantsRetake?: boolean;
      note?: string;
    };

    const task = await db.controlTask.findUnique({ where: { id } });
    if (!task) return badRequest('task not found');

    const data: Record<string, unknown> = {};

    if (body.status) {
      const allowed = ['PENDING', 'ANSWERED', 'APPLIED', 'FAILED', 'CANCELLED'];
      if (!allowed.includes(body.status)) {
        return badRequest(`status must be one of ${allowed.join('|')}`);
      }
      data.status = body.status;
      if (body.status === 'APPLIED' && !task.appliedAt) {
        data.appliedAt = new Date();
      }
    }

    if (typeof body.error === 'string') data.error = body.error.slice(0, 2000);

    // Fresh screenshot for a still-pending task (screen changed while asking).
    if (typeof body.image === 'string' && body.image) {
      const shot = await saveShot(body.image);
      data.imageName = shot.name;
      data.imageW = shot.w;
      data.imageH = shot.h;
      data.imageVersion = task.imageVersion + 1;
      data.wantsRetake = false;
    } else if (body.clearRetake) {
      data.wantsRetake = false;
    }
    if (body.wantsRetake === true) data.wantsRetake = true;

    // Post-action verification screenshot.
    if (typeof body.afterImage === 'string' && body.afterImage) {
      const after = await saveShot(body.afterImage);
      data.afterName = after.name;
    }

    if (Object.keys(data).length === 0 && !body.note) {
      return badRequest('nothing to update');
    }

    const updated = await db.controlTask.update({
      where: { id },
      data,
    });

    return Response.json({ ok: true, task: taskDTO(updated) });
  } catch (e) {
    return jsonError(e);
  }
}
