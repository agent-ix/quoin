#!/usr/bin/env node
/**
 * Fresh-root default-module assertion.
 *
 * Runs inside the clean-room container against the PUBLISHED package. Points
 * IX_HOME at an empty directory, runs `quoin module ensure-defaults`, then
 * asserts the materialized registry matches the manifest the package ships.
 *
 * This is the check that would have caught v0.12.2: on a developer machine the
 * module set was already installed (and had been updated by hand), so every
 * local run passed against a state no new user has. A representative check has
 * to start from an empty root.
 *
 * Reuses the package's own exports — `defaultModulesManifest()` and
 * `listPlugins()` — so it asserts against the same code path a real user hits,
 * not a reimplementation of it.
 */

import { execFileSync } from "node:child_process";
import { mkdtempSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

const BIN = process.env.BIN ?? "quoin";

function globalPackageEntry() {
  // Dry-running this stage against a local checkout (rather than the container's
  // global install) is how it gets verified before it ships.
  if (process.env.QUOIN_SMOKE_ENTRY) return process.env.QUOIN_SMOKE_ENTRY;
  const root = execFileSync("npm", ["root", "-g"], { encoding: "utf8" }).trim();
  const pkg = process.env.PKG ?? "@agent-ix/quoin";
  return join(root, pkg, "dist", "index.js");
}

const home = mkdtempSync(join(tmpdir(), "quoin-freshroot-"));
console.log(`\n## Stage 1b — default module set from an EMPTY root`);
console.log(`   IX_HOME=${home}`);

try {
  execFileSync(BIN, ["module", "ensure-defaults"], {
    encoding: "utf8",
    env: { ...process.env, IX_HOME: home },
    stdio: "inherit",
  });
} catch {
  console.log("   FAIL  `module ensure-defaults` did not complete");
  process.exit(1);
}

const { defaultModulesManifest, listPlugins } = await import(
  globalPackageEntry()
);

const expected = new Map(
  (defaultModulesManifest().entries ?? []).map((entry) => [
    entry.name,
    entry.source?.ref ?? "",
  ]),
);
const installed = new Map(
  listPlugins(home).map((plugin) => [plugin.name, plugin.ref ?? ""]),
);

let fail = 0;
for (const [name, ref] of expected) {
  if (!installed.has(name)) {
    console.log(`   FAIL  declared module never materialized: ${name}@${ref}`);
    fail = 1;
  } else if (installed.get(name) !== ref) {
    console.log(
      `   FAIL  ${name} resolved to ${installed.get(name)}, manifest declares ${ref}`,
    );
    fail = 1;
  } else {
    console.log(`   PASS  ${name}@${ref}`);
  }
}
for (const name of installed.keys()) {
  if (!expected.has(name)) {
    console.log(
      `   FAIL  module installed that the manifest never declared: ${name}`,
    );
    fail = 1;
  }
}

console.log(
  fail === 0
    ? `## RESULT: PASS — fresh root resolves exactly the ${expected.size} declared modules`
    : "## RESULT: FAIL — the fresh-root module set does not match the shipped manifest",
);
process.exit(fail);
