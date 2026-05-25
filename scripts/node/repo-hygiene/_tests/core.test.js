const test = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');

const {
  collectRepoHygieneFindings,
  main,
  scanSourceFile,
} = require('../core.js');

function writeFile(repoRoot, relativePath, content) {
  const absolutePath = path.join(repoRoot, relativePath);
  fs.mkdirSync(path.dirname(absolutePath), { recursive: true });
  fs.writeFileSync(absolutePath, content, 'utf8');
}

test('scanSourceFile reports debt markers and weak assertions without failing the gate', () => {
  const findings = scanSourceFile({
    relativePath: 'web/app/src/features/settings/_tests/settings-page.test.tsx',
    content: [
      "test('renders settings shell', () => {",
      '  // TODO: replace legacy assertion',
      '  expect(screen.getByText("Settings")).toBeTruthy();',
      '});',
    ].join('\n'),
  });

  assert.deepEqual(
    findings.map((finding) => finding.rule),
    ['source-debt-marker', 'weak-test-assertion']
  );
  assert.equal(findings.every((finding) => finding.severity === 'warning'), true);
});

test('scanSourceFile reports low-value test smells as advisory findings', () => {
  const findings = scanSourceFile({
    relativePath: 'web/packages/api-client/src/_tests/console-frontstage.test.ts',
    content: [
      "test('frontstage transport spy is active', () => {",
      '  expect(apiFetchSpy).toHaveBeenCalledTimes(0);',
      '});',
      '',
      "test('createEmbedContext returns the provided context', () => {",
      "  expect(createEmbedContext({ applicationId: 'app-1' })).toEqual({ applicationId: 'app-1' });",
      '});',
      '',
      "test('keeps account before settings', () => {",
      '  expect(',
      '    accountLabel.compareDocumentPosition(settingsTrigger) &',
      '      Node.DOCUMENT_POSITION_FOLLOWING',
      '  ).toBeTruthy();',
      '});',
    ].join('\n'),
  });

  assert.deepEqual(
    findings.map((finding) => finding.rule),
    [
      'setup-only-test',
      'identity-wrapper-test',
      'weak-test-assertion',
    ]
  );
  assert.equal(findings.every((finding) => finding.severity === 'warning'), true);
});

test('scanSourceFile reports front-back field contract compatibility markers as warnings', () => {
  const findings = scanSourceFile({
    relativePath: 'web/app/src/features/example/api/example.ts',
    content: [
      'export function adaptExample(payload) {',
      '  // @field-contract-compat source=display_name alias=displayName remove_by=2026-06-30',
      '  return { displayName: payload.display_name };',
      '}',
    ].join('\n'),
  });

  assert.deepEqual(
    findings.map((finding) => finding.rule),
    ['field-contract-compat-marker']
  );
  assert.equal(findings[0].severity, 'warning');
});

test('collectRepoHygieneFindings reports duplicate test titles and oversized files', () => {
  const repoRoot = fs.mkdtempSync(path.join(os.tmpdir(), 'oneflowbase-repo-hygiene-'));
  writeFile(
    repoRoot,
    'web/app/src/features/example/_tests/example-a.test.ts',
    "test('duplicates title', () => {});\n"
  );
  writeFile(
    repoRoot,
    'web/app/src/features/example/_tests/example-b.test.ts',
    "test('duplicates title', () => {});\n"
  );
  writeFile(
    repoRoot,
    'api/crates/control-plane/src/large.rs',
    `${Array.from({ length: 1501 }, (_, index) => `pub const LINE_${index}: usize = ${index};`).join('\n')}\n`
  );
  writeFile(
    repoRoot,
    'web/app/src/features/example/_tests/large-page.test.tsx',
    `${Array.from({ length: 1501 }, (_, index) => `expect(row${index}).toBeInTheDocument();`).join('\n')}\n`
  );

  const findings = collectRepoHygieneFindings({ repoRoot });

  assert.equal(
    findings.some((finding) => finding.rule === 'duplicate-test-title'),
    true
  );
  assert.equal(
    findings.some((finding) => finding.rule === 'file-size-pressure'),
    true
  );
  assert.equal(
    findings.some((finding) => finding.rule === 'test-file-size-pressure'),
    true
  );
});

test('collectRepoHygieneFindings excludes tmp sandbox tests from formal quality assets', () => {
  const repoRoot = fs.mkdtempSync(path.join(os.tmpdir(), 'oneflowbase-repo-hygiene-tmp-'));
  writeFile(
    repoRoot,
    'tmp/demo/app/src/app/_tests/demo.test.tsx',
    [
      "test.only('tmp sandbox test should not block repo hygiene', () => {",
      '  expect(screen.getByText("Demo")).toBeTruthy();',
      '});',
    ].join('\n')
  );
  writeFile(
    repoRoot,
    'web/app/src/features/example/_tests/example.test.ts',
    "test('formal test stays visible', () => {});\n"
  );

  const findings = collectRepoHygieneFindings({ repoRoot });

  assert.equal(
    findings.some((finding) => finding.file.startsWith('tmp/demo/')),
    false
  );
  assert.equal(
    findings.some((finding) => finding.file === 'web/app/src/features/example/_tests/example.test.ts'),
    false
  );
});

test('main writes an advisory report and only fails on blocking findings', async () => {
  const repoRoot = fs.mkdtempSync(path.join(os.tmpdir(), 'oneflowbase-repo-hygiene-main-'));
  writeFile(
    repoRoot,
    'web/app/src/features/example/_tests/example.test.ts',
    [
      "test.only('focused test must block CI', () => {});",
      "test('legacy marker is advisory', () => {});",
    ].join('\n')
  );

  let stderr = '';
  const status = await main([], {
    repoRoot,
    writeStdout() {},
    writeStderr(text) {
      stderr += text;
    },
  });

  assert.equal(status, 1);
  assert.match(stderr, /focused-test/u);
  assert.equal(
    fs.existsSync(path.join(repoRoot, 'tmp', 'test-governance', 'repo-hygiene.json')),
    true
  );
});
