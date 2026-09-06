#!/usr/bin/env node

import { spawnSync } from "node:child_process";
import {
  copyFileSync,
  mkdirSync,
  mkdtempSync,
  rmSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

// Keep relock qualification on the existing canonical selftest path.
import "./verification-relock-selftest.mjs";

import {
  assertRepository,
  assertRemoteRevision,
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
    quireBenchmarkQuoin: { revision: A },
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
    "missing Quire benchmark cohort",
    (v) => delete v.cohorts.quireBenchmarkQuoin,
    /quireBenchmarkQuoin must be locked/,
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
    `[dependencies]\nquire = { package = "quire-rs", git = "https://example/quire", rev = "${A}" }\n`,
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

// TC-1590: parsed Cargo fields, never comments or an unrelated dependency.
const selectedLock = `[[package]]\nname = "quire-rs"\nsource = "git+https://example/quire?rev=${A}#${A}"\n`;
for (const manifest of [
  `[dependencies]\nquire-rs = { git = "https://example/quire", rev = "${B}" }\n# reviewed: rev = "${A}"`,
  `[dependencies]\nother = { git = "https://example/quire", rev = "${A}" }`,
  `[dev-dependencies]\nquire-rs = { git = "https://example/quire", rev = "${A}" }`,
  `[dependencies]\nquire-rs = { git = "https://example/quire", rev = "${A}", branch = "main" }`,
  `[dependencies]\nquire-rs = { git = "https://example/quire", rev = "${A}", tag = "v1" }`,
  `[dependencies]\nquire-rs = { git = "https://example/quire", rev = "${A}", path = "../quire" }`,
  `[dependencies]\nquire-rs = { workspace = true }\n[workspace.dependencies]\nquire-rs = { git = "https://example/quire", rev = "${A}" }`,
  `[dependencies]\nquire-rs = { git = "https://example/quire", rev = "${A}" }\n[patch.crates-io]\nquire-rs = { path = "../quire" }`,
  `[dependencies]\nquire-rs = { git = "https://example/quire", rev = "${A}" }\n[target.'cfg(unix)'.dependencies]\nquire-rs = { git = "https://example/quire", rev = "${B}" }`,
]) {
  try {
    cliSelectsEngine(manifest, selectedLock, A);
    throw new Error("ambiguous Cargo dependency accepted");
  } catch (error) {
    if (!/Cargo.toml does not pin/.test(error.message)) throw error;
    passed += 1;
  }
}
cliSelectsEngine(
  `[dependencies.engine]\npackage = 'quire-rs'\ngit = 'https://example/quire'\nrev = '${A}'\n`,
  selectedLock,
  A,
);
passed += 1;
for (const lockfile of [
  `[[package]]\nname = "quire-rs"\nsource = "git+https://example/quire?rev=${A}#${B}"\n# reviewed #${A}`,
  `[[package]]\nname = "quire-rs"\nsource = "git+https://example/quire?branch=main#${A}"`,
  `[[package]]\nname = "quire-rs"\nsource = "git+https://other/quire?rev=${A}#${A}"`,
]) {
  try {
    cliSelectsEngine(
      `[dependencies]\nquire-rs = { git = "https://example/quire", rev = "${A}" }`,
      lockfile,
      A,
    );
    throw new Error("ambiguous Cargo resolution accepted");
  } catch (error) {
    if (!/Cargo.lock does not select/.test(error.message)) throw error;
    passed += 1;
  }
}

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

const bootstrap = mkdtempSync(join(tmpdir(), "quoin-stack-bootstrap-"));
try {
  mkdirSync(join(bootstrap, "scripts"));
  copyFileSync(
    new URL("./verification-stack.mjs", import.meta.url),
    join(bootstrap, "scripts/verification-stack.mjs"),
  );
  writeFileSync(join(bootstrap, "lock.json"), JSON.stringify(base));
  const result = spawnSync(
    process.execPath,
    [
      join(bootstrap, "scripts/verification-stack.mjs"),
      "--lock",
      join(bootstrap, "lock.json"),
    ],
    {
      encoding: "utf8",
      env: { ...process.env, QUIRE_ROOT: join(bootstrap, "absent-engine") },
    },
  );
  if (
    result.status === 0 ||
    !result.stderr.includes("quire checkout is missing") ||
    !result.stderr.includes("make verification-relock") ||
    result.stderr.includes("Cannot find")
  )
    throw new Error(
      "clean bootstrap did not reach source checks before loading development dependencies",
    );
  passed += 1;
} finally {
  rmSync(bootstrap, { recursive: true, force: true });
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
  assertRemoteRevision("fixture cohort", root, first);
  try {
    assertRemoteRevision("fixture cohort", root, B);
    throw new Error("missing cohort commit was accepted");
  } catch (error) {
    if (!/not a local commit/.test(String(error.message))) throw error;
    passed += 1;
  }
  const locked = {
    remote: "https://github.com/agent-ix/selftest",
    revision: first,
  };
  assertRepository("fixture", root, locked);

  writeFileSync(join(root, "evidence.json"), "one\n");
  git("add", "evidence.json");
  git("commit", "-m", "evidence one");
  const evidenceOne = git("rev-parse", "HEAD");
  writeFileSync(join(root, "evidence.json"), "two\n");
  git("add", "evidence.json");
  git("commit", "-m", "evidence two");
  const evidenceTip = git("rev-parse", "HEAD");
  assertRepository("fixture", root, locked, {
    allowEvidenceOverlay: true,
    allowedOverlayPaths: ["evidence.json"],
  });
  passed += 1;

  git("branch", "evidence-tip", evidenceTip);
  git("checkout", "-b", "promotion-base", first);
  git("merge", "--no-ff", "evidence-tip", "-m", "terminal promotion");
  const promotion = git("rev-parse", "HEAD");
  git("update-ref", "refs/remotes/origin/main", promotion);
  try {
    assertRepository("fixture", root, locked, {
      allowEvidenceOverlay: true,
      allowedOverlayPaths: ["evidence.json"],
    });
    throw new Error("terminal promotion was accepted without permission");
  } catch (error) {
    if (!/one permitted terminal promotion merge/.test(String(error.message)))
      throw error;
    passed += 1;
  }
  assertRepository("fixture", root, locked, {
    allowEvidenceOverlay: true,
    allowTerminalPromotionMerge: true,
    allowedOverlayPaths: ["evidence.json"],
  });
  passed += 1;

  const evidenceTree = git("rev-parse", `${evidenceTip}^{tree}`);
  const lockedTree = git("rev-parse", `${first}^{tree}`);
  writeFileSync(join(root, "tampered.txt"), "tampered\n");
  git("add", "tampered.txt");
  const tamperedTree = git("write-tree");
  git("reset", "--hard", promotion);
  const changedPromotion = git(
    "commit-tree",
    tamperedTree,
    "-p",
    first,
    "-p",
    evidenceTip,
    "-m",
    "changed promotion tree",
  );
  git("update-ref", "refs/remotes/origin/main", changedPromotion);
  git("checkout", "--detach", changedPromotion);
  try {
    assertRepository("fixture", root, locked, {
      allowEvidenceOverlay: true,
      allowTerminalPromotionMerge: true,
      allowedOverlayPaths: ["evidence.json"],
    });
    throw new Error("tree-changing terminal promotion was accepted");
  } catch (error) {
    if (!/promotion tree differs/.test(String(error.message))) throw error;
    passed += 1;
  }

  const firstParentEvidence = git(
    "commit-tree",
    lockedTree,
    "-p",
    evidenceTip,
    "-p",
    first,
    "-m",
    "first-parent evidence substitution",
  );
  git("update-ref", "refs/remotes/origin/main", firstParentEvidence);
  git("checkout", "--detach", firstParentEvidence);
  try {
    assertRepository("fixture", root, locked, {
      allowEvidenceOverlay: true,
      allowTerminalPromotionMerge: true,
      allowedOverlayPaths: ["evidence.json"],
    });
    throw new Error("first-parent evidence substitution was accepted");
  } catch (error) {
    if (!/first parent is not an ancestor/.test(String(error.message)))
      throw error;
    passed += 1;
  }

  const octopusPromotion = git(
    "commit-tree",
    evidenceTree,
    "-p",
    first,
    "-p",
    evidenceTip,
    "-p",
    evidenceOne,
    "-m",
    "octopus promotion",
  );
  git("update-ref", "refs/remotes/origin/main", octopusPromotion);
  git("checkout", "--detach", octopusPromotion);
  try {
    assertRepository("fixture", root, locked, {
      allowEvidenceOverlay: true,
      allowTerminalPromotionMerge: true,
      allowedOverlayPaths: ["evidence.json"],
    });
    throw new Error("octopus terminal promotion was accepted");
  } catch (error) {
    if (!/one permitted terminal promotion merge/.test(String(error.message)))
      throw error;
    passed += 1;
  }

  const nestedEvidenceMerge = git(
    "commit-tree",
    evidenceTree,
    "-p",
    evidenceTip,
    "-p",
    evidenceOne,
    "-m",
    "nested evidence merge",
  );
  const nestedPromotion = git(
    "commit-tree",
    evidenceTree,
    "-p",
    first,
    "-p",
    nestedEvidenceMerge,
    "-m",
    "promotion of nested evidence",
  );
  git("update-ref", "refs/remotes/origin/main", nestedPromotion);
  git("checkout", "--detach", nestedPromotion);
  try {
    assertRepository("fixture", root, locked, {
      allowEvidenceOverlay: true,
      allowTerminalPromotionMerge: true,
      allowedOverlayPaths: ["evidence.json"],
    });
    throw new Error("nested evidence merge was accepted");
  } catch (error) {
    if (
      !/contains a merge before terminal promotion/.test(String(error.message))
    )
      throw error;
    passed += 1;
  }

  const unpublishedPromotion = git(
    "commit-tree",
    evidenceTree,
    "-p",
    first,
    "-p",
    evidenceTip,
    "-m",
    "unpublished terminal promotion",
  );
  git("update-ref", "refs/remotes/origin/main", promotion);
  git("checkout", "--detach", unpublishedPromotion);
  try {
    assertRepository("fixture", root, locked, {
      allowEvidenceOverlay: true,
      allowTerminalPromotionMerge: true,
      allowedOverlayPaths: ["evidence.json"],
    });
    throw new Error("unpublished terminal promotion was accepted");
  } catch (error) {
    if (!/not reachable from a remote-tracking ref/.test(String(error.message)))
      throw error;
    passed += 1;
  }

  git("checkout", "evidence-tip");

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
    assertRepository("fixture", root, locked, {
      allowEvidenceOverlay: true,
      allowedOverlayPaths: ["evidence.json"],
    });
    throw new Error("code-changing evidence overlay was accepted");
  } catch (error) {
    if (!/changes code/.test(String(error.message))) throw error;
    passed += 1;
  }
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

console.log(`verification-stack-selftest: ${passed} invariants verified`);
