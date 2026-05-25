#!/usr/bin/env node

const fs = require('node:fs');
const path = require('node:path');

const COMPONENTS = {
  web: {
    manifest: 'web/app/package.json',
    readVersion(file) {
      return JSON.parse(fs.readFileSync(file, 'utf8')).version;
    },
  },
  'api-server': {
    manifest: 'api/apps/api-server/Cargo.toml',
    readVersion: readCargoPackageVersion,
  },
  'plugin-runner': {
    manifest: 'api/apps/plugin-runner/Cargo.toml',
    readVersion: readCargoPackageVersion,
  },
};

function readCargoPackageVersion(file) {
  const source = fs.readFileSync(file, 'utf8');
  const match = source.match(/^version\s*=\s*"([^"]+)"/m);
  if (!match) {
    throw new Error(`${file} must declare an explicit package version`);
  }
  return match[1];
}

function fail(message) {
  console.error(message);
  process.exit(1);
}

const [, , component, imageTag] = process.argv;

if (!component || !imageTag) {
  fail('Usage: node scripts/node/verify-container-version.js <web|api-server|plugin-runner> <vX.Y.Z>');
}

const config = COMPONENTS[component];
if (!config) {
  fail(`Unknown component: ${component}`);
}

if (!/^v\d+\.\d+\.\d+([-.][0-9A-Za-z.-]+)?$/.test(imageTag)) {
  fail(`Invalid image tag: ${imageTag}. Expected vX.Y.Z, optionally with a Docker-safe suffix.`);
}

const repoRoot = path.resolve(__dirname, '..', '..');
const manifestPath = path.join(repoRoot, config.manifest);
const manifestVersion = config.readVersion(manifestPath);
const expectedTag = `v${manifestVersion}`;

if (imageTag !== expectedTag) {
  fail(`${component} image tag ${imageTag} does not match ${config.manifest} version ${manifestVersion}. Expected ${expectedTag}.`);
}

console.log(`${component} ${imageTag} matches ${config.manifest}`);
