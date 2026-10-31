// GET /api/control/signals/pending — the driver drains PENDING signals
// (oldest first) and acks them one by one.
import { db } from '@/lib/db';
import { checkToken, unauthorized } from '@/lib/control';

export const dynamic = 'force-dynamic';

export async function GET(req: Request) {
  if (!checkToken(req)) return unauthorized();
  try {
    const signals = await db.controlSignal.findMany({
      where: { status: 'PENDING' },
      orderBy: { seq: 'asc' },
      take: 10,
    });
    return Response.json(
      { signals },
      { headers: { 'Cache-Control': 'no-store' } },
    );
  } catch {
    return Response.json({ signals: [] });
  }
}
