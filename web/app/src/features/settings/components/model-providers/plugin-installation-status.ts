import { i18nText } from '../../../../shared/i18n/text';

export function formatPluginAvailabilityStatus(status: string) {
  switch (status) {
    case 'available':
      return { color: 'green', label: i18nText("settings", "auto.available") };
    case 'pending_restart':
      return { color: 'gold', label: i18nText("settings", "auto.key_niiefidnpk") };
    case 'load_failed':
      return { color: 'red', label: i18nText("settings", "auto.key_pglhkebofg") };
    case 'artifact_missing':
      return { color: 'red', label: i18nText("settings", "auto.key_kfkaoleock") };
    case 'install_incomplete':
      return { color: 'orange', label: i18nText("settings", "auto.key_acgogdlbla") };
    default:
      return { color: 'default', label: i18nText("settings", "auto.key_apofkjiojp") };
  }
}
