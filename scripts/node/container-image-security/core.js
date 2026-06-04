const fs = require('node:fs');
const path = require('node:path');

const { getRepoRoot } = require('../testing/warning-capture.js');

const OUTPUT_ROOT = path.join('tmp', 'test-governance');
const REPORT_JSON_FILE = 'container-image-security.json';
const REPORT_MARKDOWN_FILE = 'container-image-security.md';
const TOP_VULNERABILITY_LIMIT = 8;
const LEVEL_ORDER = new Map([
  ['high', 0],
  ['critical', 1],
]);

function toRepoRelative(repoRoot, filePath) {
  return path.relative(repoRoot, filePath).replace(/\\/gu, '/');
}

function readJsonFile(filePath) {
  return JSON.parse(fs.readFileSync(filePath, 'utf8'));
}

function normalizeImageRef(report) {
  if (report?.ArtifactName) {
    return report.ArtifactName;
  }

  const target = report?.Results?.find((result) => result.Target)?.Target || '';
  return target.replace(/\s+\(.+\)$/u, '');
}

function vulnerabilityRecords({ report, component, level }) {
  return (report?.Results || []).flatMap((result) => (
    (result.Vulnerabilities || []).map((vulnerability) => ({
      component,
      level,
      target: result.Target || '',
      severity: vulnerability.Severity || level.toUpperCase(),
      id: vulnerability.VulnerabilityID || '',
      packageName: vulnerability.PkgName || '',
      installedVersion: vulnerability.InstalledVersion || '',
      fixedVersion: vulnerability.FixedVersion || '',
      title: vulnerability.Title || '',
    }))
  ));
}

function severityRank(severity) {
  if (severity === 'CRITICAL') {
    return 0;
  }

  if (severity === 'HIGH') {
    return 1;
  }

  return 2;
}

function sortVulnerabilities(vulnerabilities) {
  return [...vulnerabilities].sort((left, right) => (
    severityRank(left.severity) - severityRank(right.severity)
      || left.packageName.localeCompare(right.packageName)
      || left.id.localeCompare(right.id)
  ));
}

function sortEvidencePaths(left, right) {
  const leftLevel = left.includes('-critical.json') ? 'critical' : 'high';
  const rightLevel = right.includes('-critical.json') ? 'critical' : 'high';

  return (LEVEL_ORDER.get(leftLevel) ?? 99) - (LEVEL_ORDER.get(rightLevel) ?? 99)
    || left.localeCompare(right);
}

function discoverTrivyReports({ repoRoot, outputDir }) {
  if (!fs.existsSync(outputDir)) {
    return [];
  }

  return fs.readdirSync(outputDir, { withFileTypes: true })
    .filter((entry) => entry.isFile())
    .map((entry) => {
      const match = entry.name.match(/^trivy-(?<component>.+)-(?<level>high|critical)\.json$/u);
      if (!match) {
        return null;
      }

      return {
        component: match.groups.component,
        level: match.groups.level,
        path: path.join(outputDir, entry.name),
        relativePath: toRepoRelative(repoRoot, path.join(outputDir, entry.name)),
      };
    })
    .filter(Boolean)
    .sort((left, right) => (
      left.component.localeCompare(right.component)
        || left.level.localeCompare(right.level)
    ));
}

function collectContainerImageSecurityReport({
  repoRoot = getRepoRoot(),
  outputRoot = OUTPUT_ROOT,
} = {}) {
  const outputDir = path.join(repoRoot, outputRoot);
  const trivyReports = discoverTrivyReports({ repoRoot, outputDir });
  const byComponent = new Map();

  for (const trivyReport of trivyReports) {
    const report = readJsonFile(trivyReport.path);
    const entry = byComponent.get(trivyReport.component) || {
      component: trivyReport.component,
      imageRef: '',
      reports: {},
      evidence: [],
      vulnerabilities: [],
    };
    entry.imageRef ||= normalizeImageRef(report);
    entry.reports[trivyReport.level] = report;
    entry.evidence.push(trivyReport.relativePath);
    entry.vulnerabilities.push(...vulnerabilityRecords({
      report,
      component: trivyReport.component,
      level: trivyReport.level,
    }));
    byComponent.set(trivyReport.component, entry);
  }

  const components = [...byComponent.values()]
    .map((entry) => {
      const highCount = vulnerabilityRecords({
        report: entry.reports.high || {},
        component: entry.component,
        level: 'high',
      }).length;
      const criticalCount = vulnerabilityRecords({
        report: entry.reports.critical || {},
        component: entry.component,
        level: 'critical',
      }).length;

      const hasFindings = highCount > 0 || criticalCount > 0;

      return {
        component: entry.component,
        imageRef: entry.imageRef,
        status: hasFindings ? 'warning' : 'passed',
        highCount,
        criticalCount,
        evidence: entry.evidence.sort(sortEvidencePaths),
        topVulnerabilities: sortVulnerabilities(entry.vulnerabilities).slice(0, TOP_VULNERABILITY_LIMIT),
      };
    })
    .sort((left, right) => (
      left.status.localeCompare(right.status)
        || left.component.localeCompare(right.component)
    ));

  const highCount = components.reduce((total, component) => total + component.highCount, 0);
  const criticalCount = components.reduce((total, component) => total + component.criticalCount, 0);
  const status = highCount > 0 || criticalCount > 0 ? 'warning' : 'passed';

  return {
    status,
    exitCode: 0,
    highCount,
    criticalCount,
    componentCount: components.length,
    reportPath: path.join(outputRoot, REPORT_JSON_FILE).replace(/\\/gu, '/'),
    markdownPath: path.join(outputRoot, REPORT_MARKDOWN_FILE).replace(/\\/gu, '/'),
    components,
  };
}

function formatContainerImageSecurityMarkdown(report) {
  const componentRows = report.components.map((component) => (
    `| \`${component.component}\` | ${component.status} | ${component.highCount} | `
      + `${component.criticalCount} | \`${component.imageRef || 'unknown'}\` |`
  ));
  const vulnerabilityRows = report.components.flatMap((component) => (
    component.topVulnerabilities.map((vulnerability) => (
      `| \`${component.component}\` | ${vulnerability.severity} | \`${vulnerability.id || 'unknown'}\` | `
        + `\`${vulnerability.packageName || 'unknown'}\` | `
        + `\`${vulnerability.installedVersion || 'unknown'}\` | `
        + `\`${vulnerability.fixedVersion || 'unfixed'}\` |`
    ))
  ));
  const evidenceLines = report.components.flatMap((component) => (
    component.evidence.map((filePath) => `- ${component.component}: ${filePath}`)
  ));

  return [
    '# Container Image Security Report',
    '',
    '## Summary',
    '',
    `- Status: ${report.status}`,
    `- Exit code: ${report.exitCode}`,
    `- Components: ${report.componentCount ?? report.components.length}`,
    `- HIGH: ${report.highCount}`,
    `- CRITICAL: ${report.criticalCount}`,
    '',
    '## Components',
    '',
    '| Component | Status | HIGH | CRITICAL | Image |',
    '| --- | --- | ---: | ---: | --- |',
    ...componentRows,
    '',
    '## Top Vulnerabilities',
    '',
    vulnerabilityRows.length === 0 ? 'No HIGH or CRITICAL vulnerabilities were captured.' : null,
    vulnerabilityRows.length > 0 ? '| Component | Severity | Vulnerability | Package | Installed | Fixed |' : null,
    vulnerabilityRows.length > 0 ? '| --- | --- | --- | --- | --- | --- |' : null,
    ...vulnerabilityRows,
    '',
    '## Evidence',
    '',
    evidenceLines.length === 0 ? 'No Trivy JSON reports were captured.' : null,
    ...evidenceLines,
  ].filter((line) => line !== null).join('\n') + '\n';
}

function writeContainerImageSecurityReports({
  repoRoot = getRepoRoot(),
  outputRoot = OUTPUT_ROOT,
} = {}) {
  const outputDir = path.join(repoRoot, outputRoot);
  fs.mkdirSync(outputDir, { recursive: true });

  const report = collectContainerImageSecurityReport({ repoRoot, outputRoot });
  const markdown = formatContainerImageSecurityMarkdown(report);
  const jsonPath = path.join(outputDir, REPORT_JSON_FILE);
  const markdownPath = path.join(outputDir, REPORT_MARKDOWN_FILE);

  fs.writeFileSync(jsonPath, `${JSON.stringify(report, null, 2)}\n`, 'utf8');
  fs.writeFileSync(markdownPath, markdown, 'utf8');

  return {
    report,
    markdown,
    jsonPath,
    markdownPath,
  };
}

module.exports = {
  collectContainerImageSecurityReport,
  formatContainerImageSecurityMarkdown,
  writeContainerImageSecurityReports,
};
