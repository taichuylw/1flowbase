import type { BlockProtocolError } from '@1flowbase/page-protocol';

export const JS_BLOCK_ALLOWED_IMPORTS = [
  '@1flowbase/block-sdk',
  '@1flowbase/antd-facade'
] as const;

type JsBlockAllowedImport = (typeof JS_BLOCK_ALLOWED_IMPORTS)[number];

export interface ValidateJsBlockSourceSuccess {
  ok: true;
  source: string;
  normalizedSource: string;
  errors: [];
}

export interface ValidateJsBlockSourceFailure {
  ok: false;
  errors: BlockProtocolError[];
}

export type ValidateJsBlockSourceResult =
  | ValidateJsBlockSourceSuccess
  | ValidateJsBlockSourceFailure;

interface SourceToken {
  value: string;
  start: number;
  end: number;
}

interface ScanResult {
  tokens: SourceToken[];
  error?: BlockProtocolError;
}

const allowedImports = new Set<string>(
  JS_BLOCK_ALLOWED_IMPORTS satisfies readonly JsBlockAllowedImport[]
);

const deniedGlobalIdentifiers = new Set([
  'window',
  'document',
  'globalThis',
  'self',
  'localStorage',
  'sessionStorage',
  'cookie'
]);

const deniedCallIdentifiers = new Set([
  'require',
  'eval',
  'fetch',
  'sendBeacon'
]);

const deniedConstructorIdentifiers = new Set([
  'Function',
  'XMLHttpRequest',
  'WebSocket'
]);

export function validateJsBlockSource(
  source: unknown
): ValidateJsBlockSourceResult {
  try {
    if (typeof source !== 'string') {
      return failure(
        'transform_failed',
        'source',
        'JS block source must be a string.'
      );
    }

    const scan = scanSource(source);
    if (scan.error) {
      return { ok: false, errors: [scan.error] };
    }

    const errors = [
      ...validateImports(source, scan.tokens),
      ...validateDeniedIdentifiers(source, scan.tokens)
    ];

    if (errors.length > 0) {
      return { ok: false, errors };
    }

    return {
      ok: true,
      source,
      normalizedSource: source.trim(),
      errors: []
    };
  } catch {
    return failure(
      'transform_failed',
      'source',
      'JS block source validation failed.'
    );
  }
}

function scanSource(source: string): ScanResult {
  const tokens: SourceToken[] = [];
  const result = scanCode(source, 0, tokens);

  if (result.error) {
    return { tokens, error: result.error };
  }

  return { tokens };
}

interface ScanCodeResult {
  index: number;
  closedByStop: boolean;
  error?: BlockProtocolError;
}

function scanCode(
  source: string,
  start: number,
  tokens: SourceToken[],
  stopChar?: string
): ScanCodeResult {
  const delimiterStack: Array<{ expected: string; index: number }> = [];
  let index = start;

  while (index < source.length) {
    const char = source[index];
    const next = source[index + 1];

    if (stopChar && char === stopChar && delimiterStack.length === 0) {
      return { index: index + 1, closedByStop: true };
    }

    if (isWhitespace(char)) {
      index += 1;
      continue;
    }

    if (char === '/' && next === '/') {
      index = consumeLineComment(source, index + 2);
      continue;
    }

    if (char === '/' && next === '*') {
      const commentEnd = source.indexOf('*/', index + 2);
      if (commentEnd === -1) {
        return syntaxError(index, 'Unterminated block comment.');
      }
      index = commentEnd + 2;
      continue;
    }

    if (char === '"' || char === "'") {
      const stringEnd = consumeQuotedString(source, index, char);
      if (stringEnd.error) {
        return stringEnd;
      }
      index = stringEnd.index;
      continue;
    }

    if (char === '`') {
      const templateEnd = consumeTemplate(source, index, tokens);
      if (templateEnd.error) {
        return templateEnd;
      }
      index = templateEnd.index;
      continue;
    }

    if (isIdentifierStart(char)) {
      const tokenStart = index;
      index += 1;
      while (index < source.length && isIdentifierPart(source[index])) {
        index += 1;
      }
      tokens.push({
        value: source.slice(tokenStart, index),
        start: tokenStart,
        end: index
      });
      continue;
    }

    if (char === '(' || char === '[' || char === '{') {
      delimiterStack.push({
        expected: matchingDelimiter(char),
        index
      });
      index += 1;
      continue;
    }

    if (char === ')' || char === ']' || char === '}') {
      const current = delimiterStack.pop();
      if (!current || current.expected !== char) {
        return syntaxError(index, `Unexpected '${char}'.`);
      }
      index += 1;
      continue;
    }

    index += 1;
  }

  if (stopChar) {
    return syntaxError(start, `Unterminated '${stopChar}' expression.`);
  }

  const openDelimiter = delimiterStack.at(-1);
  if (openDelimiter) {
    return syntaxError(
      openDelimiter.index,
      `Unterminated '${openDelimiter.expected}' delimiter.`
    );
  }

  return { index, closedByStop: false };
}

function consumeLineComment(source: string, start: number): number {
  const lineEnd = source.indexOf('\n', start);
  return lineEnd === -1 ? source.length : lineEnd + 1;
}

function consumeQuotedString(
  source: string,
  start: number,
  quote: '"' | "'"
): ScanCodeResult {
  let index = start + 1;

  while (index < source.length) {
    const char = source[index];

    if (char === '\\') {
      index += 2;
      continue;
    }

    if (char === quote) {
      return { index: index + 1, closedByStop: true };
    }

    if (char === '\n' || char === '\r') {
      return syntaxError(start, 'Unterminated string literal.');
    }

    index += 1;
  }

  return syntaxError(start, 'Unterminated string literal.');
}

function consumeTemplate(
  source: string,
  start: number,
  tokens: SourceToken[]
): ScanCodeResult {
  let index = start + 1;

  while (index < source.length) {
    const char = source[index];
    const next = source[index + 1];

    if (char === '\\') {
      index += 2;
      continue;
    }

    if (char === '`') {
      return { index: index + 1, closedByStop: true };
    }

    if (char === '$' && next === '{') {
      const expression = scanCode(source, index + 2, tokens, '}');
      if (expression.error) {
        return expression;
      }
      index = expression.index;
      continue;
    }

    index += 1;
  }

  return syntaxError(start, 'Unterminated template literal.');
}

function validateImports(
  source: string,
  tokens: SourceToken[]
): BlockProtocolError[] {
  const errors: BlockProtocolError[] = [];
  let importIndex = 0;

  tokens.forEach((token, tokenIndex) => {
    if (token.value === 'import') {
      const importError = validateImportToken(
        source,
        tokens,
        tokenIndex,
        importIndex
      );
      if (importError) {
        errors.push(importError);
      }
      importIndex += 1;
      return;
    }

    if (token.value === 'export') {
      const exportedSource = readExportSource(source, tokens, tokenIndex);
      if (exportedSource && !allowedImports.has(exportedSource.value)) {
        errors.push(
          failureError(
            'import_denied',
            `source.imports[${importIndex}]`,
            `Import source '${exportedSource.value}' is not allowed.`
          )
        );
      }
      if (exportedSource) {
        importIndex += 1;
      }
    }
  });

  return errors;
}

function validateImportToken(
  source: string,
  tokens: SourceToken[],
  tokenIndex: number,
  importIndex: number
): BlockProtocolError | undefined {
  const token = tokens[tokenIndex];
  const path = `source.imports[${importIndex}]`;
  const nextCodeIndex = skipWhitespace(source, token.end);
  const nextChar = source[nextCodeIndex];

  if (nextChar === '(' || nextChar === '.') {
    return failureError(
      'import_denied',
      path,
      'Dynamic import and import host access are not allowed.'
    );
  }

  if (nextChar === '"' || nextChar === "'") {
    const sourceLiteral = readStringLiteral(source, nextCodeIndex);
    if (!sourceLiteral) {
      return failureError('syntax_invalid', 'source', 'Invalid import source.');
    }
    return allowedImports.has(sourceLiteral.value)
      ? undefined
      : failureError(
          'import_denied',
          path,
          `Import source '${sourceLiteral.value}' is not allowed.`
        );
  }

  const fromToken = findTokenBeforeStatementEnd(
    source,
    tokens,
    tokenIndex + 1,
    'from'
  );
  if (!fromToken) {
    return failureError(
      'syntax_invalid',
      'source',
      'Invalid import statement.'
    );
  }

  const sourceLiteralIndex = skipWhitespace(source, fromToken.end);
  const sourceLiteral = readStringLiteral(source, sourceLiteralIndex);
  if (!sourceLiteral) {
    return failureError('syntax_invalid', 'source', 'Invalid import source.');
  }

  return allowedImports.has(sourceLiteral.value)
    ? undefined
    : failureError(
        'import_denied',
        path,
        `Import source '${sourceLiteral.value}' is not allowed.`
      );
}

function readExportSource(
  source: string,
  tokens: SourceToken[],
  tokenIndex: number
): { value: string } | undefined {
  const fromToken = findTokenBeforeStatementEnd(
    source,
    tokens,
    tokenIndex + 1,
    'from'
  );
  if (!fromToken) {
    return undefined;
  }

  const sourceLiteralIndex = skipWhitespace(source, fromToken.end);
  return readStringLiteral(source, sourceLiteralIndex);
}

function findTokenBeforeStatementEnd(
  source: string,
  tokens: SourceToken[],
  startTokenIndex: number,
  tokenValue: string
): SourceToken | undefined {
  for (let index = startTokenIndex; index < tokens.length; index += 1) {
    const token = tokens[index];
    const segment = source.slice(tokens[startTokenIndex - 1].end, token.start);
    if (segment.includes(';')) {
      return undefined;
    }
    if (token.value === tokenValue) {
      return token;
    }
  }

  return undefined;
}

function validateDeniedIdentifiers(
  source: string,
  tokens: SourceToken[]
): BlockProtocolError[] {
  const errors: BlockProtocolError[] = [];

  tokens.forEach((token, tokenIndex) => {
    if (deniedGlobalIdentifiers.has(token.value)) {
      errors.push(
        failureError(
          'transform_failed',
          `source.identifiers.${token.value}`,
          `Identifier '${token.value}' is not allowed in JS block source.`
        )
      );
      return;
    }

    if (
      deniedCallIdentifiers.has(token.value) &&
      isCallExpression(source, token)
    ) {
      errors.push(
        failureError(
          token.value === 'require' ? 'import_denied' : 'transform_failed',
          `source.identifiers.${token.value}`,
          `Call '${token.value}' is not allowed in JS block source.`
        )
      );
      return;
    }

    if (
      deniedConstructorIdentifiers.has(token.value) &&
      (isCallExpression(source, token) ||
        previousToken(tokens, tokenIndex)?.value === 'new')
    ) {
      errors.push(
        failureError(
          'transform_failed',
          `source.identifiers.${token.value}`,
          `Constructor '${token.value}' is not allowed in JS block source.`
        )
      );
    }
  });

  return errors;
}

function previousToken(
  tokens: SourceToken[],
  tokenIndex: number
): SourceToken | undefined {
  return tokenIndex > 0 ? tokens[tokenIndex - 1] : undefined;
}

function isCallExpression(source: string, token: SourceToken): boolean {
  return source[skipWhitespace(source, token.end)] === '(';
}

function readStringLiteral(
  source: string,
  start: number
): { value: string } | undefined {
  const quote = source[start];
  if (quote !== '"' && quote !== "'") {
    return undefined;
  }

  let index = start + 1;
  let value = '';

  while (index < source.length) {
    const char = source[index];

    if (char === '\\') {
      value += source[index + 1] ?? '';
      index += 2;
      continue;
    }

    if (char === quote) {
      return { value };
    }

    value += char;
    index += 1;
  }

  return undefined;
}

function skipWhitespace(source: string, start: number): number {
  let index = start;

  while (index < source.length && isWhitespace(source[index])) {
    index += 1;
  }

  return index;
}

function matchingDelimiter(char: string): string {
  if (char === '(') {
    return ')';
  }
  if (char === '[') {
    return ']';
  }
  return '}';
}

function isWhitespace(char: string): boolean {
  return char === ' ' || char === '\t' || char === '\n' || char === '\r';
}

function isIdentifierStart(char: string): boolean {
  return /[A-Za-z_$]/.test(char);
}

function isIdentifierPart(char: string): boolean {
  return /[A-Za-z0-9_$]/.test(char);
}

function syntaxError(index: number, message: string): ScanCodeResult {
  return {
    index,
    closedByStop: false,
    error: failureError('syntax_invalid', 'source', message)
  };
}

function failure(
  code: BlockProtocolError['code'],
  path: string,
  message: string
): ValidateJsBlockSourceFailure {
  return {
    ok: false,
    errors: [failureError(code, path, message)]
  };
}

function failureError(
  code: BlockProtocolError['code'],
  path: string,
  message: string
): BlockProtocolError {
  return { code, path, message };
}
