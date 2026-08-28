#!/usr/bin/env node

import { spawnSync } from "node:child_process";
import { mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

import {
  assertRepository,
  cliSelectsEngine,
  parseSubmoduleRevision,
  qaCorpusCounts,
  validateLockShape,
} from "./verification-stack.mjs";

const A = "a".repeat(40);
const B = "b".repeat(40);
const digest = `sha256:${"c".repeat(64)}`;
const base = {
  schemaVersion: "quoin-verification-stack-lock-v1",
  repositories: Object.fromEntries(
    [
      "quoin",
      "quire",
      "quire-cli",
      "qa-corpus",
      "filament-ide-rs",
      "spec-artifacts-process",
      "spec-artifacts-iso",
    ].map((name) => [
      name,
      { remote: `https://github.com/agent-ix/${name}`, revision: A },
    ]),
  ),
  cohorts: {
    qaExternalQuoin: { revision: B },
    qaCorpus: { executableCases: 2, reportingCases: 1, totalCases: 3 },
  },
  requiredCapabilities: ["metrics_envelope"],
  artifacts: { "bench/metrics.json": digest },
  timeouts: {
    caseMilliseconds: 1000,
    corpusMilliseconds: 1000,
    tier1Milliseconds: 2000,
    tier2Milliseconds: 1000,
    installMilliseconds: 1000,
    quoinMilliseconds: 1000,
    spanMilliseconds: 1000,
  },
};

const shapeMutations = [
  [
    "branch",
    (v) => (v.repositories.quire.revision = "main"),
    /full lowercase commit/,
  ],
  [
    "tag",
    (v) => (v.repositories.quire.revision = "v0.46.0"),
    /full lowercase commit/,
  ],
  [
    "abbreviated SHA",
    (v) => (v.repositories.quire.revision = A.slice(0, 12)),
    /full lowercase commit/,
  ],
  [
    "ambiguous uppercase SHA",
    (v) => (v.repositories.quire.revision = A.toUpperCase()),
    /full lowercase commit/,
  ],
  [
    "moving remote ref",
    (v) => (v.repositories.quire.remote += "@main"),
    /moving ref/,
  ],
  [
    "short artifact digest",
    (v) => (v.artifacts["bench/metrics.json"] = "sha256:abc"),
    /full sha256/,
  ],
  ["missing capabilities", (v) => (v.requiredCapabilities = []), /capability/],
  [
    "missing declaration repository",
    (v) => delete v.repositories["spec-artifacts-iso"],
    /spec-artifacts-iso must be locked/,
  ],
  [
    "qa case-count partition",
    (v) => (v.cohorts.qaCorpus.totalCases = 4),
    /partition totalCases/,
  ],
  [
    "missing timeout",
    (v) => delete v.timeouts.corpusMilliseconds,
    /timeouts.corpusMilliseconds/,
  ],
  [
    "undersized Tier-1 campaign timeout",
    (v) => (v.timeouts.tier1Milliseconds = 1000),
    /cover the locked per-case budget/,
  ],
];

let passed = 0;
for (const [name, mutate, expected] of shapeMutations) {
  const value = structuredClone(base);
  mutate(value);
  try {
    validateLockShape(value);
    throw new Error(`${name} mutation was accepted`);
  } catch (error) {
    if (!expected.test(String(error.message))) throw error;
    passed += 1;
  }
}

const counts = qaCorpusCounts({
  cases: [{ mode: "detection" }, { mode: "attachment" }, { mode: "reporting" }],
});
if (
  counts.executableCases !== 2 ||
  counts.reportingCases !== 1 ||
  counts.totalCases !== 3
) {
  throw new Error("qa-corpus case counts were not partitioned exactly");
}
passed += 1;

try {
  cliSelectsEngine(
    `[dependencies]\nquire = { git = "https://example/quire", rev = "${A}" }\n`,
    `[[package]]\nname = "quire-rs"\nsource = "git+https://example/quire?rev=${A}#${B}"\n`,
    A,
  );
  throw new Error("Cargo resolution mismatch was accepted");
} catch (error) {
  if (!/does not select locked Quire/.test(String(error.message))) throw error;
  passed += 1;
}
cliSelectsEngine(
  `[dependencies]\nquire-rs = { git = "https://example/quire", rev = "${A}" }\n`,
  `[[package]]\nname = "quire-rs"\nsource = "git+https://example/quire?rev=${A}#${A}"\n`,
  A,
);
passed += 1;

if (parseSubmoduleRevision(` ${A} corpus (heads/main)`) !== A) {
  throw new Error("exact submodule revision was not preserved");
}
passed += 1;
for (const marker of ["+", "-"]) {
  try {
    parseSubmoduleRevision(`${marker}${A} corpus`);
    throw new Error(`${marker} submodule mismatch was accepted`);
  } catch (error) {
    if (!/uninitialized or mismatched/.test(String(error.message))) throw error;
    passed += 1;
  }
}

const root = mkdtempSync(join(tmpdir(), "quoin-stack-selftest-"));
const git = (...args) => {
  const done = spawnSync("git", ["-C", root, ...args], { encoding: "utf8" });
  if (done.status !== 0) throw new Error(done.stderr);
  return done.stdout.trim();
};
try {
  git("init");
  git("config", "user.email", "stack-selftest@example.invalid");
  git("config", "user.name", "stack selftest");
  writeFileSync(join(root, "code.txt"), "one\n");
  git("add", "code.txt");
  git("commit", "-m", "one");
  const first = git("rev-parse", "HEAD");
  git("remote", "add", "origin", "https://github.com/agent-ix/selftest");
  git("update-ref", "refs/remotes/origin/main", first);
  const locked = {
    remote: "https://github.com/agent-ix/selftest",
    revision: first,
  };
  assertRepository("fixture", root, locked);

  writeFileSync(join(root, "evidence.json"), "one\n");
  git("add", "evidence.json");
  git("commit", "-m", "evidence one");
  writeFileSync(join(root, "evidence.json"), "two\n");
  git("add", "evidence.json");
  git("commit", "-m", "evidence two");
  assertRepository("fixture", root, locked, {
    allowEvidenceOverlay: true,
    allowedOverlayPaths: ["evidence.json"],
  });
  passed += 1;

  writeFileSync(join(root, "dirty.txt"), "dirty\n");
  try {
    assertRepository("fixture", root, locked);
    throw new Error("dirty checkout was accepted");
  } catch (error) {
    if (!/dirty/.test(String(error.message))) throw error;
    passed += 1;
  }
  rmSync(join(root, "dirty.txt"));

  writeFileSync(join(root, "code.txt"), "two\n");
  git("add", "code.txt");
  git("commit", "-m", "two");
  const second = git("rev-parse", "HEAD");
  try {
    assertRepository("fixture", root, { ...locked, revision: second });
    throw new Error("remote-unreachable commit was accepted");
  } catch (error) {
    if (!/not reachable/.test(String(error.message))) throw error;
    passed += 1;
  }
} finally {
  rmSync(root, { recursive: true, force: true });
}

console.log(
  `verification-stack-selftest: ${passed}/${shapeMutations.length + 9} invariants verified`,
);
