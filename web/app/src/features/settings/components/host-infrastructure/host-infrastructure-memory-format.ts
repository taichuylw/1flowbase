import { formatDateTime, formatTime } from '../../../../shared/i18n/format';
import { i18nText } from '../../../../shared/i18n/text';
import type { SettingsHostInfrastructureMemoryContract } from '../../api/host-infrastructure';

export function formatBytes(value: number) {
  if (value < 1024) {
    return `${value} B`;
  }
  if (value < 1024 * 1024) {
    return `${(value / 1024).toFixed(1)} KB`;
  }
  return `${(value / 1024 / 1024).toFixed(1)} MB`;
}

export function formatTtl(value: number | null) {
  if (value == null) {
    return i18nText('settings', 'auto.no_expiry');
  }
  if (value < 60) {
    return `${value}s`;
  }
  if (value < 3600) {
    return `${Math.floor(value / 60)}m ${value % 60}s`;
  }
  return `${Math.floor(value / 3600)}h ${Math.floor((value % 3600) / 60)}m`;
}

export function formatUnixTimestamp(value: number | null) {
  if (value == null) {
    return i18nText('settings', 'auto.unknown');
  }
  return formatDateTime(new Date(value * 1000));
}

export function formatUpdatedAt(value: number) {
  if (!value) {
    return i18nText('settings', 'auto.not_refreshed_yet');
  }
  return formatTime(new Date(value));
}

export function resolveCanReveal(
  pageCanManage: boolean,
  overviewCanManage: boolean | undefined,
  contract: SettingsHostInfrastructureMemoryContract | undefined
) {
  return Boolean(
    pageCanManage &&
    overviewCanManage &&
    contract?.supported &&
    contract.capabilities.reveal_value
  );
}

export function formatInspectionPath(path: string[]) {
  return path.length ? path.join(' / ') : i18nText('settings', 'auto.root');
}
