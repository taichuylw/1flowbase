const test = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');

const {
  collectI18nHygieneFindings,
  main,
  scanJsonDuplicateKeys,
} = require('../core.js');

function writeFile(repoRoot, relativePath, content) {
  const absolutePath = path.join(repoRoot, relativePath);
  fs.mkdirSync(path.dirname(absolutePath), { recursive: true });
  fs.writeFileSync(absolutePath, content, 'utf8');
}

function writeI18nPair(repoRoot, owner, zhContent, enContent) {
  writeFile(repoRoot, `${owner}/i18n/zh-CN.json`, `${JSON.stringify(zhContent, null, 2)}\n`);
  writeFile(repoRoot, `${owner}/i18n/en-US.json`, `${JSON.stringify(enContent, null, 2)}\n`);
}

test('scanJsonDuplicateKeys reports duplicate keys with line evidence', () => {
  const findings = scanJsonDuplicateKeys({
    relativePath: 'web/app/src/features/example/i18n/zh-CN.json',
    content: [
      '{',
      '  "actions": {',
      '    "save": "保存",',
      '    "save": "保存资料"',
      '  }',
      '}',
    ].join('\n'),
  });

  assert.equal(findings.length, 1);
  assert.equal(findings[0].severity, 'error');
  assert.equal(findings[0].rule, 'duplicate-json-key');
  assert.equal(findings[0].key, 'actions.save');
  assert.equal(findings[0].line, 4);
});

test('collectI18nHygieneFindings reports locale and key contract errors', () => {
  const repoRoot = fs.mkdtempSync(path.join(os.tmpdir(), 'oneflowbase-i18n-hygiene-'));
  writeFile(
    repoRoot,
    'web/app/src/features/auth/i18n/zh-cn.json',
    '{"signIn":{"title":"登录"}}\n'
  );
  writeFile(
    repoRoot,
    'web/app/src/features/auth/i18n/en-US.json',
    '{"signIn":{"title":"Sign in","submit":"Sign in"}}\n'
  );

  const findings = collectI18nHygieneFindings({ repoRoot });

  assert.equal(
    findings.some((finding) => finding.rule === 'invalid-locale-file-name'),
    true
  );
  assert.equal(
    findings.some((finding) => finding.rule === 'missing-locale-file'),
    true
  );
  assert.equal(
    findings.some((finding) => finding.rule === 'locale-key-mismatch'),
    true
  );
});

test('collectI18nHygieneFindings fails duplicated values inside one owner locale', () => {
  const repoRoot = fs.mkdtempSync(path.join(os.tmpdir(), 'oneflowbase-i18n-value-'));
  writeI18nPair(
    repoRoot,
    'web/app/src/features/me',
    {
      actions: {
        save: '保存',
        submit: '保存',
      },
    },
    {
      actions: {
        save: 'Save',
        submit: 'Submit',
      },
    }
  );

  const findings = collectI18nHygieneFindings({ repoRoot });
  const duplicateValue = findings.find((finding) => finding.rule === 'duplicate-value-in-owner');

  assert.equal(duplicateValue?.severity, 'error');
  assert.equal(duplicateValue?.value, '保存');
  assert.deepEqual(duplicateValue?.keys, ['actions.save', 'actions.submit']);
});

test('collectI18nHygieneFindings warns on duplicate keys and values across owners', () => {
  const repoRoot = fs.mkdtempSync(path.join(os.tmpdir(), 'oneflowbase-i18n-cross-owner-'));
  writeI18nPair(
    repoRoot,
    'web/app/src/features/auth',
    { actions: { save: '保存' } },
    { actions: { save: 'Save' } }
  );
  writeI18nPair(
    repoRoot,
    'web/app/src/features/me',
    { actions: { save: '保存' } },
    { actions: { save: 'Save profile' } }
  );

  const findings = collectI18nHygieneFindings({ repoRoot });

  assert.equal(
    findings.some(
      (finding) => finding.rule === 'duplicate-key-across-owners' && finding.severity === 'warning'
    ),
    true
  );
  assert.equal(
    findings.some(
      (finding) => finding.rule === 'duplicate-value-across-owners' && finding.severity === 'warning'
    ),
    true
  );
});

test('main writes i18n hygiene report and fails on blocking findings', async () => {
  const repoRoot = fs.mkdtempSync(path.join(os.tmpdir(), 'oneflowbase-i18n-main-'));
  writeI18nPair(
    repoRoot,
    'web/app/src/features/auth',
    { title: '登录', submit: '登录' },
    { title: 'Sign in', submit: 'Sign in' }
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
  assert.match(stderr, /duplicate-value-in-owner/u);
  assert.equal(
    fs.existsSync(path.join(repoRoot, 'tmp', 'test-governance', 'i18n-hygiene.json')),
    true
  );
});
