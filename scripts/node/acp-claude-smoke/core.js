const { spawn } = require('node:child_process');
const fs = require('node:fs');
const path = require('node:path');
const readline = require('node:readline');

const DEFAULT_ADAPTER_PACKAGE = '@agentclientprotocol/claude-agent-acp@0.45.0';
const DEFAULT_OUT_DIR = path.join('tmp', 'test-governance', 'acp-claude-smoke');
const DEFAULT_PROMPT = 'Think briefly, then answer with exactly: ACP_OK. Do not use tools.';
const DEFAULT_NODE_BIN = process.env.CLAUDE_CODE_NODE_BIN || path.dirname(process.execPath);
const DEFAULT_CLAUDE_EXECUTABLE = process.env.CLAUDE_CODE_EXECUTABLE || 'claude';

function usage(writeStdout = (text) => process.stdout.write(text)) {
  writeStdout(
    [
      'Usage: node scripts/node/cli/acp-claude-smoke.js [options]',
      '',
      'Runs Claude Code through an ACP adapter and records session/update evidence.',
      '',
      'Options:',
      '  --prompt <text>              Prompt to send',
      '  --out-dir <dir>             Evidence directory',
      '  --cwd <dir>                 ACP session cwd',
      '  --adapter <pkg>             npx package, default @agentclientprotocol/claude-agent-acp@0.45.0',
      '  --node-bin <dir>            Directory prepended to PATH, default current Node bin',
      '  --claude-executable <path>  Claude Code executable path, default claude',
      '  --model <id>                Optional ACP model config value',
      '  --effort <level>            Optional ACP effort config value, default high',
      '  --no-effort                 Do not set effort',
      '  --timeout-ms <ms>           Timeout, default 180000',
      '  --max-thinking-tokens <n>   MAX_THINKING_TOKENS env value, default 1024',
      '  --no-default-settings       Disable Claude user/project/local settings',
      '  --allow-missing-thought     Exit 0 even when agent_thought_chunk is absent',
      '  -h, --help                  Show help',
      '',
    ].join('\n')
  );
}

function takeValue(argv, index, flag) {
  const value = argv[index + 1];
  if (!value || value.startsWith('--')) {
    throw new Error(`${flag} requires a value`);
  }
  return value;
}

function parseCliArgs(argv = []) {
  const options = {
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
  };

  for (let index = 0; index < argv.length; index += 1) {
    const arg = argv[index];
    if (arg === '-h' || arg === '--help') {
      options.help = true;
    } else if (arg === '--prompt') {
      options.prompt = takeValue(argv, index, arg);
      index += 1;
    } else if (arg === '--out-dir') {
      options.outDir = takeValue(argv, index, arg);
      index += 1;
    } else if (arg === '--cwd') {
      options.cwd = takeValue(argv, index, arg);
      index += 1;
    } else if (arg === '--adapter') {
      options.adapterPackage = takeValue(argv, index, arg);
      index += 1;
    } else if (arg === '--node-bin') {
      options.nodeBin = takeValue(argv, index, arg);
      index += 1;
    } else if (arg === '--claude-executable') {
      options.claudeExecutable = takeValue(argv, index, arg);
      index += 1;
    } else if (arg === '--model') {
      options.model = takeValue(argv, index, arg);
      index += 1;
    } else if (arg === '--effort') {
      options.effort = takeValue(argv, index, arg);
      index += 1;
    } else if (arg === '--no-effort') {
      options.effort = null;
    } else if (arg === '--timeout-ms') {
      options.timeoutMs = Number(takeValue(argv, index, arg));
      index += 1;
    } else if (arg === '--max-thinking-tokens') {
      options.maxThinkingTokens = takeValue(argv, index, arg);
      index += 1;
    } else if (arg === '--no-default-settings') {
      options.useDefaultSettings = false;
    } else if (arg === '--allow-missing-thought') {
      options.requireThought = false;
    } else {
      throw new Error(`Unknown option: ${arg}`);
    }
  }

  if (!Number.isFinite(options.timeoutMs) || options.timeoutMs <= 0) {
    throw new Error('--timeout-ms must be a positive number');
  }

  return options;
}

function summarizeAcpEvidence({ updates, notifications, agentRequests, errors, paths, prompt, cwd, extra }) {
  const updateCounts = {};
  for (const item of updates) {
    const kind = item?.update?.sessionUpdate ?? 'unknown';
    updateCounts[kind] = (updateCounts[kind] ?? 0) + 1;
  }

  const thoughtText = updates
    .filter((item) => item?.update?.sessionUpdate === 'agent_thought_chunk')
    .map((item) => item.update?.content?.text ?? '')
    .join('');
  const messageText = updates
    .filter((item) => item?.update?.sessionUpdate === 'agent_message_chunk')
    .map((item) => item.update?.content?.text ?? '')
    .join('');
  const rawSdkMessages = notifications
    .filter((item) => item.method === '_claude/sdkMessage')
    .map((item) => item.params?.message)
    .filter(Boolean);
  const rawSdkTypes = {};
  let rawThinkingDeltas = 0;
  let rawThinkingBlocks = 0;
  for (const message of rawSdkMessages) {
    const key =
      message.type === 'stream_event'
        ? `${message.type}:${message.event?.type ?? 'unknown'}:${message.event?.delta?.type ?? ''}`
        : `${message.type ?? 'unknown'}:${message.subtype ?? ''}`;
    rawSdkTypes[key] = (rawSdkTypes[key] ?? 0) + 1;
    if (message.type === 'stream_event' && message.event?.delta?.type === 'thinking_delta') {
      rawThinkingDeltas += 1;
    }
    if (
      message.type === 'assistant' &&
      Array.isArray(message.message?.content) &&
      message.message.content.some((content) => content?.type === 'thinking')
    ) {
      rawThinkingBlocks += 1;
    }
  }

  return {
    ok: updateCounts.agent_thought_chunk > 0 && updateCounts.agent_message_chunk > 0,
    cwd,
    prompt,
    updateCounts,
    thoughtChars: thoughtText.length,
    messageChars: messageText.length,
    thoughtPreview: thoughtText.slice(0, 300),
    messagePreview: messageText.slice(0, 300),
    agentRequestMethods: agentRequests.map((item) => item.method),
    notificationMethods: notifications.map((item) => item.method),
    rawSdkTypes,
    rawThinkingDeltas,
    rawThinkingBlocks,
    errors,
    ...paths,
    ...extra,
  };
}

function writeJsonLine(stream, message) {
  stream.write(`${JSON.stringify(message)}\n`);
}

async function runAcpClaudeSmoke(options, deps = {}) {
  const repoRoot = deps.repoRoot || process.cwd();
  const outDir = path.resolve(repoRoot, options.outDir);
  const cwd = path.resolve(repoRoot, options.cwd || path.join(options.outDir, 'workspace'));
  fs.mkdirSync(outDir, { recursive: true });
  fs.mkdirSync(cwd, { recursive: true });

  const paths = {
    rawInPath: path.join(outDir, 'acp-claude-inbound.jsonl'),
    rawOutPath: path.join(outDir, 'acp-claude-outbound.jsonl'),
    stderrPath: path.join(outDir, 'acp-claude-stderr.log'),
    summaryPath: path.join(outDir, 'acp-claude-summary.json'),
  };

  const rawIn = fs.createWriteStream(paths.rawInPath, { flags: 'w' });
  const rawOut = fs.createWriteStream(paths.rawOutPath, { flags: 'w' });
  const stderr = fs.createWriteStream(paths.stderrPath, { flags: 'w' });

  const spawnImpl = deps.spawnImpl || spawn;
  const childEnv = {
    ...process.env,
    CLAUDE_CODE_EXECUTABLE: options.claudeExecutable,
    MAX_THINKING_TOKENS: options.maxThinkingTokens,
    NO_COLOR: '1',
  };
  if (options.nodeBin) {
    childEnv.PATH = `${options.nodeBin}:${process.env.PATH ?? ''}`;
  }
  const child = spawnImpl('npx', ['--yes', options.adapterPackage], {
    cwd: repoRoot,
    stdio: ['pipe', 'pipe', 'pipe'],
    env: childEnv,
  });

  child.stderr.on('data', (chunk) => stderr.write(chunk));

  let nextId = 1;
  const pending = new Map();
  const updates = [];
  const agentRequests = [];
  const notifications = [];
  const errors = [];

  function send(message) {
    writeJsonLine(rawOut, message);
    child.stdin.write(`${JSON.stringify(message)}\n`);
  }

  function request(method, params) {
    const id = nextId;
    nextId += 1;
    send({ jsonrpc: '2.0', id, method, params });
    return new Promise((resolve, reject) => {
      pending.set(id, { resolve, reject });
    });
  }

  function respond(id, result) {
    send({ jsonrpc: '2.0', id, result });
  }

  function respondError(id, code, message) {
    send({ jsonrpc: '2.0', id, error: { code, message } });
  }

  function handleAgentRequest(message) {
    agentRequests.push(message);
    const { id, method, params } = message;
    if (method === 'session/request_permission') {
      const choices = Array.isArray(params?.options) ? params.options : [];
      const option =
        choices.find((item) => item.kind === 'allow_once') ??
        choices.find((item) => item.kind === 'allow_always') ??
        choices[0];
      respond(
        id,
        option
          ? { outcome: { outcome: 'selected', optionId: option.optionId } }
          : { outcome: { outcome: 'cancelled' } }
      );
      return;
    }
    if (method === 'fs/read_text_file') {
      respond(id, { content: '' });
      return;
    }
    if (method === 'fs/write_text_file') {
      respond(id, {});
      return;
    }
    if (typeof id !== 'undefined') {
      respondError(id, -32601, `method not implemented by smoke client: ${method}`);
    }
  }

  const rl = readline.createInterface({ input: child.stdout });
  rl.on('line', (line) => {
    const trimmed = line.trim();
    if (!trimmed) return;
    rawIn.write(`${trimmed}\n`);
    let message;
    try {
      message = JSON.parse(trimmed);
    } catch (error) {
      errors.push({ type: 'parse', line: trimmed, error: String(error) });
      return;
    }

    if (Object.prototype.hasOwnProperty.call(message, 'id') && pending.has(message.id)) {
      const waiter = pending.get(message.id);
      pending.delete(message.id);
      if (message.error) {
        waiter.reject(Object.assign(new Error(message.error.message), { rpc: message }));
      } else {
        waiter.resolve(message.result);
      }
      return;
    }

    if (message.method === 'session/update') {
      updates.push(message.params);
      return;
    }

    if (Object.prototype.hasOwnProperty.call(message, 'id')) {
      handleAgentRequest(message);
      return;
    }

    if (message.method) {
      notifications.push(message);
    }
  });

  function finish(extra = {}) {
    const summary = summarizeAcpEvidence({
      updates,
      notifications,
      agentRequests,
      errors,
      paths,
      prompt: options.prompt,
      cwd,
      extra,
    });
    fs.writeFileSync(paths.summaryPath, `${JSON.stringify(summary, null, 2)}\n`);
    return summary;
  }

  const timer = setTimeout(() => {
    const summary = finish({ timedOut: true });
    child.kill('SIGTERM');
    if (options.requireThought && !summary.ok) {
      process.exitCode = 2;
    }
  }, options.timeoutMs);

  try {
    const initialize = await request('initialize', {
      protocolVersion: 1,
      clientCapabilities: {
        fs: {
          readTextFile: true,
          writeTextFile: true,
        },
        auth: {
          terminal: false,
        },
        _meta: {
          terminal_output: false,
        },
      },
    });

    const sessionOptions = options.useDefaultSettings
      ? { tools: [] }
      : { tools: [], settingSources: [] };
    const session = await request('session/new', {
      cwd,
      mcpServers: [],
      _meta: {
        disableBuiltInTools: true,
        claudeCode: {
          emitRawSDKMessages: true,
          options: sessionOptions,
        },
      },
    });

    let modelResult = null;
    if (options.model) {
      modelResult = await request('session/set_config_option', {
        sessionId: session.sessionId,
        configId: 'model',
        value: options.model,
      });
    }

    let effortResult = null;
    if (options.effort) {
      effortResult = await request('session/set_config_option', {
        sessionId: session.sessionId,
        configId: 'effort',
        value: options.effort,
      });
    }

    const promptResult = await request('session/prompt', {
      sessionId: session.sessionId,
      prompt: [{ type: 'text', text: options.prompt }],
    });

    clearTimeout(timer);
    child.stdin.end();
    child.kill('SIGTERM');
    return finish({ initialize, session, modelResult, effortResult, promptResult });
  } catch (error) {
    clearTimeout(timer);
    child.kill('SIGTERM');
    return finish({
      failed: true,
      failureMessage: error instanceof Error ? error.message : String(error),
      failureRpc: error?.rpc,
    });
  }
}

async function main(argv = [], deps = {}) {
  const options = parseCliArgs(argv);
  if (options.help) {
    usage(deps.writeStdout);
    return 0;
  }

  const summary = await runAcpClaudeSmoke(options, deps);
  const output = `${JSON.stringify(summary, null, 2)}\n`;
  if (summary.ok || !options.requireThought) {
    (deps.writeStdout || process.stdout.write.bind(process.stdout))(output);
    return 0;
  }
  (deps.writeStderr || process.stderr.write.bind(process.stderr))(output);
  return summary.failed ? 1 : 2;
}

module.exports = {
  DEFAULT_ADAPTER_PACKAGE,
  DEFAULT_CLAUDE_EXECUTABLE,
  DEFAULT_NODE_BIN,
  DEFAULT_OUT_DIR,
  DEFAULT_PROMPT,
  main,
  parseCliArgs,
  runAcpClaudeSmoke,
  summarizeAcpEvidence,
  usage,
};
