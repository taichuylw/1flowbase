const path = require('node:path');
const { spawnSync } = require('node:child_process');

const MAX_BUFFER_BYTES = 16 * 1024 * 1024;
const SCRIPT_LABEL = '1flowbase-merge-current-to-main-latest';

function getRepoRoot() {
  return path.resolve(__dirname, '..', '..', '..');
}

function usage(writeStdout = (text) => process.stdout.write(text)) {
  writeStdout(`Usage: node scripts/node/cli/merge-current-to-main-latest.js [options]

Merges the current branch into main, pushes main, then merges main into latest and pushes latest.
Any git failure stops the script immediately and leaves the repository at the failing step.

Options:
  --remote <name>    Git remote to push to. Defaults to origin.
  --main <branch>    Main branch name. Defaults to main.
  --latest <branch>  Latest branch name. Defaults to latest.
  --allow-dirty      Allow running with local uncommitted changes.
  -h, --help         Show this help.
`);
}

function readValue(argv, index, flag) {
  const value = argv[index + 1];

  if (!value || value.startsWith('-')) {
    throw new Error(`${flag} requires a value`);
  }

  return value;
}

function assertGitName(label, value) {
  if (!value || !value.trim()) {
    throw new Error(`${label} cannot be empty`);
  }

  if (value.startsWith('-')) {
    throw new Error(`${label} cannot start with '-'`);
  }
}

function parseCliArgs(argv) {
  const options = {
    allowDirty: false,
    help: false,
    latestBranch: 'latest',
    mainBranch: 'main',
    remote: 'origin',
  };

  for (let index = 0; index < argv.length; index += 1) {
    const arg = argv[index];

    if (arg === '-h' || arg === '--help') {
      options.help = true;
      continue;
    }

    if (arg === '--allow-dirty') {
      options.allowDirty = true;
      continue;
    }

    if (arg === '--remote') {
      options.remote = readValue(argv, index, arg);
      index += 1;
      continue;
    }

    if (arg === '--main') {
      options.mainBranch = readValue(argv, index, arg);
      index += 1;
      continue;
    }

    if (arg === '--latest') {
      options.latestBranch = readValue(argv, index, arg);
      index += 1;
      continue;
    }

    throw new Error(`Unknown argument: ${arg}`);
  }

  assertGitName('remote', options.remote);
  assertGitName('main branch', options.mainBranch);
  assertGitName('latest branch', options.latestBranch);

  return options;
}

function writeIfPresent(writer, text) {
  if (text) {
    writer(text);
  }
}

function runGit({
  args,
  captureStdout = false,
  label,
  repoRoot,
  spawnSyncImpl = spawnSync,
  writeStdout = (text) => process.stdout.write(text),
  writeStderr = (text) => process.stderr.write(text),
}) {
  const result = spawnSyncImpl('git', args, {
    cwd: repoRoot,
    encoding: 'utf8',
    maxBuffer: MAX_BUFFER_BYTES,
    stdio: captureStdout ? ['ignore', 'pipe', 'pipe'] : ['inherit', 'inherit', 'inherit'],
  });

  if (result.error) {
    throw new Error(`${label} failed: ${result.error.message}`);
  }

  if (!captureStdout) {
    writeIfPresent(writeStdout, result.stdout);
    writeIfPresent(writeStderr, result.stderr);
  }

  if (result.status !== 0) {
    if (captureStdout) {
      writeIfPresent(writeStderr, result.stderr);
    }
    throw new Error(`${label} failed with exit code ${result.status ?? 1}`);
  }

  return result.stdout || '';
}

function log(message, writeStdout = (text) => process.stdout.write(text)) {
  writeStdout(`[${SCRIPT_LABEL}] ${message}\n`);
}

function getCurrentBranch(deps) {
  return runGit({
    ...deps,
    args: ['branch', '--show-current'],
    captureStdout: true,
    label: 'detect current branch',
  }).trim();
}

function assertCleanWorktree(deps) {
  const status = runGit({
    ...deps,
    args: ['status', '--porcelain'],
    captureStdout: true,
    label: 'check worktree status',
  }).trim();

  if (status) {
    throw new Error('Worktree is not clean. Commit or stash local changes before running this script.');
  }
}

function runGitStep(args, label, deps) {
  runGit({
    ...deps,
    args,
    label,
  });
}

function runMergeCurrentToMainLatest({
  options,
  repoRoot = getRepoRoot(),
  spawnSyncImpl = spawnSync,
  writeStdout = (text) => process.stdout.write(text),
  writeStderr = (text) => process.stderr.write(text),
} = {}) {
  const selectedOptions = options || parseCliArgs([]);
  const deps = {
    repoRoot,
    spawnSyncImpl,
    writeStdout,
    writeStderr,
  };

  try {
    const currentBranch = getCurrentBranch(deps);

    if (!currentBranch) {
      throw new Error('Current HEAD is detached; switch to a named branch first.');
    }

    if (!selectedOptions.allowDirty) {
      assertCleanWorktree(deps);
    }

    log(`current branch: ${currentBranch}`, writeStdout);
    log(`fetching ${selectedOptions.remote}/${selectedOptions.mainBranch} and ${selectedOptions.remote}/${selectedOptions.latestBranch}`, writeStdout);
    runGitStep(
      ['fetch', selectedOptions.remote, selectedOptions.mainBranch, selectedOptions.latestBranch],
      'fetch target branches',
      deps
    );

    log(`switching to ${selectedOptions.mainBranch}`, writeStdout);
    runGitStep(['switch', selectedOptions.mainBranch], `switch to ${selectedOptions.mainBranch}`, deps);

    log(`fast-forwarding ${selectedOptions.mainBranch} from ${selectedOptions.remote}`, writeStdout);
    runGitStep(
      ['pull', '--ff-only', selectedOptions.remote, selectedOptions.mainBranch],
      `pull ${selectedOptions.mainBranch}`,
      deps
    );

    log(`merging ${currentBranch} into ${selectedOptions.mainBranch}`, writeStdout);
    runGitStep(
      ['merge', '--no-edit', currentBranch],
      `merge ${currentBranch} into ${selectedOptions.mainBranch}`,
      deps
    );

    log(`pushing ${selectedOptions.mainBranch} to ${selectedOptions.remote}`, writeStdout);
    runGitStep(
      ['push', selectedOptions.remote, selectedOptions.mainBranch],
      `push ${selectedOptions.mainBranch}`,
      deps
    );

    log(`switching to ${selectedOptions.latestBranch}`, writeStdout);
    runGitStep(['switch', selectedOptions.latestBranch], `switch to ${selectedOptions.latestBranch}`, deps);

    log(`fast-forwarding ${selectedOptions.latestBranch} from ${selectedOptions.remote}`, writeStdout);
    runGitStep(
      ['pull', '--ff-only', selectedOptions.remote, selectedOptions.latestBranch],
      `pull ${selectedOptions.latestBranch}`,
      deps
    );

    log(`merging ${selectedOptions.mainBranch} into ${selectedOptions.latestBranch}`, writeStdout);
    runGitStep(
      ['merge', '--no-edit', selectedOptions.mainBranch],
      `merge ${selectedOptions.mainBranch} into ${selectedOptions.latestBranch}`,
      deps
    );

    log(`pushing ${selectedOptions.latestBranch} to ${selectedOptions.remote}`, writeStdout);
    runGitStep(
      ['push', selectedOptions.remote, selectedOptions.latestBranch],
      `push ${selectedOptions.latestBranch}`,
      deps
    );

    log('done', writeStdout);
    return 0;
  } catch (error) {
    writeStderr(`[${SCRIPT_LABEL}] ${error.message}\n`);
    return 1;
  }
}

function main(argv = [], deps = {}) {
  const options = parseCliArgs(argv);

  if (options.help) {
    usage(deps.writeStdout);
    return 0;
  }

  return runMergeCurrentToMainLatest({
    ...deps,
    options,
  });
}

module.exports = {
  getCurrentBranch,
  main,
  parseCliArgs,
  runMergeCurrentToMainLatest,
  usage,
};
