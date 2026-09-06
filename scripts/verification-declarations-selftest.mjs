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
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
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
const ROOT = dirname(dirname(fileURLToPath(import.meta.url)));
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

check(
  "v2 candidate derives complete source module artifacts and preserves historical producers",
  ({ base, roots }) => {
    const original = structuredClone(base);
    const candidate = prepareCandidate(base, roots);
    assert.deepEqual(base, original);
    assert.equal(Object.keys(candidate.repositories).length, 8);
    assert.deepEqual(candidate.declarations, original.declarations);
    assert.deepEqual(
      candidate.cohorts.qaExternalQuoin,
      original.cohorts.qaExternalQuoin,
    );
    assert.deepEqual(
      candidate.cohorts.quireBenchmarkQuoin,
      original.cohorts.quireBenchmarkQuoin,
    );
    assert.ok(candidate.artifacts["scripts/verification-declarations.mjs"]);
    assert.ok(
      candidate.artifacts["scripts/verification-declarations-selftest.mjs"],
    );
    const historical = structuredClone(base);
    historical.schemaVersion = "quoin-verification-stack-lock-v1";
    delete historical.repositories["engineering-assurance"];
    assert.throws(() => validateLockShape(historical), /historical v1/);
    delete historical.declarations;
    assert.equal(validateLockShape(historical), historical);
    const missing = { ...roots };
    delete missing["engineering-assurance"];
    assert.throws(
      () => prepareCandidate(base, missing),
      /explicit absolute root/,
    );
  },
);

check(
  "native v2 relock command requires eight roots and creates only a new candidate",
  ({ base, roots, scratch }) => {
    const policy = join(scratch, "policy.json");
    const output = join(scratch, "candidate.json");
    writeFileSync(policy, JSON.stringify(base));
    const args = [
      join(ROOT, "scripts/verification-relock.mjs"),
      "--lock",
      policy,
      "--out",
      output,
      ...Object.entries(roots).flatMap(([name, root]) => [
        "--root",
        `${name}=${root}`,
      ]),
    ];
    execFileSync(process.execPath, args, { stdio: ["ignore", "pipe", "pipe"] });
    const candidate = JSON.parse(readFileSync(output, "utf8"));
    assert.equal(candidate.schemaVersion, "quoin-verification-stack-lock-v2");
    assert.deepEqual(candidate.declarations, base.declarations);
    assert.deepEqual(JSON.parse(readFileSync(policy, "utf8")), base);
    assert.throws(
      () =>
        execFileSync(process.execPath, args, {
          stdio: ["ignore", "pipe", "pipe"],
        }),
      /EEXIST/,
    );
  },
);

check(
  "hidden nested schema edits cannot become candidate source",
  ({ base, roots }) => {
    const root = roots["engineering-assurance"];
    git(
      root,
      "update-index",
      "--skip-worktree",
      "engineering_assurance/schemas/frontmatter.json",
    );
    writeFileSync(
      join(root, "engineering_assurance/schemas/frontmatter.json"),
      '{"changed":true}\n',
    );
    assert.equal(git(root, "status", "--porcelain"), "");
    assert.throws(
      () => prepareCandidate(base, roots),
      /differs from git object/,
    );
  },
);

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
      assert.throws(() => prepareCandidate(changed, roots), /inventory|tree/);
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
    mkdirSync(join(scratch, "bin"));
    const producer = join(scratch, "bin", "quire");
    writeFileSync(
      producer,
      `#!${process.execPath}\nconst fs = require('node:fs');\nfs.writeFileSync(process.env.OBSERVED, JSON.stringify({ args: process.argv.slice(2), home: process.env.IX_HOME, ambient: [process.env.QUOIN_MODULE_PATHS, process.env.IX_FILAMENT_MODULES_PATH] }));\nprocess.exit(Number(process.env.FAILURE || 0));\n`,
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
    mkdirSync(join(env.IX_HOME, "filament/modules/poison"), {
      recursive: true,
    });
    writeFileSync(
      join(env.IX_HOME, "filament/modules/poison/manifest.yaml"),
      "invalid: poison\n",
    );
    runExactValidation(producer, modules, { cwd: scratch, env });
    const result = JSON.parse(readFileSync(observed, "utf8"));
    assert.deepEqual(result.ambient, [null, null]);
    assert.notEqual(result.home, env.IX_HOME);
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
    const manifest = join(scratch, "exact roots.json");
    writeFileSync(manifest, JSON.stringify(modules));
    // The native Make routing control omits only the fixture's unrelated build;
    // canonical execution still uses the unchanged build prerequisite.
    const args = [
      "--no-print-directory",
      "-o",
      "build",
      "validate",
      `QUIRE=${producer}`,
      `QUOIN_VERIFICATION_DECLARATIONS=${manifest}`,
    ];
    execFileSync("make", args, {
      cwd: ROOT,
      env,
      stdio: ["ignore", "pipe", "pipe"],
    });
    assert.deepEqual(
      JSON.parse(readFileSync(observed, "utf8")).args,
      result.args,
    );
    assert.throws(
      () =>
        execFileSync("make", args, {
          cwd: ROOT,
          env: { ...env, FAILURE: "7" },
          stdio: ["ignore", "pipe", "pipe"],
        }),
      /exact native validation failed/,
    );
    const legacy = execFileSync(
      "make",
      [
        "-n",
        "validate",
        `QUIRE=${producer}`,
        "QUOIN_VERIFICATION_DECLARATIONS=",
      ],
      { cwd: ROOT, encoding: "utf8" },
    );
    assert.match(legacy, /module ensure-defaults/);
    assert.doesNotMatch(legacy, /node scripts\/verification-declarations.mjs/);
  },
);

console.log(`${passed} declaration integration checks passed`);
