// GET /api/control/shot/[name] — serves a screenshot PNG from
// data/control/shots (name strictly sanitized; no traversal).
import { createReadStream, existsSync, statSync } from 'fs';
import path from 'path';
import { SHOTS_DIR, safeShotName } from '@/lib/control';

export const dynamic = 'force-dynamic';

type Ctx = { params: Promise<{ name: string }> };

export async function GET(_req: Request, ctx: Ctx) {
  const { name } = await ctx.params;
  const safe = safeShotName(decodeURIComponent(name));
  if (!safe) return new Response('bad name', { status: 400 });

  const file = path.join(SHOTS_DIR, safe);
  if (!file.startsWith(SHOTS_DIR) || !existsSync(file)) {
    return new Response('not found', { status: 404 });
  }

  const stat = statSync(file);
  const stream = createReadStream(file);
  return new Response(stream as unknown as ReadableStream, {
    headers: {
      'Content-Type': 'image/png',
      'Content-Length': String(stat.size),
      // The UI cache-busts with ?v=<imageVersion>, so a little caching is OK.
      'Cache-Control': 'public, max-age=120',
    },
  });
}
