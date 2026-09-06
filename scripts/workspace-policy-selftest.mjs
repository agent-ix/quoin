#!/usr/bin/env node

import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import {
  copyFileSync,
  mkdirSync,
  mkdtempSync,
  readFileSync,
  rmSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { parse, stringify } from "yaml";

const ROOT = dirname(dirname(fileURLToPath(import.meta.url)));
const rawPackage =
  "templates/semantic-module/{{cookiecutter.repo_name}}/package.json";
const workspace = readFileSync(join(ROOT, "pnpm-workspace.yaml"), "utf8");
const manager = JSON.parse(
  readFileSync(join(ROOT, "package.json"), "utf8"),
).packageManager;
const lock =
  "lockfileVersion: '9.0'\n\nsettings:\n  autoInstallPeers: true\n  excludeLinksFromLockfile: false\n\nimporters:\n\n  .: {}\n";
const scratch = mkdtempSync(join(tmpdir(), "quoin-workspace-policy-"));
// pnpm suppresses repeated verification in its script children. This fixture
// models fresh top-level invocations, so restore the default dependency check.
const env = { ...process.env };
delete env.pnpm_config_verify_deps_before_run;
const run = (args) =>
  execFileSync("corepack", ["pnpm", ...args], {
    cwd: scratch,
    env,
    encoding: "utf8",
    timeout: 30_000,
    stdio: ["ignore", "pipe", "pipe"],
  });
try {
  mkdirSync(dirname(join(scratch, rawPackage)), { recursive: true });
  copyFileSync(join(ROOT, rawPackage), join(scratch, rawPackage));
  assert.throws(
    () => JSON.parse(readFileSync(join(scratch, rawPackage), "utf8")),
    SyntaxError,
  );
  writeFileSync(
    join(scratch, "package.json"),
    JSON.stringify({
      name: "workspace-policy-fixture",
      version: "0.0.0",
      private: true,
      packageManager: manager,
      scripts: { probe: "node probe.cjs" },
    }),
  );
  writeFileSync(
    join(scratch, "probe.cjs"),
    'process.stdout.write("workspace-ok\\n");\n',
  );
  writeFileSync(join(scratch, "pnpm-lock.yaml"), lock);
  writeFileSync(join(scratch, "pnpm-workspace.yaml"), workspace);
  assert.equal(run(["--version"]).trim(), manager.slice("pnpm@".length));
  run(["install", "--frozen-lockfile", "--offline", "--ignore-scripts"]);
  assert.match(
    run(["--config.verify-deps-before-run=error", "exec", "node", "probe.cjs"]),
    /workspace-ok/,
  );
  // TC-1597 / FR-083-AC-9: same normal paths as build/test; no dependency bypass.
  assert.match(run(["run", "probe"]), /workspace-ok/);
  assert.match(run(["exec", "node", "probe.cjs"]), /workspace-ok/);
  assert.equal(readFileSync(join(scratch, "pnpm-lock.yaml"), "utf8"), lock);
  const missingPolicy = parse(workspace);
  delete missingPolicy.packages;
  writeFileSync(join(scratch, "pnpm-workspace.yaml"), stringify(missingPolicy));
  // Default install recovery can continue when PATH finds the pinned manager;
  // its failure is not universal. Error-only checking diagnoses the raw input.
  // Error-only mode tightens the refusal for an attributed diagnostic; it does not skip verification.
  assert.throws(
    () =>
      run([
        "--config.verify-deps-before-run=error",
        "exec",
        "node",
        "probe.cjs",
      ]),
    (error) =>
      error.status === 1 &&
      /cookiecutter\.repo_name/.test(String(error.stdout)),
  );
  assert.equal(readFileSync(join(scratch, "pnpm-lock.yaml"), "utf8"), lock);
  console.log(
    "workspace-policy: exact manager, frozen lock, normal run/exec and missing-policy refusal passed",
  );
} finally {
  rmSync(scratch, { recursive: true, force: true });
}
