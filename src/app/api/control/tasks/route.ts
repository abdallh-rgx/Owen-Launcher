// POST /api/control/tasks — the driver registers a new annotation request
// (screenshot + instruction). Token-protected.
import { db } from '@/lib/db';
import {
  badRequest,
  checkToken,
  jsonError,
  nextSeq,
  saveShot,
  unauthorized,
} from '@/lib/control';

export const dynamic = 'force-dynamic';

export async function POST(req: Request) {
  if (!checkToken(req)) return unauthorized();
  try {
    const body = (await req.json()) as {
      stepKey?: string;
      title?: string;
      instruction?: string;
      image?: string;
      meta?: unknown;
    };

    const image = typeof body.image === 'string' ? body.image : '';
    if (!image) return badRequest('image (base64 PNG) is required');
    if (!body.title || !body.instruction) {
      return badRequest('title and instruction are required');
    }

    const shot = await saveShot(image);

    const task = await db.$transaction(async (tx) => {
      const seq = await nextSeq(tx);
      return tx.controlTask.create({
        data: {
          seq,
          stepKey: (body.stepKey || `step-${seq}`).slice(0, 120),
          title: body.title.slice(0, 200),
          instruction: body.instruction.slice(0, 2000),
          imageName: shot.name,
          imageW: shot.w,
          imageH: shot.h,
          meta:
            body.meta === undefined
              ? null
              : JSON.stringify(body.meta).slice(0, 4000),
        },
      });
    });

    return Response.json({ ok: true, id: task.id, seq: task.seq });
  } catch (e) {
    return jsonError(e);
  }
}
