const fs = require('node:fs');
const path = require('node:path');

const OUTPUT_ROOT = path.join('tmp', 'test-governance');
const REPORT_FILE = 'i18n-hygiene.json';
const DEFAULT_MAX_FINDINGS = 400;
const SKIPPED_DIRS = new Set([
  '.git',
  'coverage',
  'dist',
  'node_modules',
  'target',
  'tmp',
]);
const I18N_ROOTS = ['web/app/src', 'api/plugins'];
const I18N_LOCALES = ['en_US', 'zh_Hans'];

function getRepoRoot() {
  return path.resolve(__dirname, '..', '..', '..');
}

function normalizePath(filePath) {
  return filePath.split(path.sep).join('/');
}

function createFinding({
  severity,
  rule,
  file,
  line = null,
  message,
  key = null,
  value = null,
  owner = null,
  locale = null,
  keys = undefined,
  files = undefined,
  snippet = '',
}) {
  const finding = {
    severity,
    rule,
    file,
    line,
    message,
    snippet: snippet.trim(),
  };

  if (key !== null) {
    finding.key = key;
  }
  if (value !== null) {
    finding.value = value;
  }
  if (owner !== null) {
    finding.owner = owner;
  }
  if (locale !== null) {
    finding.locale = locale;
  }
  if (keys !== undefined) {
    finding.keys = keys;
  }
  if (files !== undefined) {
    finding.files = files;
  }

  return finding;
}

function walkFiles(rootPath, collected = []) {
  if (!fs.existsSync(rootPath)) {
    return collected;
  }

  const stat = fs.statSync(rootPath);
  if (stat.isFile()) {
    collected.push(rootPath);
    return collected;
  }

  for (const entry of fs.readdirSync(rootPath, { withFileTypes: true })) {
    if (entry.isDirectory() && SKIPPED_DIRS.has(entry.name)) {
      continue;
    }

    const absolutePath = path.join(rootPath, entry.name);
    if (entry.isDirectory()) {
      walkFiles(absolutePath, collected);
      continue;
    }

    if (entry.isFile()) {
      collected.push(absolutePath);
    }
  }

  return collected;
}

function collectI18nJsonFiles(repoRoot) {
  return I18N_ROOTS.flatMap((entry) => walkFiles(path.join(repoRoot, entry)))
    .map((absolutePath) => ({
      absolutePath,
      relativePath: normalizePath(path.relative(repoRoot, absolutePath)),
    }))
    .filter(({ relativePath }) => /\/i18n\/[^/]+\.json$/u.test(relativePath))
    .sort((left, right) => left.relativePath.localeCompare(right.relativePath));
}

function schemeForOwner(owner) {
  if (owner.startsWith('web/app/src/')) {
    return {
      name: 'frontend',
      locales: I18N_LOCALES,
      canonicalFileName(fileName) {
        const normalized = fileName.toLowerCase().replace(/-/gu, '_');
        if (normalized === 'zh_hans.json' || normalized === 'zh_cn.json') {
          return 'zh_Hans.json';
        }
        if (normalized === 'en_us.json' || normalized === 'en.json') {
          return 'en_US.json';
        }
        return null;
      },
    };
  }

  return {
    name: 'provider',
    locales: I18N_LOCALES,
    canonicalFileName(fileName) {
      const normalized = fileName.toLowerCase().replace(/-/gu, '_');
      if (normalized === 'zh_hans.json' || normalized === 'zh_cn.json') {
        return 'zh_Hans.json';
      }
      if (normalized === 'en_us.json' || normalized === 'en.json') {
        return 'en_US.json';
      }
      return null;
    },
  };
}

function ownerFromRelativePath(relativePath) {
  return relativePath.replace(/\/i18n\/[^/]+\.json$/u, '');
}

function localeFromFileName(fileName) {
  return fileName.replace(/\.json$/u, '');
}

function lineForIndex(content, index) {
  let line = 1;
  for (let cursor = 0; cursor < index; cursor += 1) {
    if (content[cursor] === '\n') {
      line += 1;
    }
  }
  return line;
}

function readJsonStringToken(content, startIndex) {
  let cursor = startIndex + 1;
  while (cursor < content.length) {
    const char = content[cursor];
    if (char === '\\') {
      cursor += 2;
      continue;
    }
    if (char === '"') {
      const raw = content.slice(startIndex, cursor + 1);
      return {
        value: JSON.parse(raw),
        endIndex: cursor + 1,
      };
    }
    cursor += 1;
  }

  throw new Error('unterminated JSON string');
}

function scanJsonDuplicateKeys({ relativePath, content }) {
  const findings = [];
  const objectStack = [];
  const pathStack = [];
  let index = 0;

  while (index < content.length) {
    const char = content[index];

    if (/\s/u.test(char)) {
      index += 1;
      continue;
    }

    const currentObject = objectStack.at(-1);

    if (char === '{') {
      objectStack.push({
        keys: new Map(),
        expectingKey: true,
        expectingColon: false,
        pendingKey: null,
      });
      index += 1;
      continue;
    }

    if (char === '}') {
      objectStack.pop();
      if (pathStack.length > objectStack.length) {
        pathStack.pop();
      }
      index += 1;
      continue;
    }

    if (char === '[' || char === ']') {
      index += 1;
      continue;
    }

    if (char === ',') {
      if (currentObject) {
        if (pathStack.length > objectStack.length - 1) {
          pathStack.pop();
        }
        currentObject.expectingKey = true;
        currentObject.expectingColon = false;
        currentObject.pendingKey = null;
      }
      index += 1;
      continue;
    }

    if (char === ':') {
      if (currentObject?.expectingColon) {
        pathStack.push(currentObject.pendingKey);
        currentObject.expectingKey = false;
        currentObject.expectingColon = false;
      }
      index += 1;
      continue;
    }

    if (char === '"') {
      const line = lineForIndex(content, index);
      const token = readJsonStringToken(content, index);
      if (currentObject?.expectingKey) {
        const keyPath = [...pathStack, token.value].join('.');
        if (currentObject.keys.has(token.value)) {
          findings.push(createFinding({
            severity: 'error',
            rule: 'duplicate-json-key',
            file: relativePath,
            line,
            key: keyPath,
            message: `JSON object contains duplicate i18n key "${keyPath}"`,
            snippet: content.split(/\r?\n/u)[line - 1] || '',
          }));
        }
        currentObject.keys.set(token.value, line);
        currentObject.expectingKey = false;
        currentObject.expectingColon = true;
        currentObject.pendingKey = token.value;
      }
      index = token.endIndex;
      continue;
    }

    index += 1;
  }

  return findings;
}

function flattenStringValues(value, prefix = '', entries = []) {
  if (typeof value === 'string') {
    entries.push({ key: prefix, value });
    return entries;
  }

  if (!value || typeof value !== 'object' || Array.isArray(value)) {
    return entries;
  }

  for (const key of Object.keys(value).sort((left, right) => left.localeCompare(right))) {
    const nextPrefix = prefix ? `${prefix}.${key}` : key;
    flattenStringValues(value[key], nextPrefix, entries);
  }

  return entries;
}

function normalizeDisplayValue(value) {
  return value.trim().replace(/\s+/gu, ' ');
}

function groupBy(values, resolveKey) {
  const groups = new Map();
  for (const value of values) {
    const key = resolveKey(value);
    const group = groups.get(key) || [];
    group.push(value);
    groups.set(key, group);
  }
  return groups;
}

function collectOwnerFindings({ owner, files }) {
  const findings = [];
  const scheme = schemeForOwner(owner);
  const filesByName = new Map(files.map((file) => [path.basename(file.relativePath), file]));
  const filesByCanonicalName = groupBy(files, (file) => {
    const fileName = path.basename(file.relativePath);
    return scheme.canonicalFileName(fileName) || fileName;
  });

  for (const file of files) {
    const fileName = path.basename(file.relativePath);
    const canonicalName = scheme.canonicalFileName(fileName);
    if (!canonicalName || canonicalName !== fileName) {
      findings.push(createFinding({
        severity: 'error',
        rule: 'invalid-locale-file-name',
        file: file.relativePath,
        owner,
        message: `${scheme.name} i18n file must be named one of: ${scheme.locales
          .map((locale) => `${locale}.json`)
          .join(', ')}`,
      }));
    }
  }

  for (const [canonicalName, matchingFiles] of filesByCanonicalName) {
    if (
      matchingFiles.length > 1
      && scheme.locales.some((locale) => canonicalName === `${locale}.json`)
    ) {
      findings.push(createFinding({
        severity: 'warning',
        rule: 'duplicate-locale-file',
        file: matchingFiles[0].relativePath,
        owner,
        files: matchingFiles.map((file) => file.relativePath),
        message: `i18n owner has multiple files for ${canonicalName}`,
      }));
    }
  }

  for (const locale of scheme.locales) {
    const fileName = `${locale}.json`;
    if (!filesByName.has(fileName)) {
      findings.push(createFinding({
        severity: 'error',
        rule: 'missing-locale-file',
        file: `${owner}/i18n/${fileName}`,
        owner,
        locale,
        message: `i18n owner is missing required locale file ${fileName}`,
      }));
    }
  }

  return findings;
}

function collectParsedFileEntries(files) {
  const findings = [];
  const entries = [];
  const keysByOwnerLocale = new Map();

  for (const file of files) {
    const owner = ownerFromRelativePath(file.relativePath);
    const locale = localeFromFileName(path.basename(file.relativePath));
    const content = fs.readFileSync(file.absolutePath, 'utf8');
    findings.push(...scanJsonDuplicateKeys({ relativePath: file.relativePath, content }));

    let parsed;
    try {
      parsed = JSON.parse(content);
    } catch (error) {
      findings.push(createFinding({
        severity: 'error',
        rule: 'json-parse-error',
        file: file.relativePath,
        owner,
        locale,
        message: error.message,
      }));
      continue;
    }

    const flattened = flattenStringValues(parsed);
    keysByOwnerLocale.set(`${owner}\0${locale}`, new Set(flattened.map((entry) => entry.key)));
    entries.push(...flattened.map((entry) => ({
      ...entry,
      normalizedValue: normalizeDisplayValue(entry.value),
      file: file.relativePath,
      owner,
      locale,
    })));
  }

  return { findings, entries, keysByOwnerLocale };
}

function collectLocaleKeyMismatchFindings({ owners, keysByOwnerLocale }) {
  const findings = [];

  for (const [owner] of owners) {
    const scheme = schemeForOwner(owner);
    const localeKeys = scheme.locales.map((locale) => ({
      locale,
      keys: keysByOwnerLocale.get(`${owner}\0${locale}`) || new Set(),
    }));
    const allKeys = new Set(localeKeys.flatMap(({ keys }) => [...keys]));

    for (const key of [...allKeys].sort((left, right) => left.localeCompare(right))) {
      const missingLocales = localeKeys
        .filter(({ keys }) => !keys.has(key))
        .map(({ locale }) => locale);
      if (missingLocales.length === 0) {
        continue;
      }

      findings.push(createFinding({
        severity: 'error',
        rule: 'locale-key-mismatch',
        file: `${owner}/i18n`,
        owner,
        key,
        message: `i18n key "${key}" is missing in locale(s): ${missingLocales.join(', ')}`,
      }));
    }
  }

  return findings;
}

function collectDuplicateEntryFindings(entries) {
  const findings = [];
  const entriesWithValues = entries.filter((entry) => entry.normalizedValue);

  for (const [groupKey, groupEntries] of groupBy(
    entriesWithValues,
    (entry) => `${entry.owner}\0${entry.locale}\0${entry.normalizedValue}`
  )) {
    const [, , normalizedValue] = groupKey.split('\0');
    const uniqueKeys = [...new Set(groupEntries.map((entry) => entry.key))];
    if (uniqueKeys.length <= 1) {
      continue;
    }

    findings.push(createFinding({
      severity: 'error',
      rule: 'duplicate-value-in-owner',
      file: groupEntries[0].file,
      owner: groupEntries[0].owner,
      locale: groupEntries[0].locale,
      value: normalizedValue,
      keys: uniqueKeys.sort((left, right) => left.localeCompare(right)),
      message: `i18n owner has duplicated ${groupEntries[0].locale} value "${normalizedValue}"`,
    }));
  }

  for (const [groupKey, groupEntries] of groupBy(
    entriesWithValues,
    (entry) => `${entry.locale}\0${entry.normalizedValue}`
  )) {
    const [locale, normalizedValue] = groupKey.split('\0');
    const owners = [...new Set(groupEntries.map((entry) => entry.owner))];
    if (owners.length <= 1) {
      continue;
    }

    findings.push(createFinding({
      severity: 'warning',
      rule: 'duplicate-value-across-owners',
      file: groupEntries[0].file,
      locale,
      value: normalizedValue,
      files: groupEntries.map((entry) => `${entry.file}:${entry.key}`),
      message: `i18n value "${normalizedValue}" appears in multiple owners; only extract to common when semantics are identical`,
    }));
  }

  for (const [key, groupEntries] of groupBy(entries, (entry) => entry.key)) {
    const owners = [...new Set(groupEntries.map((entry) => entry.owner))];
    if (owners.length <= 1) {
      continue;
    }

    findings.push(createFinding({
      severity: 'warning',
      rule: 'duplicate-key-across-owners',
      file: groupEntries[0].file,
      key,
      files: groupEntries.map((entry) => `${entry.file}:${entry.key}`),
      message: `i18n key "${key}" appears in multiple owners; keep only if owner paths express different semantics`,
    }));
  }

  return findings;
}

function collectI18nHygieneFindings({ repoRoot = getRepoRoot() } = {}) {
  const files = collectI18nJsonFiles(repoRoot);
  const owners = groupBy(files, (file) => ownerFromRelativePath(file.relativePath));
  const findings = [];

  for (const [owner, ownerFiles] of owners) {
    findings.push(...collectOwnerFindings({ owner, files: ownerFiles }));
  }

  const parsed = collectParsedFileEntries(files);
  findings.push(...parsed.findings);
  findings.push(...collectLocaleKeyMismatchFindings({ owners, keysByOwnerLocale: parsed.keysByOwnerLocale }));
  findings.push(...collectDuplicateEntryFindings(parsed.entries));

  return findings.sort((left, right) => {
    const severityOrder = left.severity.localeCompare(right.severity);
    if (severityOrder !== 0) {
      return severityOrder;
    }
    return left.file.localeCompare(right.file) || left.rule.localeCompare(right.rule);
  });
}

function parseCliArgs(argv) {
  if (argv.includes('-h') || argv.includes('--help')) {
    return {
      help: true,
      maxFindings: DEFAULT_MAX_FINDINGS,
    };
  }

  let maxFindings = DEFAULT_MAX_FINDINGS;
  for (let index = 0; index < argv.length; index += 1) {
    const arg = argv[index];
    if (arg === '--max-findings') {
      const value = Number.parseInt(argv[index + 1], 10);
      if (!Number.isInteger(value) || value <= 0) {
        throw new Error('--max-findings must be a positive integer');
      }
      maxFindings = value;
      index += 1;
      continue;
    }
    throw new Error(`Unknown i18n-hygiene option: ${arg}`);
  }

  return {
    help: false,
    maxFindings,
  };
}

function writeReport({ repoRoot, findings }) {
  const outputDir = path.join(repoRoot, OUTPUT_ROOT);
  fs.mkdirSync(outputDir, { recursive: true });
  const report = {
    summary: {
      total: findings.length,
      errors: findings.filter((finding) => finding.severity === 'error').length,
      warnings: findings.filter((finding) => finding.severity === 'warning').length,
    },
    findings,
  };
  fs.writeFileSync(path.join(outputDir, REPORT_FILE), `${JSON.stringify(report, null, 2)}\n`, 'utf8');
  return report;
}

function usage(writeStdout = (text) => process.stdout.write(text)) {
  writeStdout(
    'Usage: node scripts/node/tooling.js i18n-hygiene [--max-findings <n>]\n'
      + 'Checks i18n locale file names, key alignment, duplicate JSON keys, and semantic value reuse.\n'
  );
}

async function main(argv = [], deps = {}) {
  const options = parseCliArgs(argv);
  const writeStdout = deps.writeStdout || ((text) => process.stdout.write(text));
  const writeStderr = deps.writeStderr || ((text) => process.stderr.write(text));

  if (options.help) {
    usage(writeStdout);
    return 0;
  }

  const repoRoot = deps.repoRoot || getRepoRoot();
  const findings = collectI18nHygieneFindings({ repoRoot });
  const report = writeReport({ repoRoot, findings });
  const reportPath = normalizePath(path.join(OUTPUT_ROOT, REPORT_FILE));

  writeStdout(
    `[1flowbase-i18n-hygiene] ${report.summary.total} findings `
      + `(${report.summary.errors} errors, ${report.summary.warnings} warnings). `
      + `Report: ${reportPath}\n`
  );

  for (const finding of findings.slice(0, options.maxFindings)) {
    const location = finding.line ? `${finding.file}:${finding.line}` : finding.file;
    const text = `[i18n-hygiene:${finding.rule}] ${location} ${finding.message}\n`;
    if (finding.severity === 'error') {
      writeStderr(text);
    } else {
      writeStdout(text);
    }
  }

  return report.summary.errors > 0 ? 1 : 0;
}

module.exports = {
  collectI18nHygieneFindings,
  collectI18nJsonFiles,
  flattenStringValues,
  main,
  parseCliArgs,
  scanJsonDuplicateKeys,
};
