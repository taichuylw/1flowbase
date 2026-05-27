const TERMINAL_RUN_STATUSES = new Set([
  'succeeded',
  'completed',
  'failed',
  'cancelled'
]);

const ACTIVE_RUN_STATUSES = new Set(['queued', 'running', 'paused']);

export function isTerminalRunStatus(status: string | null | undefined) {
  return TERMINAL_RUN_STATUSES.has(status?.trim().toLowerCase() ?? '');
}

export function isActiveRunStatus(status: string | null | undefined) {
  return ACTIVE_RUN_STATUSES.has(status?.trim().toLowerCase() ?? '');
}
