// GET /api/control/live — long-poll the live channel.
//
// ?frameSince=<seq>&logSince=<seq>&wait=1
// Holds the request up to 12s; responds the moment a new frame / log batch /
// signal lands. The browser re-polls immediately → near-realtime stream.
import {
  liveStore,
  recordOrigin,
  waitForChange,
} from '@/lib/live-store';
import type { LiveFrame, LiveLogLine } from '@/lib/live-store';

export const dynamic = 'force-dynamic';
export const maxDuration = 30;

const WAIT_MS = 12_000;

export async function GET(req: Request) {
  recordOrigin(req);
  try {
    const url = new URL(req.url);
    const frameSince = Number(url.searchParams.get('frameSince') ?? '0') || 0;
    const logSince = Number(url.searchParams.get('logSince') ?? '0') || 0;
    const wait = url.searchParams.get('wait') === '1';

    const hasNews = () =>
      liveStore.frameSeq > frameSince || liveStore.logSeq > logSince;

    if (wait && !hasNews()) {
      await waitForChange(WAIT_MS);
    }

    const frame: LiveFrame | null =
      liveStore.frame && liveStore.frame.seq > frameSince ? liveStore.frame : null;
    const logs: LiveLogLine[] = liveStore.logs.filter((l) => l.seq > logSince);

    return Response.json(
      {
        serverTime: Date.now(),
        frame,
        frameSeq: liveStore.frameSeq,
        logs,
        logSeq: liveStore.logSeq,
        signalsVersion: liveStore.signalsVersion,
        lastFrameAt: liveStore.frame?.ts ?? null,
      },
      { headers: { 'Cache-Control': 'no-store' } },
    );
  } catch (e) {
    return Response.json(
      { error: e instanceof Error ? e.message : 'live poll failed' },
      { status: 500 },
    );
  }
}
