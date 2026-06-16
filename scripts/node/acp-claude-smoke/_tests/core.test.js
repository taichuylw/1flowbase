const test = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');
const { EventEmitter } = require('node:events');
const { PassThrough } = require('node:stream');

const {
  DEFAULT_ADAPTER_PACKAGE,
  DEFAULT_CLAUDE_EXECUTABLE,
  DEFAULT_NODE_BIN,
  DEFAULT_OUT_DIR,
  DEFAULT_PROMPT,
  parseCliArgs,
  runAcpClaudeSmoke,
  summarizeAcpEvidence,
} = require('../core.js');

test('parseCliArgs defaults to an ACP Claude Code thought smoke', () => {
  assert.deepEqual(parseCliArgs([]), {
    help: false,
    prompt: DEFAULT_PROMPT,
    outDir: DEFAULT_OUT_DIR,
    cwd: null,
    adapterPackage: DEFAULT_ADAPTER_PACKAGE,
    nodeBin: DEFAULT_NODE_BIN,
    claudeExecutable: DEFAULT_CLAUDE_EXECUTABLE,
    model: null,
    effort: 'high',
    timeoutMs: 180000,
    maxThinkingTokens: '1024',
    useDefaultSettings: true,
    requireThought: true,
  });
});

test('parseCliArgs supports exploratory mode without requiring thought chunks', () => {
  const parsed = parseCliArgs([
    '--prompt',
    'hello',
    '--out-dir',
    'tmp/acp',
    '--cwd',
    'tmp/workspace',
    '--model',
    '1flowbase',
    '--no-effort',
    '--timeout-ms',
    '5000',
    '--max-thinking-tokens',
    '2048',
    '--no-default-settings',
    '--allow-missing-thought',
  ]);

  assert.equal(parsed.prompt, 'hello');
  assert.equal(parsed.outDir, 'tmp/acp');
  assert.equal(parsed.cwd, 'tmp/workspace');
  assert.equal(parsed.model, '1flowbase');
  assert.equal(parsed.effort, null);
  assert.equal(parsed.timeoutMs, 5000);
  assert.equal(parsed.maxThinkingTokens, '2048');
  assert.equal(parsed.useDefaultSettings, false);
  assert.equal(parsed.requireThought, false);
});

test('summarizeAcpEvidence requires both thought and message chunks', () => {
  const summary = summarizeAcpEvidence({
    cwd: '/repo',
    prompt: 'hi',
    paths: {
      rawInPath: path.join('/repo', 'in.jsonl'),
      rawOutPath: path.join('/repo', 'out.jsonl'),
      stderrPath: path.join('/repo', 'stderr.log'),
      summaryPath: path.join('/repo', 'summary.json'),
    },
    agentRequests: [],
    errors: [],
    updates: [
      {
        update: {
          sessionUpdate: 'agent_thought_chunk',
          content: { type: 'text', text: '先分析' },
        },
      },
      {
        update: {
          sessionUpdate: 'agent_message_chunk',
          content: { type: 'text', text: '最终回答' },
        },
      },
    ],
    notifications: [
      {
        method: '_claude/sdkMessage',
        params: {
          message: {
            type: 'stream_event',
            event: {
              type: 'content_block_delta',
              delta: { type: 'thinking_delta', thinking: '先分析' },
            },
          },
        },
      },
    ],
    extra: {},
  });

  assert.equal(summary.ok, true);
  assert.equal(summary.updateCounts.agent_thought_chunk, 1);
  assert.equal(summary.updateCounts.agent_message_chunk, 1);
  assert.equal(summary.thoughtChars, '先分析'.length);
  assert.equal(summary.messageChars, '最终回答'.length);
  assert.equal(summary.rawThinkingDeltas, 1);
});

test('runAcpClaudeSmoke returns timeout evidence when the ACP adapter stops responding', { timeout: 500 }, async () => {
  const repoRoot = fs.mkdtempSync(path.join(os.tmpdir(), 'acp-claude-timeout-'));
  const child = new EventEmitter();
  child.stdin = {
    write() {},
    end() {},
  };
  child.stdout = new PassThrough();
  child.stderr = new PassThrough();
  let killCount = 0;
  child.kill = () => {
    killCount += 1;
  };

  const summary = await runAcpClaudeSmoke(
    parseCliArgs(['--out-dir', 'out', '--timeout-ms', '20']),
    {
      repoRoot,
      spawnImpl: () => child,
    }
  );

  assert.equal(summary.ok, false);
  assert.equal(summary.timedOut, true);
  assert.ok(killCount > 0);
  assert.ok(fs.existsSync(path.join(repoRoot, 'out', 'acp-claude-summary.json')));
});
