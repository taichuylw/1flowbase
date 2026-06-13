const fs = require('node:fs');
const path = require('node:path');

const RELEASES = new Set(['patch', 'minor', 'major']);
const RELEASE_ALIASES = new Map([
  ['0', 'patch'],
  ['1', 'minor'],
  ['2', 'major'],
]);
const VERSION_SOURCE_FILE = 'VERSION';

function getRepoRoot() {
  return path.resolve(__dirname, '..', '..', '..');
}

function isSemver(version) {
  return /^\d+\.\d+\.\d+$/u.test(version);
}

function bumpSemver(version, release) {
  if (!isSemver(version)) {
    throw new Error(`无法升级非 x.y.z 版本：${version}`);
  }
  if (!RELEASES.has(release)) {
    throw new Error(`未知升级类型：${release}`);
  }

  const [major, minor, patch] = version.split('.').map((part) => Number.parseInt(part, 10));

  if (release === 'major') {
    return `${major + 1}.0.0`;
  }
  if (release === 'minor') {
    return `${major}.${minor + 1}.0`;
  }
  return `${major}.${minor}.${patch + 1}`;
}

function usage(writeStdout = (text) => process.stdout.write(text)) {
  writeStdout(`Usage: node scripts/node/cli/bump-version.js [0|1|2|patch|minor|major] [--dry-run]
       node scripts/node/cli/bump-version.js --to <x.y.z> [--dry-run]

Defaults to applying a patch bump.
Release aliases: 0/patch, 1/minor, 2/major.
Reads the current repository version from VERSION and writes one resolved version everywhere.
Targets owned frontend packages, owned Rust backend packages, and Cargo.lock owned entries.
Plugin manifests, Docker env files, and third-party image tags are not touched.
`);
}

function parseCliArgs(argv) {
  if (argv.includes('-h') || argv.includes('--help')) {
    return {
      dryRun: false,
      help: true,
      release: 'patch',
      targetVersion: null,
    };
  }

  const options = {
    dryRun: false,
    help: false,
    release: 'patch',
    targetVersion: null,
  };
  let explicitRelease = false;

  for (let index = 0; index < argv.length; index += 1) {
    const arg = argv[index];

    if (arg === '--dry-run') {
      options.dryRun = true;
      continue;
    }

    if (arg === '--to') {
      const value = argv[index + 1];
      if (!value) {
        throw new Error('--to 需要提供 x.y.z 版本号');
      }
      if (!isSemver(value)) {
        throw new Error(`--to 只支持 x.y.z 版本号：${value}`);
      }
      options.targetVersion = value;
      index += 1;
      continue;
    }

    const release = RELEASE_ALIASES.get(arg) || arg;
    if (RELEASES.has(release)) {
      if (explicitRelease) {
        throw new Error(`重复的升级类型：${arg}`);
      }
      options.release = release;
      explicitRelease = true;
      continue;
    }

    throw new Error(`未知参数：${arg}`);
  }

  if (options.targetVersion && explicitRelease) {
    throw new Error('--to 不能和 0/1/2/patch/minor/major 同时使用');
  }

  return options;
}

function resolveNextVersion(version, options) {
  return options.targetVersion || bumpSemver(version, options.release);
}

function relativePath(repoRoot, filePath) {
  return path.relative(repoRoot, filePath).split(path.sep).join('/');
}

function readIfExists(filePath) {
  if (!fs.existsSync(filePath)) {
    return null;
  }
  return fs.readFileSync(filePath, 'utf8');
}

function readVersionSource(repoRoot) {
  const filePath = path.join(repoRoot, VERSION_SOURCE_FILE);
  const text = readIfExists(filePath);
  if (text === null) {
    throw new Error(`缺少统一版本源：${VERSION_SOURCE_FILE}`);
  }

  const version = text.trim();
  if (!isSemver(version)) {
    throw new Error(`${VERSION_SOURCE_FILE} 只支持 x.y.z 版本号：${version || '(empty)'}`);
  }

  return { filePath, version };
}

function pushChange(changes, repoRoot, filePath, label, oldVersion, newVersion) {
  if (oldVersion === newVersion) {
    return;
  }
  changes.push({
    file: relativePath(repoRoot, filePath),
    label,
    oldVersion,
    newVersion,
  });
}

function getMutableText(mutatedFiles, filePath) {
  if (mutatedFiles.has(filePath)) {
    return mutatedFiles.get(filePath);
  }
  return fs.readFileSync(filePath, 'utf8');
}

function setMutableText(mutatedFiles, filePath, text) {
  mutatedFiles.set(filePath, text);
}

function updateVersionSource({ changes, currentVersion, filePath, mutatedFiles, repoRoot, targetVersion }) {
  pushChange(changes, repoRoot, filePath, 'repository version source', currentVersion, targetVersion);
  if (currentVersion !== targetVersion) {
    setMutableText(mutatedFiles, filePath, `${targetVersion}\n`);
  }
}

function discoverFrontendPackageFiles(repoRoot) {
  const packageFiles = [];
  const appPackage = path.join(repoRoot, 'web', 'app', 'package.json');
  if (fs.existsSync(appPackage)) {
    packageFiles.push(appPackage);
  }

  const packagesDir = path.join(repoRoot, 'web', 'packages');
  if (fs.existsSync(packagesDir)) {
    const entries = fs.readdirSync(packagesDir, { withFileTypes: true });
    for (const entry of entries.sort((left, right) => left.name.localeCompare(right.name))) {
      if (!entry.isDirectory()) {
        continue;
      }
      const packageFile = path.join(packagesDir, entry.name, 'package.json');
      if (fs.existsSync(packageFile)) {
        packageFiles.push(packageFile);
      }
    }
  }

  return packageFiles;
}

function updateFrontendVersions({ changes, mutatedFiles, repoRoot, targetVersion }) {
  for (const filePath of discoverFrontendPackageFiles(repoRoot)) {
    const packageJson = JSON.parse(getMutableText(mutatedFiles, filePath));
    if (!packageJson.version) {
      continue;
    }
    const label = `frontend ${packageJson.name || relativePath(repoRoot, filePath)}`;

    pushChange(changes, repoRoot, filePath, label, packageJson.version, targetVersion);

    if (packageJson.version !== targetVersion) {
      packageJson.version = targetVersion;
      setMutableText(mutatedFiles, filePath, `${JSON.stringify(packageJson, null, 2)}\n`);
    }
  }
}

function sectionBounds(text, sectionName) {
  const headerPattern = new RegExp(`^\\[${sectionName.replace(/[.*+?^${}()|[\]\\]/gu, '\\$&')}\\]\\s*$`, 'mu');
  const match = headerPattern.exec(text);
  if (!match) {
    return null;
  }

  const start = match.index + match[0].length;
  const rest = text.slice(start);
  const nextSection = /^\[[^\]]+\]\s*$/mu.exec(rest);
  const end = nextSection ? start + nextSection.index : text.length;
  return { start, end };
}

function readTomlSectionValue(text, sectionName, key) {
  const bounds = sectionBounds(text, sectionName);
  if (!bounds) {
    return null;
  }

  const section = text.slice(bounds.start, bounds.end);
  const escapedKey = key.replace(/[.*+?^${}()|[\]\\]/gu, '\\$&');
  const match = new RegExp(`^\\s*${escapedKey}\\s*=\\s*"([^"]+)"\\s*$`, 'mu').exec(section);
  return match ? match[1] : null;
}

function hasTomlWorkspaceVersion(text) {
  const bounds = sectionBounds(text, 'package');
  if (!bounds) {
    return false;
  }
  return /^\s*version\.workspace\s*=\s*true\s*$/mu.test(text.slice(bounds.start, bounds.end));
}

function replaceTomlSectionValue(text, sectionName, key, value) {
  const bounds = sectionBounds(text, sectionName);
  if (!bounds) {
    throw new Error(`找不到 TOML section：[${sectionName}]`);
  }

  const before = text.slice(0, bounds.start);
  const section = text.slice(bounds.start, bounds.end);
  const after = text.slice(bounds.end);
  const escapedKey = key.replace(/[.*+?^${}()|[\]\\]/gu, '\\$&');
  const pattern = new RegExp(`^(\\s*${escapedKey}\\s*=\\s*)"([^"]+)"(\\s*)$`, 'mu');

  if (!pattern.test(section)) {
    throw new Error(`找不到 TOML 字段：${sectionName}.${key}`);
  }

  return `${before}${section.replace(pattern, `$1"${value}"$3`)}${after}`;
}

function parseCargoWorkspaceMembers(text) {
  const bounds = sectionBounds(text, 'workspace');
  if (!bounds) {
    return [];
  }

  const section = text.slice(bounds.start, bounds.end);
  const match = /members\s*=\s*\[([\s\S]*?)\]/u.exec(section);
  if (!match) {
    return [];
  }

  const members = [];
  const memberPattern = /"([^"]+)"/gu;
  let memberMatch = memberPattern.exec(match[1]);
  while (memberMatch) {
    members.push(memberMatch[1]);
    memberMatch = memberPattern.exec(match[1]);
  }
  return members;
}

function updateBackendVersions({ backendVersions, changes, mutatedFiles, repoRoot, targetVersion }) {
  const apiRoot = path.join(repoRoot, 'api');
  const workspaceFile = path.join(apiRoot, 'Cargo.toml');
  const workspaceText = readIfExists(workspaceFile);

  if (!workspaceText) {
    return;
  }

  const workspaceVersion = readTomlSectionValue(workspaceText, 'workspace.package', 'version');
  if (!workspaceVersion) {
    throw new Error('api/Cargo.toml 缺少 [workspace.package].version');
  }

  pushChange(changes, repoRoot, workspaceFile, 'backend workspace package', workspaceVersion, targetVersion);
  if (workspaceVersion !== targetVersion) {
    setMutableText(
      mutatedFiles,
      workspaceFile,
      replaceTomlSectionValue(workspaceText, 'workspace.package', 'version', targetVersion)
    );
  }

  for (const member of parseCargoWorkspaceMembers(workspaceText)) {
    const cargoFile = path.join(apiRoot, member, 'Cargo.toml');
    const memberText = readIfExists(cargoFile);
    if (!memberText) {
      continue;
    }

    const packageName = readTomlSectionValue(memberText, 'package', 'name');
    if (!packageName) {
      continue;
    }

    if (hasTomlWorkspaceVersion(memberText)) {
      backendVersions.set(`cargo:${packageName}`, targetVersion);
      continue;
    }

    const packageVersion = readTomlSectionValue(memberText, 'package', 'version');
    if (!packageVersion) {
      continue;
    }

    backendVersions.set(`cargo:${packageName}`, targetVersion);
    pushChange(changes, repoRoot, cargoFile, `backend ${packageName}`, packageVersion, targetVersion);
    if (packageVersion !== targetVersion) {
      setMutableText(
        mutatedFiles,
        cargoFile,
        replaceTomlSectionValue(memberText, 'package', 'version', targetVersion)
      );
    }
  }
}

function updateCargoLockVersions({ backendVersions, changes, mutatedFiles, repoRoot }) {
  const lockFile = path.join(repoRoot, 'api', 'Cargo.lock');
  const lockText = readIfExists(lockFile);
  if (!lockText || backendVersions.size === 0) {
    return;
  }

  const chunks = lockText.split(/(?=^\[\[package\]\]\s*$)/mu);
  let changed = false;
  const nextChunks = chunks.map((chunk) => {
    const nameMatch = /^name\s*=\s*"([^"]+)"\s*$/mu.exec(chunk);
    if (!nameMatch) {
      return chunk;
    }

    const nextVersion = backendVersions.get(`cargo:${nameMatch[1]}`);
    if (!nextVersion) {
      return chunk;
    }

    const versionMatch = /^version\s*=\s*"([^"]+)"\s*$/mu.exec(chunk);
    if (!versionMatch || versionMatch[1] === nextVersion) {
      return chunk;
    }

    changed = true;
    pushChange(changes, repoRoot, lockFile, `cargo lock ${nameMatch[1]}`, versionMatch[1], nextVersion);
    return chunk.replace(/^(version\s*=\s*)"([^"]+)"(\s*)$/mu, `$1"${nextVersion}"$3`);
  });

  if (changed) {
    setMutableText(mutatedFiles, lockFile, nextChunks.join(''));
  }
}

function writeSummary({ changes, dryRun, writeStdout }) {
  if (changes.length === 0) {
    writeStdout('[1flowbase-bump-version] no version changes needed\n');
    return;
  }

  writeStdout(`[1flowbase-bump-version] ${dryRun ? 'dry-run ' : ''}${changes.length} change(s)\n`);
  for (const change of changes) {
    writeStdout(`- ${change.file}: ${change.label} ${change.oldVersion} -> ${change.newVersion}\n`);
  }
}

function runVersionBump({
  repoRoot = getRepoRoot(),
  options = parseCliArgs([]),
  writeStdout = (text) => process.stdout.write(text),
} = {}) {
  if (options.help) {
    usage(writeStdout);
    return 0;
  }

  const changes = [];
  const mutatedFiles = new Map();
  const backendVersions = new Map();
  const versionSource = readVersionSource(repoRoot);
  const targetVersion = resolveNextVersion(versionSource.version, options);

  updateVersionSource({
    changes,
    currentVersion: versionSource.version,
    filePath: versionSource.filePath,
    mutatedFiles,
    repoRoot,
    targetVersion,
  });
  updateFrontendVersions({
    changes,
    mutatedFiles,
    repoRoot,
    targetVersion,
  });
  updateBackendVersions({
    backendVersions,
    changes,
    mutatedFiles,
    repoRoot,
    targetVersion,
  });
  updateCargoLockVersions({
    backendVersions,
    changes,
    mutatedFiles,
    repoRoot,
  });

  writeSummary({ changes, dryRun: options.dryRun, writeStdout });

  if (!options.dryRun) {
    for (const [filePath, text] of mutatedFiles) {
      fs.writeFileSync(filePath, text, 'utf8');
    }
  }

  return 0;
}

module.exports = {
  bumpSemver,
  parseCliArgs,
  runVersionBump,
};
