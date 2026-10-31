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

// ---------------------------------------------------------------------------
// Live channel (screen stream + merged log) + manual signals + GitHub run
// ---------------------------------------------------------------------------

export interface LiveFrameDTO {
  seq: number;
  dataUrl: string;
  w: number;
  h: number;
  deviceW: number;
  deviceH: number;
  app: string;
  ts: number;
}

export type LiveLogLevel = 'INFO' | 'WARN' | 'ERROR' | 'CRASH' | 'DRIVER';
export type LiveLogSource = 'logcat' | 'driver' | 'crash' | 'signal';

export interface LiveLogLineDTO {
  seq: number;
  ts: number;
  level: LiveLogLevel;
  src: LiveLogSource;
  text: string;
}

export type SignalType =
  | 'download_done'
  | 'start_sdk'
  | 'restart_app'
  | 'crash_report'
  | 'note';

export interface SignalDTO {
  id: string;
  seq: number;
  type: SignalType;
  note: string;
  status: 'PENDING' | 'ACK';
  result: string | null;
  createdAt: string;
  ackedAt: string | null;
}

export const SIGNAL_LABELS_AR: Record<SignalType, string> = {
  download_done: 'انتهى التحميل الداخلي',
  start_sdk: 'ابدأ توليد SDK',
  restart_app: 'إعادة فتح التطبيق',
  crash_report: 'بلّغ عن كراش',
  note: 'ملاحظة',
};

export interface RunStatusDTO {
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
  signals: SignalDTO[];
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
