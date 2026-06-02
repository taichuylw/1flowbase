import { i18nText } from '../../../../shared/i18n/text';

function toDisplayErrorMessage(error: unknown): string {
  if (error instanceof Error && error.message.trim().length > 0) {
    return error.message;
  }

  return i18nText("frontstage", "auto.page_content_save_failed");
}

function requireCsrfToken(csrfToken: string | null): string {
  if (!csrfToken) {
    throw new Error('missing csrf token');
  }

  return csrfToken;
}

export { requireCsrfToken, toDisplayErrorMessage };
