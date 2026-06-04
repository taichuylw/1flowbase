const test = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');

const {
  collectContainerImageSecurityReport,
  formatContainerImageSecurityMarkdown,
  writeContainerImageSecurityReports,
} = require('../core.js');

function writeJson(filePath, value) {
  fs.mkdirSync(path.dirname(filePath), { recursive: true });
  fs.writeFileSync(filePath, `${JSON.stringify(value, null, 2)}\n`, 'utf8');
}

function writeTrivyReport({ repoRoot, component, level, imageRef, vulnerabilities }) {
  writeJson(
    path.join(repoRoot, 'tmp', 'test-governance', `trivy-${component}-${level}.json`),
    {
      SchemaVersion: 2,
      ArtifactName: imageRef,
      Results: [
        {
          Target: `${imageRef} (debian 12.14)`,
          Type: 'debian',
          Vulnerabilities: vulnerabilities,
        },
      ],
    }
  );
}

test('collectContainerImageSecurityReport summarizes high warnings and critical blockers', () => {
  const repoRoot = fs.mkdtempSync(path.join(os.tmpdir(), 'oneflowbase-container-image-security-'));

  writeTrivyReport({
    repoRoot,
    component: 'web',
    level: 'high',
    imageRef: 'ghcr.io/taichuy/1flowbase-web:scan-1',
    vulnerabilities: [
      {
        VulnerabilityID: 'CVE-2026-6732',
        PkgName: 'libxml2',
        InstalledVersion: '2.13.9-r0',
        FixedVersion: '2.13.9-r1',
        Severity: 'HIGH',
        Title: 'libxml2 vulnerability',
      },
    ],
  });
  writeTrivyReport({
    repoRoot,
    component: 'web',
    level: 'critical',
    imageRef: 'ghcr.io/taichuy/1flowbase-web:scan-1',
    vulnerabilities: [],
  });
  writeTrivyReport({
    repoRoot,
    component: 'api-server',
    level: 'high',
    imageRef: 'ghcr.io/taichuy/1flowbase-api-server:scan-1',
    vulnerabilities: [
      {
        VulnerabilityID: 'CVE-2026-33846',
        PkgName: 'libgnutls30',
        InstalledVersion: '3.7.9-2+deb12u6',
        FixedVersion: '3.7.9-2+deb12u7',
        Severity: 'HIGH',
      },
    ],
  });
  writeTrivyReport({
    repoRoot,
    component: 'api-server',
    level: 'critical',
    imageRef: 'ghcr.io/taichuy/1flowbase-api-server:scan-1',
    vulnerabilities: [
      {
        VulnerabilityID: 'CVE-2026-33845',
        PkgName: 'libgnutls30',
        InstalledVersion: '3.7.9-2+deb12u6',
        FixedVersion: '3.7.9-2+deb12u7',
        Severity: 'CRITICAL',
      },
    ],
  });

  const report = collectContainerImageSecurityReport({ repoRoot });

  assert.equal(report.status, 'failed');
  assert.equal(report.exitCode, 1);
  assert.equal(report.highCount, 2);
  assert.equal(report.criticalCount, 1);
  assert.deepEqual(report.components.map((component) => ({
    component: component.component,
    status: component.status,
    highCount: component.highCount,
    criticalCount: component.criticalCount,
  })), [
    { component: 'api-server', status: 'failed', highCount: 1, criticalCount: 1 },
    { component: 'web', status: 'passed', highCount: 1, criticalCount: 0 },
  ]);
  assert.equal(report.components[0].imageRef, 'ghcr.io/taichuy/1flowbase-api-server:scan-1');
  assert.deepEqual(report.components[0].evidence, [
    'tmp/test-governance/trivy-api-server-high.json',
    'tmp/test-governance/trivy-api-server-critical.json',
  ]);
  assert.match(report.components[0].topVulnerabilities[0].id, /CVE-2026-33845/u);
});

test('formatContainerImageSecurityMarkdown renders component and evidence tables', () => {
  const markdown = formatContainerImageSecurityMarkdown({
    status: 'failed',
    exitCode: 1,
    highCount: 2,
    criticalCount: 1,
    components: [
      {
        component: 'api-server',
        imageRef: 'ghcr.io/taichuy/1flowbase-api-server:scan-1',
        status: 'failed',
        highCount: 1,
        criticalCount: 1,
        evidence: [
          'tmp/test-governance/trivy-api-server-high.json',
          'tmp/test-governance/trivy-api-server-critical.json',
        ],
        topVulnerabilities: [
          {
            severity: 'CRITICAL',
            id: 'CVE-2026-33845',
            packageName: 'libgnutls30',
            installedVersion: '3.7.9-2+deb12u6',
            fixedVersion: '3.7.9-2+deb12u7',
          },
        ],
      },
    ],
  });

  assert.match(markdown, /# Container Image Security Report/u);
  assert.match(markdown, /\| `api-server` \| failed \| 1 \| 1 \| `ghcr\.io\/taichuy\/1flowbase-api-server:scan-1` \|/u);
  assert.match(markdown, /\| `api-server` \| CRITICAL \| `CVE-2026-33845` \| `libgnutls30` \|/u);
  assert.match(markdown, /tmp\/test-governance\/trivy-api-server-critical\.json/u);
});

test('writeContainerImageSecurityReports writes markdown and json under test governance', () => {
  const repoRoot = fs.mkdtempSync(path.join(os.tmpdir(), 'oneflowbase-container-image-security-write-'));

  writeTrivyReport({
    repoRoot,
    component: 'web',
    level: 'high',
    imageRef: 'ghcr.io/taichuy/1flowbase-web:scan-1',
    vulnerabilities: [],
  });
  writeTrivyReport({
    repoRoot,
    component: 'web',
    level: 'critical',
    imageRef: 'ghcr.io/taichuy/1flowbase-web:scan-1',
    vulnerabilities: [],
  });

  const result = writeContainerImageSecurityReports({ repoRoot });

  assert.equal(result.report.status, 'passed');
  assert.equal(fs.existsSync(path.join(repoRoot, 'tmp', 'test-governance', 'container-image-security.md')), true);
  assert.equal(fs.existsSync(path.join(repoRoot, 'tmp', 'test-governance', 'container-image-security.json')), true);
  assert.match(
    fs.readFileSync(path.join(repoRoot, 'tmp', 'test-governance', 'container-image-security.md'), 'utf8'),
    /Container Image Security Report/u
  );
});
