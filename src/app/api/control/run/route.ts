// GET /api/control/run — latest control-room workflow run status for the UI
// (and for the agent's monitoring loop). Public: contains no secrets.
import { jsonError } from '@/lib/control';
import { latestRun, ghToken } from '@/lib/gh';
import { liveStore } from '@/lib/live-store';

export const dynamic = 'force-dynamic';

export async function GET() {
  try {
    const run = await latestRun();
    return Response.json(
      {
        run,
        tokenAvailable: Boolean(ghToken()),
        lastOrigin: liveStore.lastOrigin,
        serverTime: new Date().toISOString(),
      },
      { headers: { 'Cache-Control': 'no-store' } },
    );
  } catch (e) {
    return jsonError(e);
  }
}
