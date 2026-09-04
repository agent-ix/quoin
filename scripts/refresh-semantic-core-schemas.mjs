#!/usr/bin/env node
/**
 * Refresh the vendored semantic-core JSON Schema bundle and the
 * filament-core-data package-manifest/common schemas from a filament-core-data
 * checkout (quoin FR-073, FR-075).
 *
 * Usage:
 *   node scripts/refresh-semantic-core-schemas.mjs [--source <filament-core-data checkout>]
 *
 * Files are read from the git objects named by `SEMANTIC_CONTRACT` in
 * `src/semantic/contract.ts`. The bundle digest is recomputed exactly as
 * filament-core-data's `packages/semantic-core/scripts/generate.mjs` does and
 * must equal the recorded `bundleDigest`; the two schemas must hash to their
 * recorded `sha256`. Bump the contract first; this script refuses drift.
 */

import { execFileSync } from "node:child_process";
import { createHash } from "node:crypto";
import { mkdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const repo = resolve(here, "..");
const contractFile = join(repo, "src", "semantic", "contract.ts");
const bundleDir = join(repo, "src", "semantic", "schemas", "semantic-core");
const coreDataDir = join(
  repo,
  "src",
  "semantic",
  "schemas",
  "filament-core-data",
);

function arg(name, fallback) {
  const idx = process.argv.indexOf(name);
  return idx >= 0 ? process.argv[idx + 1] : fallback;
}

function field(block, name, pattern) {
  const value = new RegExp(`${name}:\\s*"(${pattern})"`).exec(block)?.[1];
  if (!value) {
    console.error(`could not read ${name} from src/semantic/contract.ts`);
    process.exit(1);
  }
  return value;
}

const source = resolve(arg("--source", join(repo, "..", "filament-core-data")));
const contract = readFileSync(contractFile, "utf8");
const core = /semanticCore:\s*\{([\s\S]*?)\}/.exec(contract)?.[1] ?? "";
const revision = field(core, "sourceRevision", "[0-9a-f]{40}");
const path = field(core, "sourcePath", '[^"]+');
const expectedBundle = field(core, "bundleDigest", "sha256:[0-9a-f]{64}");

function show(objectPath) {
  return execFileSync("git", [
    "-C",
    source,
    "show",
    `${revision}:${objectPath}`,
  ]);
}

const names = execFileSync(
  "git",
  ["-C", source, "ls-tree", "--name-only", revision, `${path}/`],
  {
    encoding: "utf8",
  },
)
  .split("\n")
  .filter((line) => line.endsWith(".json"))
  .map((line) => line.split("/").pop())
  .sort();
const files = new Map(names.map((name) => [name, show(`${path}/${name}`)]));
const digest = createHash("sha256");
for (const [name, bytes] of files)
  digest.update(`${name}\n${bytes.toString("utf8")}`);
const actualBundle = `sha256:${digest.digest("hex")}`;
if (actualBundle !== expectedBundle) {
  console.error(
    `refusing to vendor: bundle digest ${actualBundle}, contract records ${expectedBundle}`,
  );
  process.exit(1);
}
const toolchain = show("packages/semantic-core/generated/toolchain.json");
if (!toolchain.toString("utf8").includes(expectedBundle)) {
  console.error(
    "refusing to vendor: toolchain.json at the pinned revision does not record the bundle digest",
  );
  process.exit(1);
}

const schemas = [];
for (const key of ["packageManifestSchema", "commonSchema"]) {
  const block =
    new RegExp(`${key}:\\s*\\{([\\s\\S]*?)\\}`).exec(contract)?.[1] ?? "";
  const schemaPath = field(block, "sourcePath", '[^"]+');
  const expected = field(block, "sha256", "sha256:[0-9a-f]{64}");
  const bytes = show(schemaPath);
  const actual = `sha256:${createHash("sha256").update(bytes).digest("hex")}`;
  if (actual !== expected) {
    console.error(
      `refusing to vendor: ${schemaPath} hashes to ${actual}, contract records ${expected}`,
    );
    process.exit(1);
  }
  schemas.push([schemaPath.split("/").pop(), bytes]);
}

rmSync(bundleDir, { recursive: true, force: true });
mkdirSync(bundleDir, { recursive: true });
for (const [name, bytes] of files) writeFileSync(join(bundleDir, name), bytes);
writeFileSync(join(bundleDir, "toolchain.json"), toolchain);
mkdirSync(coreDataDir, { recursive: true });
for (const [name, bytes] of schemas)
  writeFileSync(join(coreDataDir, name), bytes);
console.log(
  `vendored semantic-core bundle (${files.size} files) and ${schemas.length} filament-core-data schemas @${revision.slice(0, 12)}`,
);
