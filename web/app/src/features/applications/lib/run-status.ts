const TERMINAL_RUN_STATUSES = new Set([
  'succeeded',
  'completed',
  'failed',
  'cancelled'
]);

export function isTerminalRunStatus(status: string | null | undefined) {
  return TERMINAL_RUN_STATUSES.has(status?.trim().toLowerCase() ?? '');
}

export function isActiveRunStatus(status: string | null | undefined) {
  return Boolean(status?.trim()) && !isTerminalRunStatus(status);
}
