import { i18nText } from '../../../../shared/i18n/text';

export function formatPluginAvailabilityStatus(status: string) {
  switch (status) {
    case 'available':
      return { color: 'green', label: i18nText("settings", "auto.k_e91365cf9e") };
    case 'pending_restart':
      return { color: 'gold', label: i18nText("settings", "auto.k_d884583dfa") };
    case 'load_failed':
      return { color: 'red', label: i18nText("settings", "auto.k_f6b7a41e56") };
    case 'artifact_missing':
      return { color: 'red', label: i18nText("settings", "auto.k_a5a0eb4e2a") };
    case 'install_incomplete':
      return { color: 'orange', label: i18nText("settings", "auto.k_026e63b1b0") };
    default:
      return { color: 'default', label: i18nText("settings", "auto.k_0fe5a98e9f") };
  }
}
