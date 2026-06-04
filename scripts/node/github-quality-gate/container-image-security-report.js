const fs = require('node:fs');
const path = require('node:path');

const OUTPUT_ROOT = path.join('tmp', 'test-governance');
const CONTAINER_IMAGE_SECURITY_REPORT_FILE = 'container-image-security.json';

function readJsonFileIfPresent(filePath) {
  if (!fs.existsSync(filePath)) {
    return null;
  }

  try {
    return JSON.parse(fs.readFileSync(filePath, 'utf8'));
  } catch {
    return null;
  }
}

function normalizeContainerImageSecurityReport({ repoRoot, reportPath, report }) {
  return {
    status: report?.status || 'failed',
    exitCode: Number.isFinite(report?.exitCode) ? report.exitCode : 1,
    componentCount: Number.isFinite(report?.componentCount)
      ? report.componentCount
      : (Array.isArray(report?.components) ? report.components.length : 0),
    highCount: Number.isFinite(report?.highCount) ? report.highCount : 0,
    criticalCount: Number.isFinite(report?.criticalCount) ? report.criticalCount : 0,
    reportPath: path.relative(repoRoot, reportPath).replace(/\\/gu, '/'),
    markdownPath: report?.markdownPath || path.join(OUTPUT_ROOT, 'container-image-security.md'),
    components: Array.isArray(report?.components) ? report.components : [],
  };
}

function readContainerImageSecurityReport({ repoRoot, outputDir }) {
  const reportPath = path.join(outputDir, CONTAINER_IMAGE_SECURITY_REPORT_FILE);
  const report = readJsonFileIfPresent(reportPath);

  return report
    ? normalizeContainerImageSecurityReport({ repoRoot, reportPath, report })
    : null;
}

function formatContainerImageSecuritySummaryLine(containerImageSecurity) {
  return `- Status: ${containerImageSecurity.status}`
    + `; Components: ${containerImageSecurity.componentCount}`
    + `; HIGH: ${containerImageSecurity.highCount}`
    + `; CRITICAL: ${containerImageSecurity.criticalCount}`;
}

function formatAggregateContainerImageSecurityLine(containerImageSecurity) {
  const scope = containerImageSecurity.scope || 'container-images';
  const componentCount = containerImageSecurity.componentCount
    ?? (Array.isArray(containerImageSecurity.components) ? containerImageSecurity.components.length : 0);
  return `- ${scope}: ${containerImageSecurity.status}, components ${componentCount}, `
    + `HIGH ${containerImageSecurity.highCount}, CRITICAL ${containerImageSecurity.criticalCount}`;
}

function formatContainerImageSecurityComponentLine(component) {
  return `| \`${component.component}\` | ${component.status} | ${component.highCount} | `
    + `${component.criticalCount} | \`${component.imageRef || 'unknown'}\` |`;
}

function formatContainerImageSecurityVulnerabilityLine({ component, vulnerability }) {
  return `| \`${component.component}\` | ${vulnerability.severity || 'unknown'} | `
    + `\`${vulnerability.id || 'unknown'}\` | `
    + `\`${vulnerability.packageName || 'unknown'}\` | `
    + `\`${vulnerability.installedVersion || 'unknown'}\` | `
    + `\`${vulnerability.fixedVersion || 'unfixed'}\` |`;
}

function buildContainerImageSecurityMarkdownLines(containerImageSecurityReports) {
  if (containerImageSecurityReports.length === 0) {
    return [
      '## Container Image Security',
      '',
      'No container image security reports were captured.',
      '',
    ];
  }

  const componentRows = containerImageSecurityReports.flatMap((report) => report.components || []);
  const vulnerabilityRows = componentRows.flatMap((component) => (
    (component.topVulnerabilities || []).slice(0, 5).map((vulnerability) => (
      formatContainerImageSecurityVulnerabilityLine({ component, vulnerability })
    ))
  ));

  return [
    '## Container Image Security',
    '',
    ...containerImageSecurityReports.map(formatAggregateContainerImageSecurityLine),
    '',
    '| Component | Status | HIGH | CRITICAL | Image |',
    '| --- | --- | ---: | ---: | --- |',
    ...componentRows.map(formatContainerImageSecurityComponentLine),
    '',
    vulnerabilityRows.length > 0 ? '### Top Vulnerabilities' : null,
    vulnerabilityRows.length > 0 ? '' : null,
    vulnerabilityRows.length > 0 ? '| Component | Severity | Vulnerability | Package | Installed | Fixed |' : null,
    vulnerabilityRows.length > 0 ? '| --- | --- | --- | --- | --- | --- |' : null,
    ...vulnerabilityRows,
    '',
  ].filter((line) => line !== null);
}

module.exports = {
  buildContainerImageSecurityMarkdownLines,
  formatContainerImageSecurityComponentLine,
  formatContainerImageSecuritySummaryLine,
  formatContainerImageSecurityVulnerabilityLine,
  readContainerImageSecurityReport,
};
