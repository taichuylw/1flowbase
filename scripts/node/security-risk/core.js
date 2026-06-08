const fs = require('node:fs');
const path = require('node:path');
const { spawnSync } = require('node:child_process');

const { getRepoRoot, resolveOutputDir } = require('../testing/warning-capture.js');

const REPORT_FILE = 'security-risk.json';
const DEFAULT_BASE_REF = 'origin/main';

const HIGH_RISK_FILE_PATTERNS = [
  /^package\.json$/u,
  /^pnpm-lock\.yaml$/u,
  /^web\/.*package\.json$/u,
  /^web\/pnpm-lock\.yaml$/u,
  /^api\/Cargo\.(?:toml|lock)$/u,
  /^api\/.*\/Cargo\.toml$/u,
  /^\.github\/workflows\//u,
  /^\.github\/actions\//u,
  /^docker\//u,
  /^scripts\/(?:shell|powershell|node\/docker-deploy|node\/dev-up)\//u,
  /(?:^|\/)(?:Dockerfile|docker-compose[^/]*\.ya?ml|nginx\.conf)$/u,
];

const DEPENDENCY_MANIFEST_FILE_PATTERNS = [
  /^package\.json$/u,
  /^web\/.*package\.json$/u,
  /^api\/Cargo\.toml$/u,
  /^api\/.*\/Cargo\.toml$/u,
];

const RISK_PATTERNS = [
  {
    severity: 'high',
    kind: 'insecure-url',
    pattern: /\b(?:http|ws):\/\/[^\s"'`<>)]+/u,
  },
  {
    severity: 'medium',
    kind: 'external-url',
    pattern: /\b(?:https|wss):\/\/[^\s"'`<>)]+/u,
  },
  {
    severity: 'medium',
    kind: 'javascript-network-call',
    pattern: /\b(?:fetch|axios|WebSocket|EventSource)\s*\(/u,
  },
  {
    severity: 'medium',
    kind: 'rust-network-call',
    pattern: /\b(?:reqwest|hyper|tokio_tungstenite|tungstenite)::/u,
  },
  {
    severity: 'high',
    kind: 'process-execution',
    pattern: /\b(?:child_process|spawn|exec|execFile|Command::new)\b/u,
  },
  {
    severity: 'high',
    kind: 'install-script',
    pattern: /"(?:preinstall|install|postinstall|prepare)"\s*:/u,
  },
  {
    severity: 'high',
    kind: 'remote-dependency',
    pattern: /"[^"]+"\s*:\s*"(?:git\+https?:|https?:\/\/|github:|gitlab:|bitbucket:)[^"]*"/u,
    appliesToFile: isDependencyManifestFile,
  },
  {
    severity: 'medium',
    kind: 'callback-or-webhook',
    pattern: /\b(?:callback|webhook|proxy|upgrade|websocket|resume_url|callback_url)\b/iu,
  },
];

function normalizePath(filePath) {
  return filePath.replace(/\\/gu, '/');
}

function isDependencyManifestFile(filePath) {
  return DEPENDENCY_MANIFEST_FILE_PATTERNS.some((pattern) => pattern.test(filePath));
}

function readChangedFiles({ repoRoot, baseRef, env, spawnSyncImpl = spawnSync }) {
  if (env?.SECURITY_RISK_CHANGED_FILES) {
    return env.SECURITY_RISK_CHANGED_FILES
      .split(/\r?\n/u)
      .map((line) => normalizePath(line.trim()))
      .filter(Boolean);
  }

  const result = spawnSyncImpl('git', ['diff', '--name-only', `${baseRef}...HEAD`], {
    cwd: repoRoot,
    encoding: 'utf8',
    stdio: ['ignore', 'pipe', 'pipe'],
  });

  if (result.error) {
    throw result.error;
  }

  if (result.status !== 0) {
    throw new Error(`Failed to resolve changed files against ${baseRef}: ${result.stderr || result.stdout}`);
  }

  return result.stdout
    .split(/\r?\n/u)
    .map((line) => normalizePath(line.trim()))
    .filter(Boolean);
}

function readDiff({ repoRoot, baseRef, env, spawnSyncImpl = spawnSync }) {
  if (env?.SECURITY_RISK_DIFF) {
    return env.SECURITY_RISK_DIFF;
  }

  const result = spawnSyncImpl('git', ['diff', '--unified=0', `${baseRef}...HEAD`], {
    cwd: repoRoot,
    encoding: 'utf8',
    stdio: ['ignore', 'pipe', 'pipe'],
    maxBuffer: 16 * 1024 * 1024,
  });

  if (result.error) {
    throw result.error;
  }

  if (result.status !== 0) {
    throw new Error(`Failed to resolve diff against ${baseRef}: ${result.stderr || result.stdout}`);
  }

  return result.stdout;
}

function isAddedDiffLine(line) {
  return line.startsWith('+') && !line.startsWith('+++');
}

function scanDiffText(diffText) {
  const findings = [];
  let currentFile = '';

  for (const line of diffText.split(/\r?\n/u)) {
    if (line.startsWith('+++ b/')) {
      currentFile = normalizePath(line.slice('+++ b/'.length));
      continue;
    }

    if (!isAddedDiffLine(line)) {
      continue;
    }

    const content = line.slice(1);
    for (const rule of RISK_PATTERNS) {
      if (rule.appliesToFile && !rule.appliesToFile(currentFile)) {
        continue;
      }

      if (rule.pattern.test(content)) {
        findings.push({
          severity: rule.severity,
          kind: rule.kind,
          file: currentFile,
          source: 'added_diff_line',
          reviewIntent: 'pattern-match-review',
          sample: content.trim().slice(0, 180),
        });
      }
    }
  }

  return findings;
}

function scanChangedFiles(changedFiles) {
  return changedFiles
    .filter((filePath) => HIGH_RISK_FILE_PATTERNS.some((pattern) => pattern.test(filePath)))
    .map((filePath) => ({
      severity: 'medium',
      kind: 'sensitive-file-changed',
      file: filePath,
      source: 'changed_file',
      reviewIntent: 'sensitive-path-review',
      sample: filePath,
    }));
}

function dedupeFindings(findings) {
  const seen = new Set();
  return findings.filter((finding) => {
    const key = `${finding.severity}:${finding.kind}:${finding.file}:${finding.sample}`;
    if (seen.has(key)) {
      return false;
    }
    seen.add(key);
    return true;
  });
}

function summarizeFindings(findings) {
  return findings.reduce(
    (summary, finding) => {
      summary.total += 1;
      if (finding.severity === 'high') {
        summary.high += 1;
      }
      if (finding.severity === 'medium') {
        summary.medium += 1;
      }
      summary.bySource[finding.source] = (summary.bySource[finding.source] || 0) + 1;
      return summary;
    },
    {
      total: 0,
      high: 0,
      medium: 0,
      bySource: {},
    }
  );
}

function resolveScanSource(env, name, fallback) {
  return env?.[name] ? `env:${name}` : fallback;
}

function buildReport({ changedFiles, findings, baseRef = DEFAULT_BASE_REF, env = process.env }) {
  return {
    status: findings.some((finding) => finding.severity === 'high') ? 'review_required' : 'advisory',
    scan: {
      baseRef,
      headRef: 'HEAD',
      diffRange: `${baseRef}...HEAD`,
      changedFilesSource: resolveScanSource(env, 'SECURITY_RISK_CHANGED_FILES', 'git diff --name-only'),
      diffSource: resolveScanSource(env, 'SECURITY_RISK_DIFF', 'git diff --unified=0'),
      note: 'security-risk scans the full branch diff range. Findings may include branch-history noise that was introduced before the latest PR update; treat sensitive-file findings as review prompts, not CI blockers.',
    },
    summary: summarizeFindings(findings),
    changedFiles,
    findings,
  };
}

function writeReport({ repoRoot, env, report }) {
  const outputDir = resolveOutputDir(repoRoot, env);
  fs.mkdirSync(outputDir, { recursive: true });
  const reportPath = path.join(outputDir, REPORT_FILE);
  fs.writeFileSync(reportPath, `${JSON.stringify(report, null, 2)}\n`, 'utf8');
  return reportPath;
}

async function main(argv = [], deps = {}) {
  const repoRoot = deps.repoRoot || getRepoRoot();
  const env = deps.env || process.env;
  const baseRef = argv[0] || env.SECURITY_RISK_BASE_REF || DEFAULT_BASE_REF;
  const changedFiles = readChangedFiles({
    repoRoot,
    baseRef,
    env,
    spawnSyncImpl: deps.spawnSyncImpl,
  });
  const diffText = readDiff({
    repoRoot,
    baseRef,
    env,
    spawnSyncImpl: deps.spawnSyncImpl,
  });
  const findings = dedupeFindings([
    ...scanChangedFiles(changedFiles),
    ...scanDiffText(diffText),
  ]);
  const report = buildReport({ changedFiles, findings, baseRef, env });
  const reportPath = writeReport({ repoRoot, env, report });
  const writeStdout = deps.writeStdout || ((text) => process.stdout.write(text));
  const writeStderr = deps.writeStderr || ((text) => process.stderr.write(text));

  writeStdout(`[security-risk] Wrote ${path.relative(repoRoot, reportPath)} with ${findings.length} finding(s).\n`);

  if (findings.length > 0) {
    writeStderr(`[security-risk] Review ${findings.length} risk finding(s) before merging.\n`);
  }

  return 0;
}

module.exports = {
  buildReport,
  main,
  scanChangedFiles,
  scanDiffText,
};
