const fs = require('node:fs');
const path = require('node:path');

const OUTPUT_ROOT = path.join('tmp', 'test-governance');
const REPORT_FILE = 'repo-hygiene.json';
const DEFAULT_MAX_FINDINGS = 400;
const ROOT_ENTRIES = ['api', 'web', 'scripts', '.github', 'AGENTS.md'];
const SOURCE_EXTENSIONS = new Set([
  '.css',
  '.js',
  '.json',
  '.md',
  '.rs',
  '.ts',
  '.tsx',
  '.yml',
  '.yaml',
]);
const CODE_EXTENSIONS = new Set(['.css', '.js', '.rs', '.ts', '.tsx']);
const SKIPPED_DIRS = new Set([
  '.git',
  'coverage',
  'dist',
  'node_modules',
  'target',
  'tmp',
]);
const SKIPPED_FILES = new Set([
  'api/Cargo.lock',
  'web/pnpm-lock.yaml',
]);
const DEBT_MARKER_PATTERN = /\b(TODO|FIXME|HACK|legacy|compat(?:ibility)?|deprecated|obsolete)\b/iu;
const FIELD_CONTRACT_COMPAT_MARKER_TEXT = /@field-contract-compat\b/u;
const FIELD_CONTRACT_COMPAT_MARKER_PATTERN = /(?:\/\/|#|\/\*|\*)\s*@field-contract-compat\b/u;
const BENIGN_MARKER_PATTERNS = [
  /\bdeprecated:\s*false\b/u,
  /\bdeprecated:\s*bool\b/u,
  /\bdeprecated:\s*boolean\b/u,
  /\bcompat-data\b/u,
  FIELD_CONTRACT_COMPAT_MARKER_TEXT,
];
const FOCUSED_TEST_PATTERN = /\b(?:describe|it|test)\.only\s*\(/u;
const SKIPPED_TEST_PATTERN = /\b(?:describe|it|test)\.(?:skip|todo)\s*\(|\bx(?:describe|it)\s*\(/u;
const WEAK_ASSERTION_PATTERN = /expect\([^\n]+\)\.(?:toBeTruthy|toBeDefined)\s*\(/u;
const TEST_TITLE_PATTERN = /\b(?:describe|it|test)\s*\(\s*(['"`])([^'"`\n]+)\1/gu;
const TEST_PATH_PATTERN = /(?:^|\/)(?:_tests|tests)\//u;

function getRepoRoot() {
  return path.resolve(__dirname, '..', '..', '..');
}

function normalizePath(filePath) {
  return filePath.split(path.sep).join('/');
}

function isSkippedDirectory(entryName) {
  return SKIPPED_DIRS.has(entryName);
}

function shouldScanFile(relativePath) {
  if (SKIPPED_FILES.has(relativePath)) {
    return false;
  }

  return SOURCE_EXTENSIONS.has(path.extname(relativePath));
}

function isCodeFile(relativePath) {
  return CODE_EXTENSIONS.has(path.extname(relativePath));
}

function isTestPath(relativePath) {
  return TEST_PATH_PATTERN.test(relativePath) || /(?:^|[./-])(?:test|spec)\.[jt]sx?$/u.test(relativePath);
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
    if (entry.isDirectory() && isSkippedDirectory(entry.name)) {
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

function collectSourceFiles(repoRoot) {
  return ROOT_ENTRIES.flatMap((entry) => walkFiles(path.join(repoRoot, entry)))
    .map((absolutePath) => ({
      absolutePath,
      relativePath: normalizePath(path.relative(repoRoot, absolutePath)),
    }))
    .filter(({ relativePath }) => shouldScanFile(relativePath))
    .sort((left, right) => left.relativePath.localeCompare(right.relativePath));
}

function createFinding({ severity = 'warning', rule, file, line = null, message, snippet = '' }) {
  return {
    severity,
    rule,
    file,
    line,
    message,
    snippet: snippet.trim(),
  };
}

function isBenignMarkerLine(line) {
  return BENIGN_MARKER_PATTERNS.some((pattern) => pattern.test(line));
}

function stripStringLiterals(line) {
  return line
    .replace(/'([^'\\]|\\.)*'/gu, "''")
    .replace(/"([^"\\]|\\.)*"/gu, '""')
    .replace(/`([^`\\]|\\.)*`/gu, '``');
}

function scanSourceFile({ relativePath, content }) {
  const lines = content.split(/\r?\n/u);
  const findings = [];
  const testPath = isTestPath(relativePath);

  lines.forEach((line, index) => {
    const lineNumber = index + 1;

    if (
      !testPath
      && isCodeFile(relativePath)
      && !line.includes('FIELD_CONTRACT_COMPAT_MARKER')
      && FIELD_CONTRACT_COMPAT_MARKER_PATTERN.test(line)
    ) {
      findings.push(createFinding({
        rule: 'field-contract-compat-marker',
        file: relativePath,
        line: lineNumber,
        message: 'front-back field compatibility alias must stay visible in QA reports until removed',
        snippet: line,
      }));
    } else if (DEBT_MARKER_PATTERN.test(line) && !isBenignMarkerLine(line)) {
      findings.push(createFinding({
        rule: 'source-debt-marker',
        file: relativePath,
        line: lineNumber,
        message: 'source contains a legacy/deprecated/TODO-style marker that should stay visible in QA reports',
        snippet: line,
      }));
    }

    if (!testPath) {
      return;
    }

    const structuralLine = stripStringLiterals(line);

    if (FOCUSED_TEST_PATTERN.test(structuralLine)) {
      findings.push(createFinding({
        severity: 'error',
        rule: 'focused-test',
        file: relativePath,
        line: lineNumber,
        message: 'focused test would make CI execute an incomplete test set',
        snippet: line,
      }));
    }

    if (SKIPPED_TEST_PATTERN.test(structuralLine)) {
      findings.push(createFinding({
        rule: 'skipped-test',
        file: relativePath,
        line: lineNumber,
        message: 'skipped or todo test needs an explicit owner and removal path',
        snippet: line,
      }));
    }

    if (WEAK_ASSERTION_PATTERN.test(structuralLine)) {
      findings.push(createFinding({
        rule: 'weak-test-assertion',
        file: relativePath,
        line: lineNumber,
        message: 'weak assertion should be replaced with a behavior-specific expectation',
        snippet: line,
      }));
    }
  });

  return findings;
}

function countLines(content) {
  if (content.length === 0) {
    return 0;
  }

  return content.endsWith('\n')
    ? content.split('\n').length - 1
    : content.split('\n').length;
}

function collectLinePressureFindings({ relativePath, content }) {
  if (!isCodeFile(relativePath)) {
    return [];
  }

  const lines = countLines(content);
  if (lines < 1200) {
    return [];
  }

  return [createFinding({
    rule: 'file-size-pressure',
    file: relativePath,
    message: lines >= 1500
      ? 'file is at or over the repository split pressure line'
      : 'file is approaching the repository split pressure line',
    snippet: `${lines} lines`,
  })];
}

function collectDirectoryPressureFindings(files) {
  const counts = new Map();

  for (const { relativePath } of files) {
    if (!isCodeFile(relativePath)) {
      continue;
    }

    const directory = path.posix.dirname(relativePath);
    counts.set(directory, (counts.get(directory) || 0) + 1);
  }

  return [...counts.entries()]
    .filter(([, count]) => count > 15)
    .sort((left, right) => right[1] - left[1])
    .map(([directory, count]) => createFinding({
      rule: 'directory-pressure',
      file: directory,
      message: 'directory has more than 15 source files and should be reviewed for owner subfolders',
      snippet: `${count} files`,
    }));
}

function collectDuplicateTestTitleFindings(fileContents) {
  const byTitle = new Map();

  for (const { relativePath, content } of fileContents) {
    if (!isTestPath(relativePath)) {
      continue;
    }

    let match = TEST_TITLE_PATTERN.exec(content);
    while (match) {
      const title = match[2].replace(/\s+/gu, ' ').trim();
      if (title) {
        const entries = byTitle.get(title) || [];
        entries.push(relativePath);
        byTitle.set(title, entries);
      }
      match = TEST_TITLE_PATTERN.exec(content);
    }
  }

  return [...byTitle.entries()]
    .filter(([, files]) => new Set(files).size > 1)
    .map(([title, files]) => createFinding({
      rule: 'duplicate-test-title',
      file: [...new Set(files)].sort().join(', '),
      message: 'duplicate test title makes failure triage less precise',
      snippet: title,
    }));
}

function loadFileContents(files) {
  return files.map(({ absolutePath, relativePath }) => ({
    relativePath,
    content: fs.readFileSync(absolutePath, 'utf8'),
  }));
}

function collectRepoHygieneFindings({ repoRoot = getRepoRoot() } = {}) {
  const files = collectSourceFiles(repoRoot);
  const fileContents = loadFileContents(files);
  const findings = [];

  for (const fileContent of fileContents) {
    findings.push(...scanSourceFile(fileContent));
    findings.push(...collectLinePressureFindings(fileContent));
  }

  findings.push(...collectDirectoryPressureFindings(files));
  findings.push(...collectDuplicateTestTitleFindings(fileContents));

  return findings;
}

function summarizeFindings(findings) {
  const summary = {
    total: findings.length,
    errors: findings.filter((finding) => finding.severity === 'error').length,
    warnings: findings.filter((finding) => finding.severity === 'warning').length,
    byRule: {},
  };

  for (const finding of findings) {
    summary.byRule[finding.rule] = (summary.byRule[finding.rule] || 0) + 1;
  }

  return summary;
}

function parseRepoHygieneCliArgs(argv = []) {
  const options = {
    help: false,
    maxFindings: DEFAULT_MAX_FINDINGS,
  };

  for (let index = 0; index < argv.length; index += 1) {
    const arg = argv[index];

    if (arg === '-h' || arg === '--help') {
      return { ...options, help: true };
    }

    if (arg === '--max-findings') {
      const value = argv[index + 1];
      if (!value || value.startsWith('--')) {
        throw new Error('--max-findings requires a value');
      }
      options.maxFindings = Number.parseInt(value, 10);
      index += 1;
      continue;
    }

    if (arg.startsWith('--max-findings=')) {
      options.maxFindings = Number.parseInt(arg.slice('--max-findings='.length), 10);
      continue;
    }

    throw new Error(`Unknown repo-hygiene option: ${arg}`);
  }

  if (!Number.isFinite(options.maxFindings) || options.maxFindings <= 0) {
    throw new Error('--max-findings must be a positive integer');
  }

  return options;
}

function writeReport({ repoRoot, findings, maxFindings }) {
  const outputDir = path.join(repoRoot, OUTPUT_ROOT);
  fs.mkdirSync(outputDir, { recursive: true });

  const report = {
    status: findings.some((finding) => finding.severity === 'error') ? 'failed' : 'passed',
    summary: summarizeFindings(findings),
    findings: findings.slice(0, maxFindings),
    truncated: findings.length > maxFindings,
  };

  const reportPath = path.join(outputDir, REPORT_FILE);
  fs.writeFileSync(reportPath, `${JSON.stringify(report, null, 2)}\n`, 'utf8');

  return { report, reportPath };
}

function usage(writeStdout = (text) => process.stdout.write(text)) {
  writeStdout(
    'Usage: node scripts/node/tooling.js repo-hygiene [--max-findings <n>]\n'
      + 'Scans repository hygiene signals: debt markers, field contract compatibility markers, weak assertions, duplicate tests, file and directory pressure.\n'
  );
}

async function main(argv = [], deps = {}) {
  const options = parseRepoHygieneCliArgs(argv);
  const writeStdout = deps.writeStdout || ((text) => process.stdout.write(text));
  const writeStderr = deps.writeStderr || ((text) => process.stderr.write(text));

  if (options.help) {
    usage(writeStdout);
    return 0;
  }

  const repoRoot = deps.repoRoot || getRepoRoot();
  const findings = (deps.collectFindingsImpl || collectRepoHygieneFindings)({ repoRoot });
  const { report, reportPath } = writeReport({
    repoRoot,
    findings,
    maxFindings: options.maxFindings,
  });

  writeStdout(
    `[1flowbase-repo-hygiene] ${report.summary.total} findings `
      + `(${report.summary.errors} errors, ${report.summary.warnings} warnings). `
      + `Report: ${normalizePath(path.relative(repoRoot, reportPath))}\n`
  );

  for (const finding of findings.filter((candidate) => candidate.severity === 'error')) {
    writeStderr(
      `[repo-hygiene:${finding.rule}] ${finding.file}`
        + `${finding.line ? `:${finding.line}` : ''} ${finding.message}\n`
    );
  }

  return report.summary.errors > 0 ? 1 : 0;
}

module.exports = {
  collectDirectoryPressureFindings,
  collectDuplicateTestTitleFindings,
  collectRepoHygieneFindings,
  collectSourceFiles,
  main,
  parseRepoHygieneCliArgs,
  scanSourceFile,
  summarizeFindings,
  writeReport,
};
