const localBindingIdentifiers = new Set<string>([
  'as',
  'async',
  'await',
  'break',
  'case',
  'catch',
  'class',
  'const',
  'continue',
  'debugger',
  'default',
  'delete',
  'do',
  'else',
  'export',
  'extends',
  'false',
  'finally',
  'for',
  'from',
  'function',
  'if',
  'import',
  'in',
  'instanceof',
  'let',
  'new',
  'null',
  'return',
  'super',
  'switch',
  'this',
  'throw',
  'true',
  'try',
  'typeof',
  'undefined',
  'var',
  'void',
  'while',
  'with',
  'yield'
]);

export function isImportName(value: string): boolean {
  return value === 'default' || isIdentifierName(value);
}

export function isLocalBindingName(
  value: string,
  reservedIdentifiers: ReadonlySet<string>
): boolean {
  return (
    isIdentifierName(value) &&
    !localBindingIdentifiers.has(value) &&
    !reservedIdentifiers.has(value)
  );
}

export function isIdentifierName(value: string): boolean {
  return /^[A-Za-z_$][A-Za-z0-9_$]*$/.test(value);
}

export function isWhitespace(char: string): boolean {
  return char === ' ' || char === '\t' || char === '\n' || char === '\r';
}

export function isIdentifierStart(char: string): boolean {
  return /[A-Za-z_$]/.test(char);
}

export function isIdentifierPart(char: string): boolean {
  return /[A-Za-z0-9_$]/.test(char);
}
