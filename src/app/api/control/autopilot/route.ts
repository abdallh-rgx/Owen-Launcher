// GET  /api/control/autopilot — حالة الطيار الآلي.
// POST /api/control/autopilot — تحكم: { enabled?, autoSdk?, autoRestart?,
//                                humanGraceMs?, resetEvidence?, forceSignal? }
//
// المستخدم قال "افعل كل شيء بنفسك" — لذا الطيار مُفعّل افتراضياً، وهنا
// مجرد مفتاح طوارئ + إعادة ضبط الأدلة + دفع إشارة يدوية عند الحاجة.
import { ensureAutopilot, autopilotCommand, autopilotDTO } from '@/lib/autopilot';
import { jsonError } from '@/lib/control';

export const dynamic = 'force-dynamic';

export async function GET() {
  ensureAutopilot();
  try {
    return Response.json(
      { ok: true, autopilot: autopilotDTO() },
      { headers: { 'Cache-Control': 'no-store' } },
    );
  } catch (e) {
    return jsonError(e);
  }
}

export async function POST(req: Request) {
  ensureAutopilot();
  try {
    const body = (await req.json().catch(() => ({}))) as {
      enabled?: boolean;
      autoSdk?: boolean;
      autoRestart?: boolean;
      humanGraceMs?: number;
      resetEvidence?: boolean;
      forceSignal?: 'download_done' | 'start_sdk' | 'restart_app' | 'crash_report';
    };
    const autopilot = await autopilotCommand(body);
    return Response.json(
      { ok: true, autopilot },
      { headers: { 'Cache-Control': 'no-store' } },
    );
  } catch (e) {
    return jsonError(e);
  }
}
