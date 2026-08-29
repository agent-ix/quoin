#!/usr/bin/env node

import { spawnSync } from "node:child_process";
import { createHash } from "node:crypto";
import {
  chmodSync,
  copyFileSync,
  existsSync,
  mkdtempSync,
  readFileSync,
  rmSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const ROOT = dirname(dirname(fileURLToPath(import.meta.url)));
const DEFAULT_LOCK = join(ROOT, "quality", "verification-stack-lock.json");
const FULL_SHA = /^[0-9a-f]{40}$/;
const DIGEST = /^sha256:[0-9a-f]{64}$/;

export function sha256(bytes) {
  return `sha256:${createHash("sha256").update(bytes).digest("hex")}`;
}

export function validateLockShape(lock) {
  if (lock?.schemaVersion !== "quoin-verification-stack-lock-v1") {
    throw new Error("verification lock has unsupported schemaVersion");
  }
  const required = [
    "quoin",
    "quire",
    "quire-cli",
    "qa-corpus",
    "filament-ide-rs",
    "spec-artifacts-process",
    "spec-artifacts-iso",
  ];
  for (const name of required) {
    const source = lock.repositories?.[name];
    if (!source || !FULL_SHA.test(source.revision ?? "")) {
      throw new Error(
        `${name} must be locked to one full lowercase commit SHA`,
      );
    }
    if (
      typeof source.remote !== "string" ||
      !source.remote.startsWith("https://")
    ) {
      throw new Error(`${name} must name an HTTPS remote`);
    }
    if (/[#@](main|master|HEAD|v?\d+(?:\.\d+)*)$/.test(source.remote)) {
      throw new Error(`${name} remote contains a moving ref`);
    }
  }
  if (!FULL_SHA.test(lock.cohorts?.qaExternalQuoin?.revision ?? "")) {
    throw new Error("qaExternalQuoin must be locked to one full commit SHA");
  }
  if (!FULL_SHA.test(lock.cohorts?.quireBenchmarkQuoin?.revision ?? "")) {
    throw new Error(
      "quireBenchmarkQuoin must be locked to one full commit SHA",
    );
  }
  const qaCounts = lock.cohorts?.qaCorpus;
  for (const field of ["executableCases", "reportingCases", "totalCases"]) {
    if (!Number.isInteger(qaCounts?.[field]) || qaCounts[field] < 0) {
      throw new Error(`qaCorpus.${field} must be a non-negative integer`);
    }
  }
  if (
    qaCounts.executableCases + qaCounts.reportingCases !==
    qaCounts.totalCases
  ) {
    throw new Error(
      "qaCorpus case counts must partition totalCases into executable and reporting cases",
    );
  }
  if (
    !Array.isArray(lock.requiredCapabilities) ||
    lock.requiredCapabilities.length === 0
  ) {
    throw new Error("verification lock requires a non-empty capability set");
  }
  for (const [name, digest] of Object.entries(lock.artifacts ?? {})) {
    if (!DIGEST.test(digest))
      throw new Error(`${name} is not a full sha256 digest`);
  }
  if (Object.keys(lock.artifacts ?? {}).length === 0) {
    throw new Error("verification lock requires artifact digests");
  }
  for (const field of [
    "caseMilliseconds",
    "corpusMilliseconds",
    "tier1Milliseconds",
    "tier2Milliseconds",
    "installMilliseconds",
    "quoinMilliseconds",
    "spanMilliseconds",
  ]) {
    if (
      !Number.isInteger(lock.timeouts?.[field]) ||
      lock.timeouts[field] < 1000
    ) {
      throw new Error(
        `timeouts.${field} must be an integer of at least 1000ms`,
      );
    }
  }
  const tier1WorstCase =
    qaCounts.executableCases * lock.timeouts.caseMilliseconds;
  if (lock.timeouts.tier1Milliseconds < tier1WorstCase) {
    throw new Error(
      `timeouts.tier1Milliseconds must cover the locked per-case budget ` +
        `(${qaCounts.executableCases} * ${lock.timeouts.caseMilliseconds} = ${tier1WorstCase}ms)`,
    );
  }
  return lock;
}

export function qaCorpusCounts(inventory) {
  if (!Array.isArray(inventory?.cases)) {
    throw new Error("qa-corpus bounds did not emit a cases array");
  }
  const reportingCases = inventory.cases.filter(
    (entry) => entry?.mode === "reporting",
  ).length;
  return {
    executableCases: inventory.cases.length - reportingCases,
    reportingCases,
    totalCases: inventory.cases.length,
  };
}

function assertQaCorpusCounts(lock, root) {
  const inventory = JSON.parse(
    run("python3", [join(root, "bounds.py"), "--json"], {
      cwd: root,
      timeout: lock.timeouts.corpusMilliseconds,
    }),
  );
  const observed = qaCorpusCounts(inventory);
  const expected = lock.cohorts.qaCorpus;
  for (const field of ["executableCases", "reportingCases", "totalCases"]) {
    if (observed[field] !== expected[field]) {
      throw new Error(
        `qa-corpus ${field} drift: expected ${expected[field]}, observed ${observed[field]}`,
      );
    }
  }
  return observed;
}

function run(command, args, options = {}) {
  const timeout = options.timeout ?? 120_000;
  const done = spawnSync(command, args, {
    cwd: options.cwd,
    env: options.env ?? process.env,
    encoding: options.encoding ?? "utf8",
    maxBuffer: 128 * 1024 * 1024,
    timeout,
    stdio: options.stdio ?? ["ignore", "pipe", "pipe"],
  });
  if (done.error || done.status !== 0) {
    const detail =
      done.error?.code === "ETIMEDOUT"
        ? `exceeded ${timeout}ms`
        : String(done.stderr || done.error?.message || "no diagnostic").trim();
    throw new Error(`${command} ${args.join(" ")} failed: ${detail}`);
  }
  return String(done.stdout ?? "");
}

function git(root, ...args) {
  return run("git", ["-C", root, ...args]).trim();
}

function normalizedRemote(value) {
  return value
    .replace(/\.git$/, "")
    .replace(/^git@github\.com:/, "https://github.com/");
}

export function assertRepository(name, root, locked, options = {}) {
  if (!existsSync(join(root, ".git")) && !existsSync(root)) {
    throw new Error(`${name} checkout is missing at ${root}`);
  }
  const status = git(root, "status", "--porcelain=v1", "--untracked-files=all");
  if (status) throw new Error(`${name} checkout is dirty:\n${status}`);
  const head = git(root, "rev-parse", "HEAD");
  if (head !== locked.revision) {
    if (!options.allowEvidenceOverlay) {
      throw new Error(
        `${name} HEAD ${head} does not equal locked ${locked.revision}`,
      );
    }
    const commonAncestor = git(root, "merge-base", locked.revision, head);
    if (commonAncestor !== locked.revision) {
      throw new Error(
        `${name} evidence overlay ${head} does not descend from locked code ${locked.revision}`,
      );
    }
    const overlayCommits = git(
      root,
      "rev-list",
      "--parents",
      `${locked.revision}..${head}`,
    )
      .split("\n")
      .filter(Boolean);
    if (overlayCommits.some((line) => line.trim().split(/\s+/).length !== 2)) {
      throw new Error(
        `${name} evidence overlay ${head} is not a linear, merge-free chain from locked code ${locked.revision}`,
      );
    }
    const changed = git(root, "diff", "--name-only", locked.revision, head)
      .split("\n")
      .filter(Boolean);
    const allowed = options.allowedOverlayPaths ?? [];
    const unexpected = changed.filter(
      (path) =>
        !allowed.some(
          (prefix) => path === prefix || path.startsWith(`${prefix}/`),
        ),
    );
    if (unexpected.length > 0) {
      throw new Error(
        `${name} evidence overlay changes code: ${unexpected.join(", ")}`,
      );
    }
  }
  const remote = normalizedRemote(git(root, "remote", "get-url", "origin"));
  if (remote !== normalizedRemote(locked.remote)) {
    throw new Error(
      `${name} origin ${remote} does not equal locked ${locked.remote}`,
    );
  }
  run("git", ["-C", root, "cat-file", "-e", `${locked.revision}^{commit}`]);
  const refs = git(
    root,
    "for-each-ref",
    "--format=%(refname)",
    "--contains",
    locked.revision,
    "refs/remotes",
  );
  if (!refs) {
    throw new Error(
      `${name} locked commit ${locked.revision} is not reachable from a remote-tracking ref`,
    );
  }
  return {
    revision: locked.revision,
    sourceState: "clean",
    remote: locked.remote,
  };
}

function assertArtifactDigests(lock) {
  for (const [path, expected] of Object.entries(lock.artifacts)) {
    const full = resolve(ROOT, path);
    if (!full.startsWith(`${ROOT}/`) || !existsSync(full)) {
      throw new Error(`locked artifact is missing or outside Quoin: ${path}`);
    }
    const observed = sha256(readFileSync(full));
    if (observed !== expected) {
      throw new Error(
        `artifact drift for ${path}: expected ${expected}, observed ${observed}`,
      );
    }
  }
}

function assertToolchains(lock) {
  const node = process.version.replace(/^v/, "");
  if (node !== lock.toolchains.node) {
    throw new Error(
      `Node drift: expected ${lock.toolchains.node}, observed ${node}`,
    );
  }
  const declaredNode = readFileSync(join(ROOT, ".node-version"), "utf8").trim();
  if (declaredNode !== node)
    throw new Error(`.node-version ${declaredNode} does not match ${node}`);
  const rust = run("rustc", ["--version"]).trim().split(/\s+/)[1];
  if (rust !== lock.toolchains.rust) {
    throw new Error(
      `Rust drift: expected ${lock.toolchains.rust}, observed ${rust}`,
    );
  }
  const python = run("python3", ["--version"]).trim().split(/\s+/)[1];
  if (python !== lock.toolchains.python) {
    throw new Error(
      `Python drift: expected ${lock.toolchains.python}, observed ${python}`,
    );
  }
}

export function cliSelectsEngine(manifest, lockfile, engineRevision) {
  const blocks = lockfile
    .split("[[package]]")
    .filter((block) => /\nname = "quire-rs"\n/.test(block));
  if (blocks.length !== 1 || !blocks[0].includes(`#${engineRevision}`)) {
    throw new Error(
      `quire-cli Cargo.lock does not select locked Quire ${engineRevision}`,
    );
  }
  if (!manifest.includes(`rev = "${engineRevision}"`)) {
    throw new Error(
      `quire-cli Cargo.toml does not pin locked Quire ${engineRevision}`,
    );
  }
  return true;
}

function assertCliSelectsEngine(cliRoot, engineRevision) {
  return cliSelectsEngine(
    readFileSync(join(cliRoot, "Cargo.toml"), "utf8"),
    readFileSync(join(cliRoot, "Cargo.lock"), "utf8"),
    engineRevision,
  );
}

function buildCli(cliRoot, scratch) {
  const target = join(scratch, "cargo-target");
  const output = run(
    "cargo",
    [
      "build",
      "--release",
      "--locked",
      "--message-format=json-render-diagnostics",
    ],
    {
      cwd: cliRoot,
      env: { ...process.env, CARGO_TARGET_DIR: target },
      timeout: 900_000,
    },
  );
  const artifacts = output
    .split("\n")
    .filter(Boolean)
    .map((line) => {
      try {
        return JSON.parse(line);
      } catch {
        return null;
      }
    })
    .filter(
      (row) =>
        row?.reason === "compiler-artifact" &&
        row?.target?.name === "quire" &&
        row?.executable,
    );
  if (artifacts.length !== 1) {
    throw new Error(
      `Cargo reported ${artifacts.length} quire executables, expected exactly one`,
    );
  }
  const snapshot = join(scratch, "quire");
  copyFileSync(artifacts[0].executable, snapshot);
  chmodSync(snapshot, 0o555);
  return snapshot;
}

function buildExternalQuoin(lock, scratch) {
  const cohort = lock.cohorts.qaExternalQuoin;
  const checkout = join(scratch, "qa-external-quoin");
  assertRemoteRevision("qaExternalQuoin", ROOT, cohort.revision);
  run("git", [
    "-C",
    ROOT,
    "worktree",
    "add",
    "--detach",
    checkout,
    cohort.revision,
  ]);
  run("corepack", ["pnpm", "install", "--frozen-lockfile"], {
    cwd: checkout,
    timeout: 600_000,
    stdio: "inherit",
  });
  run("corepack", ["pnpm", "run", "build"], {
    cwd: checkout,
    timeout: 300_000,
    stdio: "inherit",
  });
  const executable = join(checkout, "bin", "quoin.js");
  const version = run(process.execPath, [executable, "--version"], {
    cwd: checkout,
  }).trim();
  if (version !== cohort.version) {
    throw new Error(
      `qaExternalQuoin version ${version} does not equal locked ${cohort.version}`,
    );
  }
  return { checkout, executable };
}

export function assertRemoteRevision(label, root, revision) {
  try {
    git(root, "cat-file", "-e", `${revision}^{commit}`);
  } catch {
    throw new Error(`${label} ${revision} is not a local commit`);
  }
  const refs = git(
    root,
    "for-each-ref",
    "--format=%(refname)",
    "--contains",
    revision,
    "refs/remotes",
  );
  if (!refs) throw new Error(`${label} ${revision} is not remotely reachable`);
}

function assertToolProvenance(binary, lock) {
  const provenance = JSON.parse(run(binary, ["provenance", "--json"]));
  if (provenance.schemaVersion !== "quire-tool-provenance-v1") {
    throw new Error("built CLI emitted unsupported provenance schema");
  }
  const expected = {
    cli: lock.repositories["quire-cli"].revision,
    engine: lock.repositories.quire.revision,
  };
  for (const [layer, revision] of Object.entries(expected)) {
    if (
      provenance[layer]?.sourceRevision !== revision ||
      provenance[layer]?.sourceState !== "clean"
    ) {
      throw new Error(
        `built ${layer} provenance does not equal clean locked ${revision}`,
      );
    }
  }
  const available = new Set(provenance.capabilities ?? []);
  const missing = lock.requiredCapabilities.filter(
    (item) => !available.has(item),
  );
  if (missing.length > 0)
    throw new Error(`built CLI lacks capabilities: ${missing.join(", ")}`);
  return provenance;
}

export function parseSubmoduleRevision(row) {
  const match = row.trim().match(/^([0-9a-f]{40}) corpus(?: |$)/);
  if (!match) {
    throw new Error(
      `qa-corpus submodule is uninitialized or mismatched: ${row.trim()}`,
    );
  }
  return match[1];
}

function submoduleRevision() {
  const row = run("git", ["-C", ROOT, "submodule", "status", "--", "corpus"]);
  return parseSubmoduleRevision(row);
}

export function lockDigest(lockPath) {
  return sha256(readFileSync(lockPath));
}

async function main() {
  const lockPath = resolve(valueOf("--lock") ?? DEFAULT_LOCK);
  const lock = validateLockShape(JSON.parse(readFileSync(lockPath, "utf8")));
  const roots = {
    quoin: ROOT,
    quire: resolve(process.env.QUIRE_ROOT ?? join(ROOT, "..", "quire-rs")),
    "quire-cli": resolve(
      process.env.QUIRE_CLI_ROOT ?? join(ROOT, "..", "quire-cli"),
    ),
    "qa-corpus": resolve(process.env.QA_CORPUS_ROOT ?? join(ROOT, "corpus")),
    "filament-ide-rs": resolve(
      process.env.FILAMENT_IDE_RS_ROOT ?? join(ROOT, "..", "filament-ide-rs"),
    ),
    "spec-artifacts-process": resolve(
      process.env.SPEC_ARTIFACTS_PROCESS_ROOT ??
        join(ROOT, "..", "spec-artifacts-process"),
    ),
    "spec-artifacts-iso": resolve(
      process.env.SPEC_ARTIFACTS_ISO_ROOT ??
        join(ROOT, "..", "spec-artifacts-iso"),
    ),
  };
  const sources = {};
  for (const name of Object.keys(lock.repositories).filter(
    (name) => name !== "quoin",
  )) {
    if (!roots[name])
      throw new Error(`verification lock has no checkout route for ${name}`);
    sources[name] = assertRepository(
      name,
      roots[name],
      lock.repositories[name],
    );
  }
  sources.quoin = assertRepository("quoin", ROOT, lock.repositories.quoin, {
    allowEvidenceOverlay: true,
    allowedOverlayPaths: [
      "quality/verification-stack-lock.json",
      "quality/verification-evidence.json",
      "bench/span-breadth-v1-labels.json",
      "bench/guidance-evaluator-contract-v1.json",
      "bench/guidance-independent-review-v1.json",
      "spec/evidence/measurements",
      "bench/tier1-baseline.json",
      "bench/battletest-baseline.json",
      "corpus/baselines/quoin.json",
    ],
  });
  if (submoduleRevision() !== lock.repositories["qa-corpus"].revision) {
    throw new Error("qa-corpus submodule does not equal the locked revision");
  }
  assertArtifactDigests(lock);
  assertToolchains(lock);
  assertCliSelectsEngine(roots["quire-cli"], lock.repositories.quire.revision);

  const scratch = mkdtempSync(join(tmpdir(), "quoin-stack-"));
  let externalQuoin = null;
  let isolatedQuoinCheckout = null;
  try {
    const binary = buildCli(roots["quire-cli"], scratch);
    const provenance = assertToolProvenance(binary, lock);
    assertRemoteRevision(
      "quireBenchmarkQuoin",
      ROOT,
      lock.cohorts.quireBenchmarkQuoin.revision,
    );
    assertRemoteRevision(
      "qaExternalQuoin",
      ROOT,
      lock.cohorts.qaExternalQuoin.revision,
    );
    sources["quoin-benchmark-corpus"] = {
      revision: lock.cohorts.quireBenchmarkQuoin.revision,
      sourceState: "clean",
      remote: lock.repositories.quoin.remote,
    };
    sources["quoin-qa-external"] = {
      revision: lock.cohorts.qaExternalQuoin.revision,
      sourceState: "clean",
      remote: lock.repositories.quoin.remote,
    };
    const attestation = {
      schemaVersion: "verification-stack-attestation-v1",
      lockDigest: lockDigest(lockPath),
      executableDigest: sha256(readFileSync(binary)),
      buildProfile: "release",
      toolchains: structuredClone(lock.toolchains),
      sources,
      capabilities: [...provenance.capabilities].sort(),
      artifacts: structuredClone(lock.artifacts),
    };
    const attestationPath = join(scratch, "attestation.json");
    writeFileSync(attestationPath, `${JSON.stringify(attestation, null, 2)}\n`);
    if (process.argv.includes("--preflight")) {
      console.log(JSON.stringify(attestation, null, 2));
      return;
    }

    const env = {
      ...process.env,
      QUIRE: binary,
      QUOIN_QUIRE: binary,
      QUOIN_EXPECTED_QUIRE_SHA256: attestation.executableDigest,
      QUOIN_EXPECTED_CLI_REVISION: lock.repositories["quire-cli"].revision,
      QUOIN_EXPECTED_ENGINE_REVISION: lock.repositories.quire.revision,
      QA_EXPECTED_CLI_REVISION: lock.repositories["quire-cli"].revision,
      QA_EXPECTED_ENGINE_REVISION: lock.repositories.quire.revision,
      QUOIN_TIER1_CASE_TIMEOUT_MS: String(lock.timeouts.caseMilliseconds),
      QUOIN_LOCKED_SOURCE_REVISION: lock.repositories.quoin.revision,
    };
    console.error("verification-stack: frozen package install and Quoin gates");
    run("corepack", ["pnpm", "install", "--frozen-lockfile"], {
      cwd: ROOT,
      env,
      timeout: lock.timeouts.installMilliseconds,
      stdio: "inherit",
    });
    run("corepack", ["pnpm", "run", "audit:tool-drift"], {
      cwd: ROOT,
      env,
      stdio: "inherit",
    });
    run("corepack", ["pnpm", "run", "test:tool-drift"], {
      cwd: ROOT,
      env,
      stdio: "inherit",
    });
    run("corepack", ["pnpm", "run", "lint"], {
      cwd: ROOT,
      env,
      timeout: lock.timeouts.quoinMilliseconds,
      stdio: "inherit",
    });
    const testEnv = { ...env };
    delete testEnv.QUOIN_QUIRE;
    delete testEnv.QUOIN_EXPECTED_QUIRE_SHA256;
    run("make", ["test", `QUIRE=${binary}`], {
      cwd: ROOT,
      env: testEnv,
      timeout: lock.timeouts.quoinMilliseconds,
      stdio: "inherit",
    });
    isolatedQuoinCheckout = join(scratch, "quoin-runtime-source");
    run("git", [
      "-C",
      ROOT,
      "worktree",
      "add",
      "--detach",
      isolatedQuoinCheckout,
      lock.repositories.quoin.revision,
    ]);
    run("corepack", ["pnpm", "install", "--frozen-lockfile"], {
      cwd: isolatedQuoinCheckout,
      env,
      timeout: lock.timeouts.installMilliseconds,
      stdio: "inherit",
    });
    run("corepack", ["pnpm", "run", "build"], {
      cwd: isolatedQuoinCheckout,
      env,
      timeout: lock.timeouts.quoinMilliseconds,
      stdio: "inherit",
    });
    const quoinRuntime = join(scratch, "quoin-runtime");
    run(
      "corepack",
      [
        "pnpm",
        "--filter",
        "@agent-ix/quoin",
        "deploy",
        "--prod",
        "--legacy",
        "--frozen-lockfile",
        quoinRuntime,
      ],
      {
        cwd: isolatedQuoinCheckout,
        env,
        timeout: lock.timeouts.installMilliseconds,
        stdio: "inherit",
      },
    );
    const isolatedQuoin = join(quoinRuntime, "bin", "quoin.js");
    const sourceVersion = run(process.execPath, [
      join(ROOT, "bin", "quoin.js"),
      "--version",
    ]).trim();
    const isolatedVersion = run(process.execPath, [
      isolatedQuoin,
      "--version",
    ]).trim();
    if (isolatedVersion !== sourceVersion) {
      throw new Error(
        `isolated Quoin ${isolatedVersion} does not equal built source ${sourceVersion}`,
      );
    }
    console.error("verification-stack: broad span-grounding gate");
    const spanResult = run(
      process.execPath,
      [
        join(ROOT, "scripts", "verify-span-breadth.mjs"),
        "--quire",
        binary,
        "--json",
      ],
      { cwd: ROOT, env, timeout: lock.timeouts.spanMilliseconds },
    );
    const spanResultPath = join(scratch, "span-breadth.json");
    writeFileSync(spanResultPath, `${spanResult}\n`);
    console.error(
      "verification-stack: build the historical QA external producer cohort",
    );
    externalQuoin = buildExternalQuoin(lock, scratch);
    const qaEnv = {
      ...env,
      PATH: `${dirname(binary)}:${process.env.PATH ?? ""}`,
    };
    console.error("verification-stack: qa-corpus canonical CI");
    run(
      "make",
      ["ci", `QUIRE=${binary}`, `QUOIN=${externalQuoin.executable}`],
      {
        cwd: roots["qa-corpus"],
        env: qaEnv,
        timeout: lock.timeouts.corpusMilliseconds,
        stdio: "inherit",
      },
    );
    const qaCounts = assertQaCorpusCounts(lock, roots["qa-corpus"]);
    console.error(
      `verification-stack: locked QA inventory ` +
        `${qaCounts.executableCases} executable + ` +
        `${qaCounts.reportingCases} reporting = ${qaCounts.totalCases}`,
    );
    console.error("verification-stack: Quoin Tier-1 canonical gate");
    const benchmarkArgs = [
      join(ROOT, "scripts", "bench-tier1.mjs"),
      "--quire",
      binary,
      "--quoin",
      isolatedQuoin,
      "--attestation",
      attestationPath,
      "--span-breadth",
      spanResultPath,
    ];
    if (process.argv.includes("--update")) {
      benchmarkArgs.push(
        "--update",
        "--recall-baseline-out",
        join(scratch, "qa-recall-baseline.json"),
      );
    }
    run(process.execPath, benchmarkArgs, {
      cwd: ROOT,
      env,
      timeout: lock.timeouts.tier1Milliseconds,
      stdio: "inherit",
    });
    console.error("verification-stack: Tier-2 immutable cohort gate");
    const tier2Args = [
      join(ROOT, "scripts", "battletest.mjs"),
      "--quire",
      binary,
      "--corpus",
      roots["filament-ide-rs"],
      "--declaration-repo",
      `agent-ix/spec-artifacts-process=${roots["spec-artifacts-process"]}`,
      "--declaration-repo",
      `agent-ix/spec-artifacts-iso=${roots["spec-artifacts-iso"]}`,
    ];
    if (process.argv.includes("--update")) tier2Args.push("--update");
    run(process.execPath, tier2Args, {
      cwd: ROOT,
      env,
      timeout: lock.timeouts.tier2Milliseconds,
      stdio: "inherit",
    });
    const output = valueOf("--evidence-out");
    if (output)
      writeFileSync(
        resolve(output),
        `${JSON.stringify(attestation, null, 2)}\n`,
      );
  } finally {
    if (isolatedQuoinCheckout && existsSync(isolatedQuoinCheckout)) {
      try {
        run("git", [
          "-C",
          ROOT,
          "worktree",
          "remove",
          "--force",
          isolatedQuoinCheckout,
        ]);
      } catch {
        // Scratch removal below is authoritative; stale worktree metadata can
        // be pruned if this cleanup itself is interrupted.
      }
    }
    if (externalQuoin?.checkout && existsSync(externalQuoin.checkout)) {
      try {
        run("git", [
          "-C",
          ROOT,
          "worktree",
          "remove",
          "--force",
          externalQuoin.checkout,
        ]);
      } catch {
        // The temporary root is still removed below; a later `git worktree prune`
        // can discard metadata if the process was interrupted mid-cleanup.
      }
    }
    rmSync(scratch, { recursive: true, force: true });
  }
}

function valueOf(flag) {
  const index = process.argv.indexOf(flag);
  return index < 0 ? null : process.argv[index + 1];
}

if (
  resolve(process.argv[1] ?? "") === resolve(fileURLToPath(import.meta.url))
) {
  await main();
}
