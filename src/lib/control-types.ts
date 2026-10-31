// ---------------------------------------------------------------------------
// Owen Control Room — shared types (client-safe: no server imports here)
// ---------------------------------------------------------------------------

export type ControlAction =
  | 'tap'
  | 'swipe'
  | 'wait'
  | 'back'
  | 'home'
  | 'done'
  | 'skip';

export type ControlStatus =
  | 'PENDING'
  | 'ANSWERED'
  | 'APPLIED'
  | 'FAILED'
  | 'CANCELLED';

export interface TaskDTO {
  id: string;
  seq: number;
  stepKey: string;
  title: string;
  instruction: string;
  status: ControlStatus;
  action: ControlAction | null;
  x: number | null;
  y: number | null;
  x2: number | null;
  y2: number | null;
  description: string | null;
  waitMs: number | null;
  imageUrl: string | null;
  afterImageUrl: string | null;
  imageW: number | null;
  imageH: number | null;
  imageVersion: number;
  wantsRetake: boolean;
  meta: TaskMeta | null;
  error: string | null;
  createdAt: string;
  answeredAt: string | null;
  appliedAt: string | null;
}

export interface TaskMeta {
  app?: string;
  focus?: string;
  note?: string;
  uptime?: string;
  [key: string]: unknown;
}

export interface HeartbeatDTO {
  state: string;
  phase: string;
  device: string;
  message: string;
  logTail: string;
  updatedAt: string;
}

export interface ControlStats {
  pending: number;
  answered: number;
  applied: number;
  cancelled: number;
  failed: number;
  total: number;
}

export interface StateDTO {
  serverTime: string;
  token: string;
  heartbeat: HeartbeatDTO | null;
  stats: ControlStats;
  current: TaskDTO | null;
  queue: TaskDTO[];
  recent: TaskDTO[];
}

export const CONTROL_ACTIONS: ControlAction[] = [
  'tap',
  'swipe',
  'wait',
  'back',
  'home',
  'done',
  'skip',
];

export const ACTION_LABELS_AR: Record<ControlAction, string> = {
  tap: 'نقرة على الزر (Tap)',
  swipe: 'سحب (Swipe)',
  wait: 'انتظار فقط (بدون ضغط)',
  back: 'زر الرجوع (Back)',
  home: 'زر الرئيسية (Home)',
  done: 'إنهاء الجلسة ✋',
  skip: 'تخطي — لقطة جديدة',
};

/** Fresh-tasks beeper + status pill thresholds. */
export const HEARTBEAT_STALE_MS = 90_000;
export const POLL_INTERVAL_MS = 2_000;
