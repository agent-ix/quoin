#!/usr/bin/env node
/**
 * Instantiate the semantic-module cookiecutter and run what it renders
 * (quoin FR-083, issue #307).
 *
 * `make test` renders every variant and checks it against the conformance
 * contract, the residue classes and the drift pins — everything that can be
 * decided by reading the rendered bytes. This script adds the two legs that
 * cannot: emitting the schemas with the pinned TypeSpec compiler (which needs
 * the registry), and running the rendered repository's own suite (which needs a
 * Python toolchain and the Quire engine).
 *
 * Every tool it needs is a hard requirement. When one is absent the script FAILS
 * naming the command and how to install it — it never reports success for a leg
 * it did not run, because a gate that passes without running is the defect this
 * gate exists to catch (NFR-020).
 */
import { execFileSync, spawnSync } from "node:child_process";
import { existsSync, mkdtempSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const ROOT = dirname(dirname(fileURLToPath(import.meta.url)));
const TEMPLATE = join(ROOT, "templates", "semantic-module");

const VARIANTS = [
  { kind: "object", repo: "spec-objects-example", extra: [] },
  { kind: "artifact", repo: "spec-artifacts-example", extra: [] },
  {
    kind: "mixed",
    repo: "spec-mixed-example",
    extra: ["imported_modules=agent-ix/spec-objects-business@0.3.0"],
  },
];

const TOOLS = [
  {
    command: "cookiecutter",
    args: ["--version"],
    install: "pipx install cookiecutter",
  },
  { command: "npm", args: ["--version"], install: "ships with Node" },
  { command: "node", args: ["--version"], install: "https://nodejs.org" },
  {
    command: "python3",
    args: ["--version"],
    install: "your platform's Python 3",
  },
];

/**
 * Refuse, by THROWING.
 *
 * `process.exit()` does not unwind the stack, so an exit here would skip the
 * `finally` that removes the rendered directory and would make the per-variant
 * tally below unreachable: the first failing step of the first variant would
 * end the process, leaking its tree and reporting at most one failure however
 * many there were.
 */
class GateError extends Error {}

function fail(message) {
  throw new GateError(message);
}

function requireTools() {
  for (const tool of TOOLS) {
    try {
      execFileSync(tool.command, tool.args, { stdio: "pipe" });
    } catch (error) {
      fail(
        `\`${tool.command}\` is required and is not usable here. Install it with: ` +
          `${tool.install}. This leg fails rather than skipping — a template gate ` +
          `that reports green without instantiating anything verified nothing. ` +
          `(${error.message})`,
      );
    }
  }

  // The rendered suite imports these. Without them pytest reports collection
  // errors that read as template defects, and without `quire` the semantic rows
  // fail on the engine rather than on the module. Name them here, once, with the
  // command that installs them — the same discipline the rendered `conftest.py`
  // applies to the engine.
  const modules = [
    { name: "pytest", install: "python3 -m pip install pytest" },
    { name: "ruff", install: "python3 -m pip install ruff" },
    { name: "black", install: "python3 -m pip install black" },
    { name: "yaml", install: "python3 -m pip install pyyaml" },
    {
      name: "jsonschema",
      install: "python3 -m pip install 'jsonschema>=4.26'",
    },
    { name: "referencing", install: "python3 -m pip install referencing" },
    {
      name: "quire",
      install:
        "make dev-quire in a rendered repository, or " +
        "pip install --index-url http://pypi.ix/root/dev/+simple/ --trusted-host pypi.ix 'quire>=0.46.0' " +
        "(agent-ix/quire-rs#392)",
    },
  ];
  for (const module of modules) {
    const probe = spawnSync("python3", ["-c", `import ${module.name}`], {
      stdio: "pipe",
      encoding: "utf8",
    });
    if (probe.status !== 0) {
      fail(
        `the rendered suite imports \`${module.name}\`, which this interpreter ` +
          `cannot import. Install it with: ${module.install}. The gate installs ` +
          "the Node toolchain the rendered repository pins; the Python side is " +
          "the ambient interpreter's, so its absence is named rather than " +
          "worked around.",
      );
    }
  }
}

function run(command, args, options) {
  const result = spawnSync(command, args, {
    stdio: "inherit",
    encoding: "utf8",
    ...options,
  });
  if (result.status !== 0) {
    fail(
      `\`${command} ${args.join(" ")}\` exited ${result.status ?? "on a signal"} in ` +
        `${options?.cwd ?? ROOT}`,
    );
  }
}

function main() {
  requireTools();
  const failed = [];
  for (const variant of VARIANTS) {
    const scratch = mkdtempSync(
      join(tmpdir(), `quoin-template-gate-${variant.kind}-`),
    );
    const root = join(scratch, variant.repo);
    try {
      console.log(`\n=== ${variant.kind} ===`);
      run("cookiecutter", [
        TEMPLATE,
        "--no-input",
        "-o",
        scratch,
        `module_kind=${variant.kind}`,
        `repo_name=${variant.repo}`,
        ...variant.extra,
      ]);
      if (!existsSync(root))
        fail(`${variant.kind} rendered no ${variant.repo}`);

      run("npm", ["install", "--no-audit", "--no-fund"], { cwd: root });
      run("node", ["scripts/generate-schemas.mjs"], { cwd: root });
      // Emitting twice from an unchanged source must change nothing (NFR-019).
      run("node", ["scripts/generate-schemas.mjs", "--check"], { cwd: root });
      // The rendered repository's own lint gate, on the bytes the template
      // shipped. A generated repository whose first `make gate` fails on
      // formatting is a repository whose gate the maintainer learns to ignore.
      run("python3", ["-m", "ruff", "check", "."], { cwd: root });
      run("python3", ["-m", "black", "--check", "."], { cwd: root });
      run("python3", ["-m", "pytest", "tests", "-q", "-o", "addopts=", "-rs"], {
        cwd: root,
      });
    } catch (error) {
      failed.push(variant.kind);
      console.error(`template-gate: ${variant.kind} failed: ${error.message}`);
    } finally {
      rmSync(scratch, { recursive: true, force: true });
    }
  }
  if (failed.length > 0) {
    fail(`${failed.length} variant(s) failed: ${failed.join(", ")}`);
  }
  console.log(
    `\ntemplate-gate: ${VARIANTS.length} variants rendered, emitted, re-checked and tested`,
  );
}

try {
  main();
} catch (error) {
  console.error(
    `template-gate: ${error instanceof GateError ? error.message : error.stack}`,
  );
  process.exit(1);
}
