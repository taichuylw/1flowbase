import type { HttpRequestBodyType } from './contract';
import { parseHttpRequestUrlParts } from './url';

export interface ParsedHttpRequestCurlCommand {
  method: string;
  url: string;
  headers: Array<{ name: string; value: string }>;
  params: Array<{ name: string; value: string }>;
  body: string;
  bodyType: HttpRequestBodyType;
}

function tokenizeCurlCommand(command: string) {
  const tokens: string[] = [];
  let current = '';
  let quote: '"' | "'" | null = null;
  let escaped = false;

  for (const char of command) {
    if (escaped) {
      current += char;
      escaped = false;
      continue;
    }

    if (char === '\\') {
      escaped = true;
      continue;
    }

    if (quote) {
      if (char === quote) {
        quote = null;
      } else {
        current += char;
      }
      continue;
    }

    if (char === '"' || char === "'") {
      quote = char;
      continue;
    }

    if (/\s/.test(char)) {
      if (current.length > 0) {
        tokens.push(current);
        current = '';
      }
      continue;
    }

    current += char;
  }

  if (current.length > 0) {
    tokens.push(current);
  }

  return tokens;
}

function splitOptionValue(token: string) {
  const equalIndex = token.indexOf('=');

  return equalIndex >= 0 ? token.slice(equalIndex + 1) : null;
}

function valueAfterOption(
  tokens: string[],
  index: number
): { value: string | null; nextIndex: number } {
  const inlineValue = splitOptionValue(tokens[index]);

  if (inlineValue !== null) {
    return { value: inlineValue, nextIndex: index };
  }

  return { value: tokens[index + 1] ?? null, nextIndex: index + 1 };
}

function parseHeader(value: string) {
  const separatorIndex = value.indexOf(':');

  if (separatorIndex <= 0) {
    return null;
  }

  return {
    name: value.slice(0, separatorIndex).trim(),
    value: value.slice(separatorIndex + 1).trim()
  };
}

function inferBodyType(
  headers: Array<{ name: string; value: string }>,
  body: string
): HttpRequestBodyType {
  if (!body) {
    return 'none';
  }

  const contentType =
    headers.find((header) => header.name.toLowerCase() === 'content-type')
      ?.value ?? '';

  if (contentType.toLowerCase().includes('application/json')) {
    return 'json';
  }

  if (
    contentType.toLowerCase().includes('application/x-www-form-urlencoded')
  ) {
    return 'x-www-form-urlencoded';
  }

  return 'raw';
}

export function parseHttpRequestCurlCommand(
  command: string
): ParsedHttpRequestCurlCommand {
  const tokens = tokenizeCurlCommand(command.trim());
  const headers: ParsedHttpRequestCurlCommand['headers'] = [];
  let method = '';
  let rawUrl = '';
  let body = '';

  for (let index = tokens[0] === 'curl' ? 1 : 0; index < tokens.length; index += 1) {
    const token = tokens[index];

    if (token === '-X' || token === '--request' || token.startsWith('--request=')) {
      const next = valueAfterOption(tokens, index);
      method = next.value?.toUpperCase() ?? '';
      index = next.nextIndex;
      continue;
    }

    if (
      token === '-H' ||
      token === '--header' ||
      token.startsWith('--header=')
    ) {
      const next = valueAfterOption(tokens, index);
      const header = next.value ? parseHeader(next.value) : null;

      if (header) {
        headers.push(header);
      }
      index = next.nextIndex;
      continue;
    }

    if (
      token === '-d' ||
      token === '--data' ||
      token === '--data-raw' ||
      token === '--data-binary' ||
      token.startsWith('--data=') ||
      token.startsWith('--data-raw=') ||
      token.startsWith('--data-binary=')
    ) {
      const next = valueAfterOption(tokens, index);
      body = next.value ?? '';
      index = next.nextIndex;
      continue;
    }

    if (!token.startsWith('-') && !rawUrl) {
      rawUrl = token;
    }
  }

  const { url, params } = parseHttpRequestUrlParts(rawUrl);

  return {
    method: method || (body ? 'POST' : 'GET'),
    url,
    headers,
    params,
    body,
    bodyType: inferBodyType(headers, body)
  };
}
