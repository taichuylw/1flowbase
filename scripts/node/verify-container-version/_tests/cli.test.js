const test = require("node:test");
const assert = require("node:assert/strict");
const fs = require("node:fs");
const path = require("node:path");
const { spawnSync } = require("node:child_process");

const repoRoot = path.resolve(__dirname, "..", "..", "..", "..");
const scriptPath = path.join(
  repoRoot,
  "scripts",
  "node",
  "container-image-security",
  "verify-version.js",
);

test("print-tag reads the web component manifest from the repository root", () => {
  const version = JSON.parse(
    fs.readFileSync(path.join(repoRoot, "web", "app", "package.json"), "utf8"),
  ).version;

  const result = spawnSync(process.execPath, [scriptPath, "print-tag", "web"], {
    cwd: repoRoot,
    encoding: "utf8",
  });

  assert.equal(result.status, 0, result.stderr);
  assert.equal(result.stdout.trim(), `v${version}`);
});

test("component validation accepts the current api-server image tag", () => {
  const version = fs
    .readFileSync(
      path.join(repoRoot, "api", "apps", "api-server", "Cargo.toml"),
      "utf8",
    )
    .match(/^version\s*=\s*"([^"]+)"/m)[1];

  const result = spawnSync(
    process.execPath,
    [scriptPath, "api-server", `v${version}`],
    {
      cwd: repoRoot,
      encoding: "utf8",
    },
  );

  assert.equal(result.status, 0, result.stderr);
});
