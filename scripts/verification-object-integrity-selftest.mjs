#!/usr/bin/env node

import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import {
  chmodSync,
  mkdirSync,
  mkdtempSync,
  readFileSync,
  rmSync,
  statSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import {
  committedTree,
  writeCommittedTree,
} from "./verification-declarations.mjs";
import { fixture } from "./verification-relock-selftest.mjs";
import { prepareCandidate } from "./verification-relock.mjs";
import { assertRepository } from "./verification-stack.mjs";

// Fixture commands deliberately exercise ordinary replacement-ref behavior;
// production readers must suppress it themselves rather than inherit this env.
const env = { ...process.env };
delete env.GIT_NO_REPLACE_OBJECTS;
function git(root, args, input) {
  return execFileSync("git", ["-C", root, ...args], {
    input,
    env,
    encoding: "utf8",
    timeout: 30_000,
    stdio: [input === undefined ? "ignore" : "pipe", "pipe", "pipe"],
  }).trim();
}
const failures = [];
let passed = 0;
function check(name, action) {
  const scratch = mkdtempSync(join(tmpdir(), "quoin-object-integrity-"));
  try {
    action(scratch);
    passed += 1;
    console.log(`ok ${name}`);
  } catch (error) {
    failures.push(`${name}: ${error.message}`);
    console.error(`not ok ${name}: ${error.message}`);
  } finally {
    rmSync(scratch, { recursive: true, force: true });
  }
}
function repository(scratch) {
  const root = join(scratch, "source");
  mkdirSync(root);
  git(root, ["init", "-q"]);
  git(root, ["config", "user.name", "snapshot fixture"]);
  git(root, ["config", "user.email", "snapshot@example.invalid"]);
  git(root, [
    "remote",
    "add",
    "origin",
    "https://github.com/agent-ix/snapshot-fixture",
  ]);
  writeFileSync(join(root, "manifest.yaml"), "name: original\n");
  git(root, ["add", "--all"]);
  git(root, ["commit", "-qm", "original"]);
  const revision = git(root, ["rev-parse", "HEAD"]);
  git(root, ["update-ref", "refs/remotes/origin/main", revision]);
  return {
    root,
    revision,
    remote: "https://github.com/agent-ix/snapshot-fixture",
  };
}
function replaceCommit(root, revision) {
  const blob = git(
    root,
    ["hash-object", "-w", "--stdin"],
    "name: replacement\n",
  );
  const tree = git(root, ["mktree"], `100644 blob ${blob}\tmanifest.yaml\n`);
  const replacement = git(root, ["commit-tree", tree], "replacement\n");
  git(root, ["replace", revision, replacement]);
  return replacement;
}

// TC-1595 / FR-043-AC-35
check("declared file modes survive restrictive caller umask", (scratch) => {
  const { root } = repository(scratch);
  writeFileSync(join(root, "runner.sh"), "#!/bin/sh\nexit 0\n");
  chmodSync(join(root, "runner.sh"), 0o755);
  git(root, ["add", "--all"]);
  git(root, ["commit", "-qm", "executable"]);
  const snapshot = committedTree(root, git(root, ["rev-parse", "HEAD"]));
  const destination = join(scratch, "snapshot");
  const mask = process.umask(0o077);
  try {
    writeCommittedTree(snapshot, destination);
  } finally {
    process.umask(mask);
  }
  for (const file of snapshot.files)
    assert.equal(
      statSync(join(destination, file.path)).mode & 0o777,
      file.mode === "100755" ? 0o755 : 0o644,
      file.path,
    );
});
check(
  "requested commit bytes ignore replacement refs without deleting them",
  (scratch) => {
    const { root, revision } = repository(scratch);
    const original = committedTree(root, revision);
    assert.equal(original.files[0].bytes.toString(), "name: original\n");
    const replacement = replaceCommit(root, revision);
    assert.equal(
      git(root, ["show", `${revision}:manifest.yaml`]),
      "name: replacement",
    );
    assert.deepEqual(committedTree(root, revision), original);
    assert.equal(
      git(root, ["rev-parse", `refs/replace/${revision}`]),
      replacement,
    );
  },
);
check(
  "source preflight binds original commit despite replacement refs",
  (scratch) => {
    const { root, revision, remote } = repository(scratch);
    assertRepository("fixture", root, { revision, remote });
    const replacement = replaceCommit(root, revision);
    assertRepository("fixture", root, { revision, remote });
    assert.equal(
      git(root, ["rev-parse", `refs/replace/${revision}`]),
      replacement,
    );
  },
);
check("candidate schema reads ignore blob replacements", (scratch) => {
  const { roots, base } = fixture(scratch);
  const expected = prepareCandidate(base, roots);
  const root = roots.quire;
  const blob = git(root, [
    "rev-parse",
    "HEAD:schemas/output/coverage-v1.schema.json",
  ]);
  const replacement = git(
    root,
    ["hash-object", "-w", "--stdin"],
    '{"replacement":true}\n',
  );
  git(root, ["replace", blob, replacement]);
  assert.deepEqual(prepareCandidate(base, roots), expected);
  assert.equal(git(root, ["rev-parse", `refs/replace/${blob}`]), replacement);
});
check(
  "valid Unicode survives but invalid UTF-8 Git names are refused",
  (scratch) => {
    const { root, remote } = repository(scratch);
    writeFileSync(join(root, "schéma-測定.yaml"), "healthy: true\n");
    git(root, ["add", "--all"]);
    git(root, ["commit", "-qm", "valid Unicode"]);
    const healthy = committedTree(root, git(root, ["rev-parse", "HEAD"]));
    assert.ok(healthy.files.some((file) => file.path === "schéma-測定.yaml"));
    const unicode = join(scratch, "unicode-snapshot");
    writeCommittedTree(healthy, unicode);
    assert.equal(
      readFileSync(join(unicode, "schéma-測定.yaml"), "utf8"),
      "healthy: true\n",
    );
    const invalid = Buffer.concat([
      Buffer.from(root + "/"),
      Buffer.from([0xff]),
      Buffer.from(".yaml"),
    ]);
    writeFileSync(invalid, "invalid: path\n");
    git(root, ["add", "--all"]);
    git(root, ["commit", "-qm", "non-UTF8 path"]);
    const revision = git(root, ["rev-parse", "HEAD"]);
    git(root, ["update-ref", "refs/remotes/origin/main", revision]);
    assert.throws(() => committedTree(root, revision), /UTF-8/);
    assert.throws(
      () => assertRepository("fixture", root, { revision, remote }),
      /UTF-8/,
    );
  },
);
assert.deepEqual(failures, [], "literal Git object integrity failures");
console.log(`${passed} literal Git object integrity controls passed`);
