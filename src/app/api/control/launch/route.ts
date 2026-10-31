// POST /api/control/launch — start the GitHub Actions control-room session.
//
// The control_url is resolved AUTOMATICALLY: the browser origin of whoever
// calls this, plus every origin the operator was seen from recently, are
// PROBED (GET /api/control/state, 3s) — the first that actually answers is
// used. This matters because some preview gateways only work with session
// headers (e.g. the fcapp.run one 400s without x-session-id) while the
// space-z.ai one is publicly reachable.
// Body: { hours?: number, apkPath?: string, dumpApk?: string, controlUrl?: string }
import { badRequest, jsonError } from '@/lib/control';
import { launchRun } from '@/lib/gh';
import { liveStore, recordOrigin } from '@/lib/live-store';

export const dynamic = 'force-dynamic';

function normalizeCandidate(raw: string | undefined | null): string | null {
  if (!raw) return null;
  const c = raw.replace(/\/+$/, '');
  if (!/^https?:\/\/.+\..+/.test(c)) return null;
  if (/^https?:\/\/(localhost|127\.)/.test(c)) return null;
  return c;
}

/** GET /api/control/state with a hard timeout — is this origin usable? */
async function probeOrigin(origin: string): Promise<boolean> {
  try {
    const res = await fetch(`${origin}/api/control/state`, {
      signal: AbortSignal.timeout(3500),
      cache: 'no-store',
      headers: { 'user-agent': 'owen-launch-probe' },
    });
    return res.ok;
  } catch {
    return false;
  }
}

async function resolveControlUrl(
  req: Request,
  explicit?: string,
): Promise<{ url: string; probed: boolean } | null> {
  const headerOrigin = normalizeCandidate(req.headers.get('origin'));
  const referer = normalizeCandidate(
    req.headers.get('referer')?.match(/^https?:\/\/[^/]+/)?.[0],
  );
  const hostBased = normalizeCandidate(
    `${req.headers.get('x-forwarded-proto') ?? 'https'}://${req.headers.get('host') ?? ''}`,
  );

  // sticky winner from a previous successful launch/probe
  const sticky = liveStore.preferredOrigin;

  // all seen origins, newest first
  const seen = [...liveStore.origins.entries()]
    .sort((a, b) => b[1] - a[1])
    .map(([u]) => u);

  const candidates = [
    ...(sticky ? [sticky] : []),
    normalizeCandidate(explicit) ?? '',
    headerOrigin ?? '',
    referer ?? '',
    ...seen,
    hostBased ?? '',
  ].filter(Boolean) as string[];

  const deduped = [...new Set(candidates)];

  for (const c of deduped) {
    if (await probeOrigin(c)) {
      liveStore.preferredOrigin = c;
      return { url: c, probed: true };
    }
  }
  // nothing answered — fall back to the newest seen origin
  const fallback = deduped[deduped.length - 1] ?? null;
  return fallback ? { url: fallback, probed: false } : null;
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

    const resolved = await resolveControlUrl(req, body.controlUrl);
    if (!resolved) {
      return badRequest(
        'تعذّر تحديد control_url صالح — افتح الموقع من رابط المعاينة العام وأعد المحاولة',
      );
    }

    const hours = Math.min(5.5, Math.max(1, Number(body.hours ?? 5.5) || 5.5));

    await launchRun({
      controlUrl: resolved.url,
      hours,
      apkPath: body.apkPath?.slice(0, 300),
      dumpApk: body.dumpApk?.slice(0, 300),
    });

    return Response.json({
      ok: true,
      controlUrl: resolved.url,
      probed: resolved.probed,
      hours,
      message: resolved.probed
        ? 'تم إطلاق الجلسة على GitHub Actions — انتظر دقيقتين حتى يقلع المحاكي'
        : 'أُطلقت الجلسة لكن لم يُتحقق من الرابط (قد يحتاج x-session-id) — راقب البث',
    });
  } catch (e) {
    return jsonError(e);
  }
}
