// ---------------------------------------------------------------------------
// Owen Control Room — server-side GitHub Actions client.
//
// Lets the site (and the agent) start / watch / stop the "Owen Emulator
// Control Room" workflow on the Owen-Launcher repo without any manual steps.
// The token is read from the local git remote (no extra secrets to manage).
// ---------------------------------------------------------------------------
import { readFileSync } from 'fs';
import path from 'path';

export const GH_REPO = {
  owner: 'abdallh-rgx',
  name: 'Owen-Launcher',
  /** the workflow lives on (and the repo's default branch is) rust-rewrite */
  ref: 'rust-rewrite',
  workflow: 'emulator-control.yml',
} as const;

const API = 'https://api.github.com';

let cachedToken: string | null | undefined;

/** Parse the GitHub token out of .git/config (remote "owen"). */
export function ghToken(): string | null {
  if (cachedToken !== undefined) return cachedToken;
  cachedToken = null;
  try {
    const cfg = readFileSync(path.join(process.cwd(), '.git', 'config'), 'utf8');
    const m = cfg.match(/url\s*=\s*https:\/\/([^@\s]+)@github\.com\/abdallh-rgx\/Owen-Launcher\.git/);
    if (m && m[1]) cachedToken = decodeURIComponent(m[1]);
  } catch {
    /* no .git/config — fall back to env */
  }
  if (!cachedToken && process.env.OWEN_GH_TOKEN) {
    cachedToken = process.env.OWEN_GH_TOKEN;
  }
  return cachedToken;
}

async function ghFetch(
  apiPath: string,
  init: RequestInit = {},
): Promise<{ ok: boolean; status: number; body: unknown }> {
  const token = ghToken();
  if (!token) throw new Error('GitHub token غير متوفر (تحقق من remote owen)');
  const res = await fetch(`${API}${apiPath}`, {
    ...init,
    headers: {
      Authorization: `Bearer ${token}`,
      Accept: 'application/vnd.github+json',
      'X-GitHub-Api-Version': '2022-11-28',
      'User-Agent': 'owen-control-room',
      'Content-Type': 'application/json',
      ...(init.headers ?? {}),
    },
    signal: AbortSignal.timeout(12_000),
  });
  const body = await res.json().catch(() => null);
  return { ok: res.ok, status: res.status, body };
}

export interface LaunchInput {
  controlUrl: string;
  hours: number;
  apkPath?: string;
  dumpApk?: string;
}

/** workflow_dispatch the control-room workflow. 204 on success. */
export async function launchRun(input: LaunchInput): Promise<void> {
  const { ok, status, body } = await ghFetch(
    `/repos/${GH_REPO.owner}/${GH_REPO.name}/actions/workflows/${GH_REPO.workflow}/dispatches`,
    {
      method: 'POST',
      body: JSON.stringify({
        ref: GH_REPO.ref,
        inputs: {
          control_url: input.controlUrl,
          control_token: process.env.CONTROL_TOKEN || 'owen-ctl-7f4k9qz2m8',
          hours: String(input.hours),
          ...(input.apkPath ? { apk_path: input.apkPath } : {}),
          ...(input.dumpApk ? { dump_apk: input.dumpApk } : {}),
        },
      }),
    },
  );
  if (!ok) {
    const msg = (body as { message?: string } | null)?.message ?? `HTTP ${status}`;
    throw new Error(`GitHub: ${msg}`);
  }
}

export interface RunStatus {
  id: number;
  number: number;
  status: string;
  conclusion: string | null;
  url: string;
  htmlUrl: string;
  createdAt: string;
  updatedAt: string;
  headBranch: string;
  event: string;
}

let runCache: { at: number; run: RunStatus | null } = { at: 0, run: null };

export async function latestRun(force = false): Promise<RunStatus | null> {
  if (!force && Date.now() - runCache.at < 5_000) return runCache.run;
  const { ok, body } = await ghFetch(
    `/repos/${GH_REPO.owner}/${GH_REPO.name}/actions/workflows/${GH_REPO.workflow}/runs?per_page=1`,
  );
  if (!ok || !body) return runCache.run;
  const runs = (body as { workflow_runs?: Array<Record<string, unknown>> })
    .workflow_runs;
  if (!runs || runs.length === 0) {
    runCache = { at: Date.now(), run: null };
    return null;
  }
  const r = runs[0];
  const run: RunStatus = {
    id: Number(r.id),
    number: Number(r.run_number),
    status: String(r.status ?? ''),
    conclusion: (r.conclusion as string | null) ?? null,
    url: String(r.url ?? ''),
    htmlUrl: String(r.html_url ?? ''),
    createdAt: String(r.created_at ?? ''),
    updatedAt: String(r.updated_at ?? ''),
    headBranch: String(r.head_branch ?? ''),
    event: String(r.event ?? ''),
  };
  runCache = { at: Date.now(), run };
  return run;
}

export async function cancelRun(runId: number): Promise<void> {
  const { ok, body } = await ghFetch(
    `/repos/${GH_REPO.owner}/${GH_REPO.name}/actions/runs/${runId}/cancel`,
    { method: 'POST' },
  );
  if (!ok && ok !== false) return;
  if (!ok) {
    const msg = (body as { message?: string } | null)?.message;
    // 409 conflict = already completed → treat as success
    if (!/conflict/i.test(msg ?? '')) throw new Error(msg ?? 'فشل الإيقاف');
  }
}
