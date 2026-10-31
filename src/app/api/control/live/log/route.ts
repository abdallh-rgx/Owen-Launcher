// POST /api/control/live/log — the driver pushes a merged log batch
// (logcat lines + its own [DRIVER] lines + crash buffer lines).
// Body: { lines: [{ level, src, text }] }
import { badRequest, checkToken, jsonError, unauthorized } from '@/lib/control';
import { pushLogs } from '@/lib/live-store';

export const dynamic = 'force-dynamic';

export async function POST(req: Request) {
  if (!checkToken(req)) return unauthorized();
  try {
    const body = (await req.json()) as {
      lines?: Array<{ level?: string; src?: string; text?: string }>;
    };
    const lines = Array.isArray(body.lines) ? body.lines.slice(0, 200) : [];
    if (lines.length === 0) return badRequest('lines[] is required');
    const added = pushLogs(
      lines.map((l) => ({ level: l.level, src: l.src, text: String(l.text ?? '') })),
    );
    return Response.json({ ok: true, added });
  } catch (e) {
    return jsonError(e);
  }
}
