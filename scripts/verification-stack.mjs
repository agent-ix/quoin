#!/usr/bin/env node

import { spawnSync } from "node:child_process";
import { createHash } from "node:crypto";
import {
  chmodSync,
  copyFileSync,
  existsSync,
  lstatSync,
  mkdirSync,
  mkdtempSync,
  readFileSync,
  renameSync,
  readlinkSync,
  rmSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { createRequire } from "node:module";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import {
  decodeGitText,
  literalGit,
  materializeDeclarations,
  sourceNames,
  validateDeclarationShape,
} from "./verification-declarations.mjs";

const ROOT = dirname(dirname(fileURLToPath(import.meta.url)));
const DEFAULT_LOCK = join(ROOT, "quality", "verification-stack-lock.json");
const FULL_SHA = /^[0-9a-f]{40}$/;
const DIGEST = /^sha256:[0-9a-f]{64}$/;
const require = createRequire(import.meta.url);

export function sha256(bytes) {
  return `sha256:${createHash("sha256").update(bytes).digest("hex")}`;
}

export function validateLockShape(lock) {
  const required = sourceNames(lock);
  validateDeclarationShape(lock);
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
    env: { ...(options.env ?? process.env), GIT_NO_REPLACE_OBJECTS: "1" },
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
  return decodeGitText(literalGit(root, args)).trim();
}

function normalizedRemote(value) {
  return value
    .replace(/\.git$/, "")
    .replace(/^git@github\.com:/, "https://github.com/");
}

function remoteTrackingRefsContaining(root, revision) {
  return git(
    root,
    "for-each-ref",
    "--format=%(refname)",
    "--contains",
    revision,
    "refs/remotes",
  );
}

function evidenceHeadForCheckout(name, root, lockedRevision, head, options) {
  const row = git(root, "rev-list", "--parents", "-n", "1", head)
    .split(/\s+/)
    .filter(Boolean);
  const parents = row.slice(1);
  if (parents.length <= 1) return head;
  if (!options.allowTerminalPromotionMerge || parents.length !== 2) {
    throw new Error(
      `${name} evidence overlay ${head} is not a linear chain or one permitted terminal promotion merge from locked code ${lockedRevision}`,
    );
  }
  const [baseParent, evidenceParent] = parents;
  if (git(root, "merge-base", baseParent, evidenceParent) !== baseParent) {
    throw new Error(
      `${name} terminal promotion first parent is not an ancestor of its evidence parent`,
    );
  }
  if (
    git(root, "rev-parse", `${head}^{tree}`) !==
    git(root, "rev-parse", `${evidenceParent}^{tree}`)
  ) {
    throw new Error(
      `${name} terminal promotion tree differs from its evidence parent`,
    );
  }
  if (!remoteTrackingRefsContaining(root, head)) {
    throw new Error(
      `${name} terminal promotion ${head} is not reachable from a remote-tracking ref`,
    );
  }
  return evidenceParent;
}

export function assertRepository(name, root, locked, options = {}) {
  try {
    return assertRepositorySource(name, root, locked, options);
  } catch (cause) {
    throw new Error(
      `${cause.message}\nReview the source change and prepare a candidate with ` +
        `make verification-relock; see quality/verification-stack.md. ` +
        `Relocking does not waive source checks or promote evidence.`,
      { cause },
    );
  }
}

function assertRepositorySource(name, root, locked, options) {
  if (!existsSync(join(root, ".git")) && !existsSync(root)) {
    throw new Error(`${name} checkout is missing at ${root}`);
  }
  const status = git(root, "status", "--porcelain=v1", "--untracked-files=all");
  if (status) throw new Error(`${name} checkout is dirty:\n${status}`);
  const head = git(root, "rev-parse", "HEAD");
  assertTrackedSource(name, root, head);
  if (head !== locked.revision) {
    if (!options.allowEvidenceOverlay) {
      throw new Error(
        `${name} HEAD ${head} does not equal locked ${locked.revision}`,
      );
    }
    const evidenceHead = evidenceHeadForCheckout(
      name,
      root,
      locked.revision,
      head,
      options,
    );
    const commonAncestor = git(
      root,
      "merge-base",
      locked.revision,
      evidenceHead,
    );
    if (commonAncestor !== locked.revision) {
      throw new Error(
        `${name} evidence overlay ${evidenceHead} does not descend from locked code ${locked.revision}`,
      );
    }
    const overlayCommits = git(
      root,
      "rev-list",
      "--parents",
      `${locked.revision}..${evidenceHead}`,
    )
      .split("\n")
      .filter(Boolean);
    if (overlayCommits.some((line) => line.trim().split(/\s+/).length !== 2)) {
      throw new Error(
        `${name} evidence overlay ${evidenceHead} contains a merge before terminal promotion from locked code ${locked.revision}`,
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
  const refs = remoteTrackingRefsContaining(root, locked.revision);
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

/** Git status can conceal assume-unchanged/skip-worktree file modifications. */
function assertTrackedSource(name, root, revision) {
  for (const row of git(root, "ls-tree", "-r", "-z", revision)
    .split("\0")
    .filter(Boolean)) {
    const separator = row.indexOf("\t");
    const [mode, kind, object] = row.slice(0, separator).split(" ");
    const path = row.slice(separator + 1);
    // Gitlink contents are separate source identities, checked at their own
    // consuming boundaries (notably the explicitly selected corpus source).
    if (mode === "160000" && kind === "commit") continue;
    let bytes;
    try {
      const full = join(root, path);
      const stat = lstatSync(full);
      if (mode === "120000" && stat.isSymbolicLink()) {
        bytes = readlinkSync(full, { encoding: "buffer" });
      } else if (
        ["100644", "100755"].includes(mode) &&
        stat.isFile() &&
        Boolean(stat.mode & 0o111) === (mode === "100755")
      ) {
        bytes = readFileSync(full);
      }
    } catch {
      /* absence or a changed file kind is a source mismatch */
    }
    const observed =
      bytes &&
      createHash("sha1")
        .update(`blob ${bytes.length}\0`)
        .update(bytes)
        .digest("hex");
    if (kind !== "blob" || observed !== object) {
      throw new Error(
        `${name} tracked source artifact ${path} differs from git object ${revision}`,
      );
    }
  }
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
  // Rustup resolves the pinned toolchain from the working directory.  Quoin's
  // pin lives in rust/rust-toolchain.toml, so running from the repository root
  // would silently measure the caller's default toolchain instead.
  const rust = run("rustc", ["--version"], { cwd: join(ROOT, "rust") })
    .trim()
    .split(/\s+/)[1];
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

export function cliSelectsEngine(
  manifest,
  lockfile,
  engineRevision,
  engineRemote,
) {
  // The canonical entrypoint must work before node_modules exists. It performs
  // its frozen install after source checks and before reaching this parser.
  const { parse: parseToml } = require("smol-toml");
  const declared = parseToml(manifest);
  const resolved = parseToml(lockfile);
  const isEngine = ([name, dependency]) =>
    name === "quire-rs" || dependency?.package === "quire-rs";
  const selected = Object.entries(declared.dependencies ?? {}).filter(isEngine);
  const otherTables = [
    ...Object.values(declared.target ?? {}).map(
      (target) => target.dependencies,
    ),
    ...Object.values(declared.patch ?? {}),
  ];
  const ambiguous =
    otherTables.some((table) => Object.entries(table ?? {}).some(isEngine)) ||
    Object.keys(declared.replace ?? {}).some((name) =>
      /^quire-rs(?::|$)/.test(name),
    );
  const dependency = selected[0]?.[1];
  if (
    selected.length !== 1 ||
    ambiguous ||
    typeof dependency !== "object" ||
    dependency === null ||
    (dependency.package !== undefined && dependency.package !== "quire-rs") ||
    dependency.rev !== engineRevision ||
    typeof dependency.git !== "string" ||
    !dependency.git.startsWith("https://") ||
    ["branch", "tag", "path", "workspace"].some((key) => key in dependency) ||
    (engineRemote &&
      normalizedRemote(dependency.git) !== normalizedRemote(engineRemote))
  ) {
    throw new Error(
      `quire-cli Cargo.toml does not pin locked Quire ${engineRevision} as one unambiguous normal dependency`,
    );
  }
  const packages = (resolved.package ?? []).filter(
    (entry) => entry.name === "quire-rs",
  );
  let source;
  try {
    const value = packages[0]?.source;
    if (typeof value === "string" && value.startsWith("git+"))
      source = new URL(value.slice(4));
  } catch {
    /* malformed source remains a mismatch */
  }
  if (
    packages.length !== 1 ||
    !source ||
    source.hash !== `#${engineRevision}` ||
    source.searchParams.get("rev") !== engineRevision ||
    [...source.searchParams.keys()].join() !== "rev" ||
    normalizedRemote(`${source.origin}${source.pathname}`) !==
      normalizedRemote(dependency.git)
  ) {
    throw new Error(
      `quire-cli Cargo.lock does not select locked Quire ${engineRevision}`,
    );
  }
  return true;
}

function assertCliSelectsEngine(cliRoot, engineRevision, engineRemote) {
  return cliSelectsEngine(
    readFileSync(join(cliRoot, "Cargo.toml"), "utf8"),
    readFileSync(join(cliRoot, "Cargo.lock"), "utf8"),
    engineRevision,
    engineRemote,
  );
}

function buildCli(cliRoot, scratch) {
  // A caller may retain a Cargo target directory for a constrained host whose
  // linker cannot reliably create a fresh release link. Cargo still validates
  // the exact checkout and lockfile below; the copied executable is then
  // authenticated by its embedded provenance before it can run the campaign.
  const target = resolve(
    process.env.QUIRE_CARGO_TARGET_DIR ?? join(scratch, "cargo-target"),
  );
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

function buildQuoinCore(scratch) {
  const target = resolve(
    process.env.QUOIN_CORE_CARGO_TARGET_DIR ??
      join(scratch, "quoin-core-cargo-target"),
  );
  run(
    "cargo",
    ["build", "-p", "quoin-core", "--locked", "--target-dir", target],
    { cwd: join(ROOT, "rust"), timeout: 900_000, stdio: "inherit" },
  );
  const executable = join(target, "debug", "quoin-core");
  if (!existsSync(executable) || !lstatSync(executable).isFile()) {
    throw new Error("Cargo did not produce the quoin-core executable");
  }
  return executable;
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

const STAGES = [
  "audit-pre",
  "tool-drift",
  "stack-selftest",
  "lint",
  "core",
  "test",
  "runtime",
  "span",
  "qa",
  "guidance",
  "tier1",
  "tier2",
  "final-audit",
];

function stagePlan() {
  const startAt = valueOf("--start-at") ?? "audit-pre";
  const stopAfter = valueOf("--stop-after") ?? "final-audit";
  const stateDir = valueOf("--state-dir");
  const start = STAGES.indexOf(startAt);
  const stop = STAGES.indexOf(stopAfter);
  if (
    !stateDir &&
    (process.argv.includes("--start-at") ||
      process.argv.includes("--stop-after"))
  ) {
    throw new Error("--start-at and --stop-after require --state-dir");
  }
  if (!stateDir) return null;
  if (start < 0 || stop < 0 || start > stop) {
    throw new Error(
      `staged verification requires ordered stages: ${STAGES.join(", ")}`,
    );
  }
  if (process.argv.includes("--preflight")) {
    throw new Error("--preflight cannot use a persistent verification state");
  }
  return { start, stop, stateDir: resolve(stateDir) };
}

function stageRuns(plan, stage) {
  if (!plan) return true;
  const index = STAGES.indexOf(stage);
  return index >= plan.start && index <= plan.stop;
}

function stageEnds(plan, stage) {
  return Boolean(plan && plan.stop === STAGES.indexOf(stage));
}

function stateRecordPath(root, name) {
  return join(root, `${name}.json`);
}

function writeStateRecord(root, name, value) {
  const temporary = `${stateRecordPath(root, name)}.next`;
  writeFileSync(temporary, `${JSON.stringify(value, null, 2)}\n`);
  renameSync(temporary, stateRecordPath(root, name));
}

function readStateRecord(root, name, expectedLockDigest) {
  const path = stateRecordPath(root, name);
  if (!existsSync(path)) {
    throw new Error(
      `verification state lacks the completed ${name} stage; run that stage first`,
    );
  }
  let record;
  try {
    record = JSON.parse(readFileSync(path, "utf8"));
  } catch {
    throw new Error(`verification state ${name} record is not valid JSON`);
  }
  if (record.lockDigest !== expectedLockDigest) {
    throw new Error(
      `verification state ${name} record belongs to a different verification lock`,
    );
  }
  return record;
}

async function main() {
  const plan = stagePlan();
  const lockPath = resolve(valueOf("--lock") ?? DEFAULT_LOCK);
  const lock = validateLockShape(JSON.parse(readFileSync(lockPath, "utf8")));
  const update = process.argv.includes("--update");
  const currentLockDigest = lockDigest(lockPath);
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
    "engineering-assurance": resolve(
      process.env.ENGINEERING_ASSURANCE_ROOT ??
        join(ROOT, "..", "engineering-assurance"),
    ),
  };
  const sources = {};
  const producerEvidenceOverlays = {
    quire: ["spec/evidence/measurements"],
    "qa-corpus": ["spec/evidence/measurements"],
  };
  for (const name of Object.keys(lock.repositories).filter(
    (name) => name !== "quoin",
  )) {
    if (!roots[name])
      throw new Error(`verification lock has no checkout route for ${name}`);
    const allowedOverlayPaths = producerEvidenceOverlays[name];
    sources[name] = assertRepository(
      name,
      roots[name],
      lock.repositories[name],
      allowedOverlayPaths
        ? { allowEvidenceOverlay: true, allowedOverlayPaths }
        : {},
    );
  }
  sources.quoin = assertRepository("quoin", ROOT, lock.repositories.quoin, {
    allowEvidenceOverlay: true,
    allowTerminalPromotionMerge: true,
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
  assertRepository(
    "qa-corpus submodule",
    join(ROOT, "corpus"),
    lock.repositories["qa-corpus"],
  );
  assertArtifactDigests(lock);
  assertToolchains(lock);
  console.error("verification-stack: frozen package install");
  run("corepack", ["pnpm", "install", "--frozen-lockfile"], {
    cwd: ROOT,
    timeout: lock.timeouts.installMilliseconds,
    stdio: "inherit",
  });
  assertCliSelectsEngine(
    roots["quire-cli"],
    lock.repositories.quire.revision,
    lock.repositories.quire.remote,
  );

  if (plan && !existsSync(plan.stateDir)) {
    if (plan.start !== 0) {
      throw new Error(
        `verification state ${plan.stateDir} does not exist; begin at gates`,
      );
    }
    mkdirSync(plan.stateDir, { recursive: false, mode: 0o700 });
  }
  const scratch = plan?.stateDir ?? mkdtempSync(join(tmpdir(), "quoin-stack-"));
  const transient = plan
    ? mkdtempSync(join(tmpdir(), "quoin-stack-stage-"))
    : scratch;
  let externalQuoin = null;
  let isolatedQuoinCheckout = null;
  try {
    const declarationRoots =
      lock.schemaVersion === "quoin-verification-stack-lock-v2"
        ? materializeDeclarations(
            lock,
            roots,
            join(transient, "validation-modules"),
          )
        : null;
    const declarationManifest = join(transient, "validation-module-roots.json");
    if (declarationRoots)
      writeFileSync(
        declarationManifest,
        `${JSON.stringify(declarationRoots)}\n`,
        { flag: "wx" },
      );
    let binary;
    if (plan && plan.start > 0) {
      const prepared = readStateRecord(scratch, "prepared", currentLockDigest);
      binary = join(scratch, "quire");
      if (
        !existsSync(binary) ||
        sha256(readFileSync(binary)) !== prepared.executableDigest
      ) {
        throw new Error(
          "verification state Quire executable does not match preparation record",
        );
      }
    } else {
      binary = buildCli(roots["quire-cli"], scratch);
    }
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
      lockDigest: currentLockDigest,
      executableDigest: sha256(readFileSync(binary)),
      buildProfile: "release",
      toolchains: structuredClone(lock.toolchains),
      sources,
      capabilities: [...provenance.capabilities].sort(),
      artifacts: structuredClone(lock.artifacts),
    };
    const attestationPath = join(scratch, "attestation.json");
    writeFileSync(attestationPath, `${JSON.stringify(attestation, null, 2)}\n`);
    if (plan && plan.start === 0) {
      writeStateRecord(scratch, "prepared", {
        lockDigest: currentLockDigest,
        executableDigest: attestation.executableDigest,
      });
    }
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
    // An evidence refresh starts from evidence whose producer pin is
    // deliberately stale. Running the provenance audit before that refresh
    // makes `make bench-tier1-update` impossible. The update route therefore
    // records this intentionally deferred gate and replays it at final-audit.
    if (stageRuns(plan, "audit-pre")) {
      if (!update) {
        run("corepack", ["pnpm", "run", "audit:tool-drift"], {
          cwd: ROOT,
          env,
          stdio: "inherit",
        });
      }
      if (plan)
        writeStateRecord(scratch, "audit-pre", {
          lockDigest: currentLockDigest,
        });
    }
    if (stageEnds(plan, "audit-pre")) return;
    if (plan && plan.start > STAGES.indexOf("audit-pre")) {
      readStateRecord(scratch, "audit-pre", currentLockDigest);
    }
    if (stageRuns(plan, "tool-drift")) {
      run("corepack", ["pnpm", "run", "test:tool-drift"], {
        cwd: ROOT,
        env,
        stdio: "inherit",
      });
      if (plan)
        writeStateRecord(scratch, "tool-drift", {
          lockDigest: currentLockDigest,
        });
    }
    if (stageEnds(plan, "tool-drift")) return;
    if (plan && plan.start > STAGES.indexOf("tool-drift")) {
      readStateRecord(scratch, "tool-drift", currentLockDigest);
    }
    if (stageRuns(plan, "stack-selftest")) {
      run("corepack", ["pnpm", "run", "test:verification-stack"], {
        cwd: ROOT,
        env,
        stdio: "inherit",
      });
      if (plan)
        writeStateRecord(scratch, "stack-selftest", {
          lockDigest: currentLockDigest,
        });
    }
    if (stageEnds(plan, "stack-selftest")) return;
    if (plan && plan.start > STAGES.indexOf("stack-selftest")) {
      readStateRecord(scratch, "stack-selftest", currentLockDigest);
    }
    if (stageRuns(plan, "lint")) {
      const lintOutput = run("corepack", ["pnpm", "run", "lint"], {
        cwd: ROOT,
        env,
        timeout: lock.timeouts.quoinMilliseconds,
      });
      process.stdout.write(lintOutput);
      if (plan)
        writeStateRecord(scratch, "lint", { lockDigest: currentLockDigest });
    }
    if (stageEnds(plan, "lint")) return;
    if (plan && plan.start > STAGES.indexOf("lint")) {
      readStateRecord(scratch, "lint", currentLockDigest);
    }
    let quoinCore;
    if (stageRuns(plan, "core")) {
      quoinCore = buildQuoinCore(scratch);
      if (plan) {
        writeStateRecord(scratch, "core", {
          lockDigest: currentLockDigest,
          executable: quoinCore,
          executableDigest: sha256(readFileSync(quoinCore)),
        });
      }
    }
    if (stageEnds(plan, "core")) return;
    if (plan && plan.start > STAGES.indexOf("core")) {
      const coreRecord = readStateRecord(scratch, "core", currentLockDigest);
      quoinCore = resolve(coreRecord.executable ?? "");
      if (
        !existsSync(quoinCore) ||
        sha256(readFileSync(quoinCore)) !== coreRecord.executableDigest
      ) {
        throw new Error(
          "verification state quoin-core executable does not match core record",
        );
      }
    }
    if (stageRuns(plan, "test")) {
      quoinCore ??= buildQuoinCore(scratch);
      const requestedShard = valueOf("--test-shard");
      const requestedTemplateGroup = valueOf("--template-group");
      if (requestedShard && !plan) {
        throw new Error("--test-shard requires --state-dir");
      }
      if (requestedTemplateGroup && (!requestedShard || Number(requestedShard) !== 2)) {
        throw new Error("--template-group requires --test-shard 2");
      }
      const templateGroup = requestedTemplateGroup
        ? Number(requestedTemplateGroup)
        : null;
      if (
        templateGroup !== null &&
        (!Number.isInteger(templateGroup) || templateGroup < 1 || templateGroup > 9)
      ) {
        throw new Error("--template-group must be an integer from 1 through 9");
      }
      const testShards = requestedShard
        ? [Number(requestedShard)]
        : Array.from({ length: 20 }, (_, index) => index + 1);
      if (
        testShards.some(
          (shard) => !Number.isInteger(shard) || shard < 1 || shard > 20,
        )
      ) {
        throw new Error("--test-shard must be an integer from 1 through 20");
      }
      if (plan && requestedShard && testShards[0] > 1) {
        readStateRecord(
          scratch,
          `test-${testShards[0] - 1}`,
          currentLockDigest,
        );
      }
      const testEnv = { ...env };
      // Do not let an inherited explicit-set override change either replay mode.
      delete testEnv.QUOIN_VERIFICATION_DECLARATIONS;
      if (declarationRoots)
        testEnv.QUOIN_VERIFICATION_DECLARATIONS = declarationManifest;
      // The suite deliberately substitutes fake `quire` executables through
      // PATH to grade subprocess failures. The Make target prepends the exact,
      // already-hashed binary as the default; individual fixtures may still
      // override it without inheriting a digest that belongs to another file.
      delete testEnv.QUOIN_QUIRE;
      delete testEnv.QUOIN_EXPECTED_QUIRE_SHA256;
      testEnv.QUOIN_CORE = quoinCore;
      // The test suite is unchanged; controlled shards keep each real
      // subprocess beneath constrained-host command lifetimes. A stage record
      // is written only after every shard, including its build/validation
      // prerequisites, has completed.
      for (const shard of testShards) {
        if (plan && shard === 2 && templateGroup !== null && templateGroup > 1) {
          readStateRecord(
            scratch,
            `test-2-group-${templateGroup - 1}`,
            currentLockDigest,
          );
        }
        const invocations =
          shard === 2
            ? [
                "the renderer is present",
                "variants render from one core",
                "an invalid input is refused, naming the value",
                "the rendered tree conforms and carries no residue",
                "the rendered repository is public-ready",
                "the rendered governance tree validates as rendered",
                "the template depends on shared tooling rather than copying it",
                "the conformance contract tracks the maintained repositories",
                "the rendered suite carries the rows the template gate executes",
              ].filter((_, index) => !templateGroup || index + 1 === templateGroup).map((name) => [
                `VITEST_FILE=tests/semantic-module-template.test.ts`,
                `VITEST_NAME=^${name}`,
              ])
            : [[`VITEST_ARGS=--shard=${shard}/20`]];
        for (const invocation of invocations) {
          run("make", ["test-with-quire", `QUIRE=${binary}`, ...invocation], {
            cwd: ROOT,
            env: testEnv,
            timeout: lock.timeouts.quoinMilliseconds,
            stdio: "inherit",
          });
        }
        if (plan && shard === 2 && templateGroup !== null) {
          writeStateRecord(scratch, `test-2-group-${templateGroup}`, {
            lockDigest: currentLockDigest,
          });
          if (templateGroup === 9) {
            for (const group of Array.from({ length: 9 }, (_, index) => index + 1)) {
              readStateRecord(scratch, `test-2-group-${group}`, currentLockDigest);
            }
            writeStateRecord(scratch, "test-2", { lockDigest: currentLockDigest });
          }
        } else if (plan) {
          writeStateRecord(scratch, `test-${shard}`, {
            lockDigest: currentLockDigest,
          });
        }
      }
      if (plan && (!requestedShard || testShards[0] === 20)) {
        for (const shard of Array.from(
          { length: 20 },
          (_, index) => index + 1,
        )) {
          readStateRecord(scratch, `test-${shard}`, currentLockDigest);
        }
        writeStateRecord(scratch, "test", { lockDigest: currentLockDigest });
      }
    }
    if (stageEnds(plan, "test")) return;
    if (plan && plan.start > STAGES.indexOf("test")) {
      readStateRecord(scratch, "test", currentLockDigest);
    }

    let isolatedQuoin;
    const runtimeRecord =
      plan && plan.start > STAGES.indexOf("runtime")
        ? readStateRecord(scratch, "runtime", currentLockDigest)
        : null;
    if (runtimeRecord) {
      isolatedQuoinCheckout = join(scratch, "quoin-runtime-source");
      isolatedQuoin = join(scratch, "quoin-runtime", "bin", "quoin.js");
      if (
        !existsSync(isolatedQuoin) ||
        sha256(readFileSync(isolatedQuoin)) !== runtimeRecord.executableDigest
      ) {
        throw new Error(
          "verification state Quoin runtime does not match runtime record",
        );
      }
      assertRepository(
        "verification state Quoin runtime source",
        isolatedQuoinCheckout,
        lock.repositories.quoin,
      );
    }
    if (stageRuns(plan, "runtime")) {
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
      isolatedQuoin = join(quoinRuntime, "bin", "quoin.js");
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
      if (plan) {
        writeStateRecord(scratch, "runtime", {
          lockDigest: currentLockDigest,
          executableDigest: sha256(readFileSync(isolatedQuoin)),
        });
      }
    }
    if (stageEnds(plan, "runtime")) return;
    let spanResultPath = join(scratch, "span-breadth.json");
    if (stageRuns(plan, "span")) {
      if (update) {
        console.error("verification-stack: refresh reviewed span evidence");
        run(
          process.execPath,
          [join(isolatedQuoinCheckout, "scripts", "freeze-span-breadth.mjs")],
          {
            cwd: isolatedQuoinCheckout,
            env: {
              ...env,
              QUIRE: binary,
              QUIRE_ROOT: roots.quire,
              FILAMENT_IDE_RS_ROOT: roots["filament-ide-rs"],
              QUOIN_LABEL_REVISION: lock.repositories.quoin.revision,
            },
            stdio: "inherit",
          },
        );
        copyFileSync(
          join(isolatedQuoinCheckout, "bench", "span-breadth-v1-labels.json"),
          join(ROOT, "bench", "span-breadth-v1-labels.json"),
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
      writeFileSync(spanResultPath, `${spanResult}\n`);
      if (plan) {
        writeStateRecord(scratch, "span", {
          lockDigest: currentLockDigest,
          digest: sha256(readFileSync(spanResultPath)),
        });
      }
    } else if (plan && plan.start > STAGES.indexOf("span")) {
      const spanRecord = readStateRecord(scratch, "span", currentLockDigest);
      if (
        !existsSync(spanResultPath) ||
        sha256(readFileSync(spanResultPath)) !== spanRecord.digest
      ) {
        throw new Error(
          "verification state span result does not match span record",
        );
      }
    }
    if (stageEnds(plan, "span")) return;
    if (stageRuns(plan, "qa")) {
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
      if (plan)
        writeStateRecord(scratch, "qa", { lockDigest: currentLockDigest });
    }
    if (stageEnds(plan, "qa")) return;
    if (stageRuns(plan, "guidance")) {
      if (update) {
        const guidanceCandidate = join(scratch, "guidance-candidate.json");
        console.error("verification-stack: refresh reviewed guidance evidence");
        run(
          process.execPath,
          [
            join(ROOT, "scripts", "bench-tier1.mjs"),
            "--experimental",
            "--quire",
            binary,
            "--quoin",
            isolatedQuoin,
            "--guidance-candidate-out",
            guidanceCandidate,
            "--guidance-candidate-only",
          ],
          {
            cwd: ROOT,
            env,
            timeout: lock.timeouts.tier1Milliseconds,
            stdio: "inherit",
          },
        );
        run(
          process.execPath,
          [
            join(ROOT, "scripts", "freeze-guidance-review.mjs"),
            "--candidate",
            guidanceCandidate,
          ],
          { cwd: ROOT, env, stdio: "inherit" },
        );
      }
      if (plan)
        writeStateRecord(scratch, "guidance", {
          lockDigest: currentLockDigest,
        });
    }
    if (stageEnds(plan, "guidance")) return;

    if (plan && update && plan.start > STAGES.indexOf("guidance")) {
      readStateRecord(scratch, "guidance", currentLockDigest);
    }
    if (stageRuns(plan, "tier1")) {
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
      if (update) {
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
      if (plan)
        writeStateRecord(scratch, "tier1", { lockDigest: currentLockDigest });
    }
    if (stageEnds(plan, "tier1")) return;
    if (stageRuns(plan, "tier2")) {
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
      if (update) tier2Args.push("--update");
      run(process.execPath, tier2Args, {
        cwd: ROOT,
        env,
        timeout: lock.timeouts.tier2Milliseconds,
        stdio: "inherit",
      });
      if (plan)
        writeStateRecord(scratch, "tier2", { lockDigest: currentLockDigest });
    }
    if (stageEnds(plan, "tier2")) return;
    if (stageRuns(plan, "final-audit") && update) {
      if (plan) {
        for (const name of ["tier1", "tier2"]) {
          readStateRecord(scratch, name, currentLockDigest);
        }
      }
      console.error("verification-stack: refreshed evidence provenance audit");
      run("corepack", ["pnpm", "run", "audit:tool-drift"], {
        cwd: ROOT,
        env,
        stdio: "inherit",
      });
    }
    if (plan && stageRuns(plan, "final-audit")) {
      writeStateRecord(scratch, "final-audit", {
        lockDigest: currentLockDigest,
      });
    }
    const output = valueOf("--evidence-out");
    if (output)
      writeFileSync(
        resolve(output),
        `${JSON.stringify(attestation, null, 2)}\n`,
      );
  } finally {
    if (!plan && isolatedQuoinCheckout && existsSync(isolatedQuoinCheckout)) {
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
    if (plan) rmSync(transient, { recursive: true, force: true });
    else rmSync(scratch, { recursive: true, force: true });
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
