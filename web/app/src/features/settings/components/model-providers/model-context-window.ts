import { i18nText } from '../../../../shared/i18n/text';

export const MODEL_CONTEXT_WINDOW_VALIDATION_MESSAGE =
  i18nText("settings", "auto.k_4b1b72bbbf");

export const MODEL_CONTEXT_WINDOW_PRESET_OPTIONS = [
  { label: '16K', value: '16K' },
  { label: '32K', value: '32K' },
  { label: '64K', value: '64K' },
  { label: '128K', value: '128K' },
  { label: '256K', value: '256K' },
  { label: '1M', value: '1M' }
] as const;

export function parseModelContextWindowInput(input: string): {
  value: number | null;
  error: string | null;
} {
  if (input.length === 0) {
    return {
      value: null,
      error: null
    };
  }

  const trimmedInput = input.trim();
  if (trimmedInput.length === 0) {
    return {
      value: null,
      error: MODEL_CONTEXT_WINDOW_VALIDATION_MESSAGE
    };
  }

  const normalizedInput = trimmedInput.toLowerCase();

  if (/^\d+$/.test(normalizedInput)) {
    return {
      value: Number(normalizedInput),
      error: null
    };
  }

  if (/^\d+k$/.test(normalizedInput)) {
    return {
      value: Number(normalizedInput.slice(0, -1)) * 1000,
      error: null
    };
  }

  if (/^\d+m$/.test(normalizedInput)) {
    return {
      value: Number(normalizedInput.slice(0, -1)) * 1000000,
      error: null
    };
  }

  return {
    value: null,
    error: MODEL_CONTEXT_WINDOW_VALIDATION_MESSAGE
  };
}

export function formatModelContextWindowValue(value: number | null | undefined) {
  if (value === null || value === undefined) {
    return '';
  }

  if (value >= 1000000 && value % 1000000 === 0) {
    return `${value / 1000000}M`;
  }

  if (value >= 1000 && value % 1000 === 0) {
    return `${value / 1000}K`;
  }

  return String(value);
}
