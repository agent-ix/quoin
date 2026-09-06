#!/usr/bin/env node

import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import {
  chmodSync,
  mkdirSync,
  mkdtempSync,
  readFileSync,
  rmSync,
  symlinkSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { fixture } from "./verification-relock-selftest.mjs";
import { prepareCandidate } from "./verification-relock.mjs";
import { sha256, validateLockShape } from "./verification-stack.mjs";

const git = (root, ...args) =>
  execFileSync("git", ["-C", root, ...args], {
    encoding: "utf8",
    stdio: ["ignore", "pipe", "pipe"],
  }).trim();
const routes = [
  ["spec-artifacts-process", "spec_artifacts_process"],
  ["spec-artifacts-iso", "spec_artifacts_iso"],
  ["engineering-assurance", "engineering_assurance"],
];
let passed = 0;
function check(name, action) {
  const scratch = mkdtempSync(join(tmpdir(), "quoin-declarations-selftest-"));
  try {
    const { roots, base } = fixture(scratch);
    base.schemaVersion = "quoin-verification-stack-lock-v2";
    const ea = join(scratch, "engineering-assurance");
    mkdirSync(ea);
    roots["engineering-assurance"] = ea;
    git(ea, "init", "-q");
    git(ea, "config", "user.email", "declarations@example.invalid");
    git(ea, "config", "user.name", "declaration selftest");
    git(
      ea,
      "remote",
      "add",
      "origin",
      "https://github.com/agent-ix/engineering-assurance",
    );
    base.declarations = {
      quoinValidation: routes.map(([repository, path]) => {
        const root = roots[repository];
        mkdirSync(join(root, path, "schemas"), { recursive: true });
        const files = [
          ["manifest.yaml", `name: ${repository}\n`],
          ["schemas/frontmatter.json", '{"type":"object"}\n'],
        ].map(([file, bytes]) => {
          writeFileSync(join(root, path, file), bytes);
          return { path: file, mode: "100644", digest: sha256(bytes) };
        });
        git(root, "add", "--all");
        git(root, "commit", "-qm", "module fixture");
        const revision = git(root, "rev-parse", "HEAD");
        git(root, "update-ref", "refs/remotes/origin/main", revision);
        base.repositories[repository] = {
          remote: `https://github.com/agent-ix/${repository}`,
          revision,
        };
        return {
          repository,
          path,
          tree: git(root, "rev-parse", `HEAD:${path}`),
          files,
        };
      }),
    };
    action({ roots, base, scratch });
    passed += 1;
    console.log(`ok ${name}`);
  } finally {
    rmSync(scratch, { recursive: true, force: true });
  }
}

// TC-1593 / FR-043-AC-34: the pre-implementation v1 reader refuses this healthy v2 input.
check("healthy explicit eight-source v2 policy is supported", ({ base }) =>
  assert.equal(validateLockShape(base), base),
);

const { describeModule, materializeDeclarations, runExactValidation } =
  await import("./verification-declarations.mjs");

// TC-1594 / FR-043-AC-34
check(
  "partial and malformed declaration policy cannot acquire a pass",
  ({ base }) => {
    for (const mutate of [
      (lock) => delete lock.repositories["engineering-assurance"],
      (lock) => {
        lock.repositories.unknown = lock.repositories.quire;
      },
      (lock) => {
        lock.declarations.quoinValidation.pop();
      },
      (lock) => {
        lock.declarations.quoinValidation[2] =
          lock.declarations.quoinValidation[0];
      },
      (lock) => {
        lock.declarations.quoinValidation[0].path = "../outside";
      },
      (lock) => {
        lock.declarations.quoinValidation[0].tree = "main";
      },
      (lock) => {
        lock.declarations.quoinValidation[0].files = [];
      },
      (lock) => {
        lock.declarations.quoinValidation[0].files.reverse();
      },
      (lock) => {
        lock.declarations.quoinValidation[0].files[0].digest = "partial";
      },
      (lock) => {
        lock.declarations.quoinValidation[0].files[0].mode = "120000";
      },
    ]) {
      const changed = structuredClone(base);
      mutate(changed);
      assert.throws(() => validateLockShape(changed));
    }
  },
);

// TC-1595 / FR-043-AC-35
check(
  "committed module trees exclude ignored files and reject partial or altered inventories",
  ({ base, roots, scratch }) => {
    const declaration = base.declarations.quoinValidation[0];
    const source = roots[declaration.repository];
    writeFileSync(join(source, ".git", "info", "exclude"), "ignored.yaml\n");
    writeFileSync(
      join(source, declaration.path, "ignored.yaml"),
      "poison: true\n",
    );
    assert.deepEqual(
      describeModule(
        source,
        base.repositories[declaration.repository].revision,
        declaration.path,
      ),
      { tree: declaration.tree, files: declaration.files },
    );
    const outputs = materializeDeclarations(
      base,
      roots,
      join(scratch, "modules"),
    );
    assert.equal(outputs.length, 3);
    assert.throws(
      () => readFileSync(join(outputs[0], "ignored.yaml")),
      /ENOENT/,
    );
    assert.equal(
      readFileSync(join(outputs[0], "schemas/frontmatter.json"), "utf8"),
      '{"type":"object"}\n',
    );
    for (const mutate of [
      (entry) => {
        entry.files[1].digest = `sha256:${"b".repeat(64)}`;
      },
      (entry) => {
        entry.files.pop();
      },
      (entry) => {
        entry.tree = "b".repeat(40);
      },
    ]) {
      const changed = structuredClone(base);
      mutate(changed.declarations.quoinValidation[0]);
      assert.throws(
        () => materializeDeclarations(changed, roots, join(scratch, "invalid")),
        /inventory|tree/,
      );
    }
    symlinkSync("/tmp", join(source, declaration.path, "escape"));
    git(source, "add", "--all");
    git(source, "commit", "-qm", "unsafe module");
    assert.throws(
      () =>
        describeModule(
          source,
          git(source, "rev-parse", "HEAD"),
          declaration.path,
        ),
      /non-regular/,
    );
  },
);

// TC-1596 / FR-043-AC-35: real child process checks the actual argv/env, not a mocked verdict.
check(
  "native exact validation ignores poisoned discovery and propagates failure",
  ({ base, roots, scratch }) => {
    const modules = materializeDeclarations(
      base,
      roots,
      join(scratch, "modules"),
    );
    const producer = join(scratch, "quire-fixture");
    writeFileSync(
      producer,
      `#!${process.execPath}\nconst fs = require('node:fs');\nfs.writeFileSync(process.env.OBSERVED, JSON.stringify({ args: process.argv.slice(2), ambient: [process.env.QUOIN_MODULE_PATHS, process.env.IX_FILAMENT_MODULES_PATH] }));\nprocess.exit(Number(process.env.FAILURE || 0));\n`,
    );
    chmodSync(producer, 0o755);
    const observed = join(scratch, "observed.json");
    const env = {
      ...process.env,
      OBSERVED: observed,
      IX_HOME: join(scratch, "poison-home"),
      QUOIN_MODULE_PATHS: "/poison",
      IX_FILAMENT_MODULES_PATH: "/poison",
    };
    runExactValidation(producer, modules, { cwd: scratch, env });
    const result = JSON.parse(readFileSync(observed, "utf8"));
    assert.deepEqual(result.ambient, [null, null]);
    assert.deepEqual(result.args, [
      "validate",
      "spec/**/*.md",
      "plan/**/*.md",
      "reviews/*.md",
      ...modules.flatMap((path) => ["--module", path]),
    ]);
    assert.throws(
      () => runExactValidation(producer, [], { cwd: scratch, env }),
      /non-empty/,
    );
    assert.throws(
      () =>
        runExactValidation(producer, modules, {
          cwd: scratch,
          env: { ...env, FAILURE: "7" },
        }),
      /failed/,
    );
  },
);

console.log(`${passed} declaration integration checks passed`);
