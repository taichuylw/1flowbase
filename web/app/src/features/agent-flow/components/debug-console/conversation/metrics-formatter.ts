function formatScaledValue(value: number): string {
  const rounded = Math.round(value * 100) / 100;
  return Number(rounded.toFixed(2)).toString();
}

function formatScaledTokensValue(value: number): string {
  const truncated = Math.floor(value * 100) / 100;
  return Number(truncated.toFixed(2)).toString();
}

export function formatTokens(tokens: number): string {
  const absTokens = Math.abs(tokens);
  const sign = tokens < 0 ? '-' : '';

  if (absTokens >= 1_000_000_000) {
    return `${sign}${formatScaledTokensValue(absTokens / 1_000_000_000)} B`;
  }
  if (absTokens >= 1_000_000) {
    return `${sign}${formatScaledTokensValue(absTokens / 1_000_000)} M`;
  }
  if (absTokens >= 1_000) {
    return `${sign}${formatScaledTokensValue(absTokens / 1_000)} K`;
  }
  return `${sign}${absTokens}`;
}

export function formatTokenDelta(tokenDelta: number): string {
  if (tokenDelta >= 0) {
    return `+${formatTokens(tokenDelta)}`;
  }
  return formatTokens(tokenDelta);
}

export function formatDurationScaled(durationMs: number): string {
  if (durationMs < 1000) {
    return `${durationMs} ms`;
  }

  const seconds = durationMs / 1000;
  if (seconds < 60) {
    return `${formatScaledValue(seconds)} s`;
  }

  const minutes = seconds / 60;
  if (minutes < 60) {
    return `${formatScaledValue(minutes)} m`;
  }

  const hours = minutes / 60;
  if (hours < 24) {
    return `${formatScaledValue(hours)} h`;
  }

  const days = hours / 24;
  return `${formatScaledValue(days)} d`;
}
