// POST /api/control/launch — start the GitHub Actions control-room session.
//
// The control_url is resolved automatically (the browser origin of whoever
// calls this), so neither the operator nor the agent has to paste anything.
// Body: { hours?: number, apkPath?: string, dumpApk?: string, controlUrl?: string }
import { badRequest, jsonError } from '@/lib/control';
import { launchRun } from '@/lib/gh';
import { liveStore, recordOrigin } from '@/lib/live-store';

export const dynamic = 'force-dynamic';

function resolveControlUrl(req: Request, explicit?: string): string | null {
  const candidates = [
    explicit,
    req.headers.get('origin') ?? undefined,
    liveStore.lastOrigin && Date.now() - liveStore.lastOrigin.ts < 6 * 3600_000
      ? liveStore.lastOrigin.url
      : undefined,
    `${req.headers.get('x-forwarded-proto') ?? 'https'}://${req.headers.get('host') ?? ''}`,
  ];
  for (const c of candidates) {
    if (c && /^https?:\/\/.+\..+/.test(c) && !/^https?:\/\/localhost/.test(c)) {
      return c.replace(/\/+$/, '');
    }
  }
  return null;
}

export async function POST(req: Request) {
  recordOrigin(req);
  try {
    const body = (await req.json().catch(() => ({}))) as {
      hours?: number;
      apkPath?: string;
      dumpApk?: string;
      controlUrl?: string;
    };

    const controlUrl = resolveControlUrl(req, body.controlUrl);
    if (!controlUrl) {
      return badRequest('تعذّر تحديد control_url — افتح الموقع من رابط المعاينة العام');
    }

    const hours = Math.min(5.5, Math.max(1, Number(body.hours ?? 5.5) || 5.5));

    await launchRun({
      controlUrl,
      hours,
      apkPath: body.apkPath?.slice(0, 300),
      dumpApk: body.dumpApk?.slice(0, 300),
    });

    return Response.json({
      ok: true,
      controlUrl,
      hours,
      message: 'تم إطلاق الجلسة على GitHub Actions — انتظر دقيقة حتى يقلع المحاكي',
    });
  } catch (e) {
    return jsonError(e);
  }
}
