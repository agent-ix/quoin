#!/usr/bin/env node

import { execFileSync } from "node:child_process";
import {
  mkdtempSync,
  readFileSync,
  realpathSync,
  rmSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { dirname, isAbsolute, join, relative, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import {
  committedTree,
  describeModule,
  literalGit,
  sourceNames,
  V1_SOURCES,
  V2_SOURCES,
  verifyDeclarations,
  writeCommittedTree,
} from "./verification-declarations.mjs";
import {
  assertRemoteRevision,
  assertRepository,
  cliSelectsEngine,
  qaCorpusCounts,
  sha256,
  validateLockShape,
} from "./verification-stack.mjs";

const ROOT = dirname(dirname(fileURLToPath(import.meta.url)));
export const SOURCE_NAMES = V1_SOURCES;
// The one vendored Quire schema quoin still carries. quoin#502 linked the
// engine into `quoin-core` and deleted `src/quire/`, which took the other four
// with it: `coverage-v1`, `properties-v1`, `clause-binding-v1` and
// `clause-diff-v1` were read by the TypeScript that no longer exists, and the
// shapes they described are now Rust types the engine and quoin share by
// linking rather than by copying. `assurance-v1` survives because it describes
// a document that arrives from OUTSIDE — see `quoin-quire/src/schema.rs`.
const SCHEMAS = ["assurance-v1.schema.json"];
const CONTRACT_SOURCE = "rust/crates/quoin-quire/src/schema.rs";
const SCHEMA_DIR = "rust/crates/quoin-quire/schemas";
const RELOCK_ARTIFACTS = [
  "scripts/verification-relock.mjs",
  "scripts/verification-relock-selftest.mjs",
  "scripts/verification-declarations.mjs",
  "scripts/verification-declarations-selftest.mjs",
  "scripts/verification-object-integrity-selftest.mjs",
  "scripts/workspace-policy-selftest.mjs",
];

function git(root, ...args) {
  return literalGit(root, args);
}

export function parseArguments(args) {
  const options = { roots: {} };
  for (let index = 0; index < args.length; index += 2) {
    const flag = args[index];
    const value = args[index + 1];
    if (!value || value.startsWith("--")) {
      throw new Error(`${flag} requires a value`);
    }
    if (flag === "--root") {
      const separator = value.indexOf("=");
      const name = value.slice(0, separator);
      const path = value.slice(separator + 1);
      if (!V2_SOURCES.includes(name) || !isAbsolute(path)) {
        throw new Error(
          `--root requires a known source and absolute path: ${value}`,
        );
      }
      if (options.roots[name]) throw new Error(`duplicate --root ${name}`);
      options.roots[name] = path;
    } else if (flag === "--lock" || flag === "--out") {
      const key = flag.slice(2);
      if (options[key]) throw new Error(`duplicate ${flag}`);
      options[key] = resolve(value);
    } else {
      throw new Error(`unknown argument ${flag}`);
    }
  }
  if (!options.out) throw new Error("--out requires a new candidate filename");
  for (const name of SOURCE_NAMES) {
    if (!options.roots[name])
      throw new Error(`missing explicit --root ${name}`);
  }
  return options;
}

function artifactPath(root, path) {
  const full = realpathSync(resolve(root, path));
  const within = relative(realpathSync(root), full);
  if (!within || within.startsWith("../") || isAbsolute(within)) {
    throw new Error(`locked artifact is outside Quoin: ${path}`);
  }
  return full;
}

function committedBytes(root, revision, path) {
  const entry = git(root, "ls-tree", revision, "--", path).toString().trim();
  if (!/^100(?:644|755) blob [0-9a-f]{40}\t/.test(entry)) {
    throw new Error(
      `artifact ${path} is not a regular file in git object ${revision}`,
    );
  }
  const bytes = git(root, "show", `${revision}:${path}`);
  if (!bytes.equals(readFileSync(artifactPath(root, path)))) {
    throw new Error(
      `artifact ${path} differs from committed git object ${revision}`,
    );
  }
  return bytes;
}

export function committedInventory(root, revision, timeout) {
  const scratch = mkdtempSync(join(tmpdir(), "quoin-relock-inventory-"));
  try {
    const snapshot = join(scratch, "source");
    writeCommittedTree(committedTree(root, revision, "", timeout), snapshot);
    return JSON.parse(
      execFileSync("python3", ["-I", join(snapshot, "bounds.py"), "--json"], {
        cwd: snapshot,
        encoding: "utf8",
        timeout,
        maxBuffer: 128 * 1024 * 1024,
        stdio: ["ignore", "pipe", "pipe"],
      }),
    );
  } finally {
    rmSync(scratch, { recursive: true, force: true });
  }
}

/**
 * Read the vendored contract's provenance constants.
 *
 * Until quoin#502 this parsed the `QUIRE_CONTRACT` object literal out of
 * `src/quire/contract.ts` with the TypeScript AST, because the value sat in
 * executable TypeScript beside functions that read it. The constants now live
 * in `rust/crates/quoin-quire/src/schema.rs` as `pub const` string literals
 * with nothing else on the declaration, so a literal match is exact and a
 * second parser is not worth carrying. The whitespace between `=` and the
 * literal is permissive only because rustfmt wraps the 64-character digest
 * onto its own line; the literal itself is still matched exactly, and the
 * declaration must still be the only thing on the line it starts.
 */
export function parseContract(source) {
  const constant = (name, pattern) => {
    const matches = [
      ...source.matchAll(
        new RegExp(`^pub const ${name}: &str =\\s+"(${pattern})";$`, "gm"),
      ),
    ];
    if (matches.length !== 1) {
      throw new Error(
        `expected one exact literal ${name} in ${CONTRACT_SOURCE}`,
      );
    }
    return matches[0][1];
  };
  return {
    revision: constant("VENDORED_SOURCE_REVISION", "[0-9a-f]{40}"),
    hashes: {
      "assurance-v1.schema.json": constant("VENDORED_SHA256", "[0-9a-f]{64}"),
    },
  };
}

// TC-1589 / FR-043-AC-32: candidate preparation is not evidence promotion.
export function prepareCandidate(base, roots) {
  validateLockShape(base);
  const names = sourceNames(base);
  if (
    Object.keys(base.repositories).sort().join() !== [...names].sort().join()
  ) {
    throw new Error(
      "candidate requires exactly the known source repositories for its explicit schema version",
    );
  }
  if (Object.keys(roots).some((name) => !names.includes(name)))
    throw new Error("candidate requires exactly its explicit source root set");
  const candidate = structuredClone(base);
  for (const name of names) {
    if (!roots[name] || !isAbsolute(roots[name])) {
      throw new Error(`missing explicit absolute root for ${name}`);
    }
    const revision = git(roots[name], "rev-parse", "HEAD").toString().trim();
    const source = { ...base.repositories[name], revision };
    assertRepository(name, roots[name], source);
    candidate.repositories[name] = source;
  }
  const quoin = roots.quoin;
  if (base.schemaVersion === "quoin-verification-stack-lock-v2")
    verifyDeclarations(base, roots);
  const engine = roots.quire;
  const engineRevision = candidate.repositories.quire.revision;
  cliSelectsEngine(
    committedBytes(
      roots["quire-cli"],
      candidate.repositories["quire-cli"].revision,
      "Cargo.toml",
    ).toString(),
    committedBytes(
      roots["quire-cli"],
      candidate.repositories["quire-cli"].revision,
      "Cargo.lock",
    ).toString(),
    engineRevision,
    candidate.repositories.quire.remote,
  );
  const submodule = git(quoin, "ls-tree", "HEAD", "--", "corpus")
    .toString()
    .trim();
  if (
    submodule !==
    `160000 commit ${candidate.repositories["qa-corpus"].revision}\tcorpus`
  ) {
    throw new Error(
      "Quoin committed corpus gitlink does not equal selected qa-corpus",
    );
  }
  assertRepository(
    "qa-corpus submodule",
    join(quoin, "corpus"),
    candidate.repositories["qa-corpus"],
  );
  for (const name of ["qaExternalQuoin", "quireBenchmarkQuoin"]) {
    assertRemoteRevision(name, quoin, base.cohorts[name].revision);
  }
  const { revision, hashes } = parseContract(
    committedBytes(
      quoin,
      candidate.repositories.quoin.revision,
      CONTRACT_SOURCE,
    ).toString(),
  );
  assertRemoteRevision("vendored Quire contract", engine, revision);
  for (const name of SCHEMAS) {
    const bytes = committedBytes(
      quoin,
      candidate.repositories.quoin.revision,
      `${SCHEMA_DIR}/${name}`,
    );
    if (sha256(bytes) !== `sha256:${hashes[name]}`) {
      throw new Error(`vendored schema hash drift: ${name}`);
    }
    for (const sourceRevision of new Set([revision, engineRevision])) {
      if (
        !bytes.equals(
          git(engine, "show", `${sourceRevision}:schemas/output/${name}`),
        )
      ) {
        throw new Error(
          `vendored schema ${name} differs from engine git object ${sourceRevision}; review and refresh schemas before relocking`,
        );
      }
    }
  }
  candidate.contracts = {
    ...base.contracts,
    quire: { remote: candidate.repositories.quire.remote, revision },
  };
  const inventory = committedInventory(
    roots["qa-corpus"],
    candidate.repositories["qa-corpus"].revision,
    base.timeouts.corpusMilliseconds,
  );
  candidate.cohorts.qaCorpus = qaCorpusCounts(inventory);
  if (base.schemaVersion === "quoin-verification-stack-lock-v2") {
    candidate.declarations.quoinValidation =
      base.declarations.quoinValidation.map(({ repository, path }) => ({
        repository,
        path,
        ...describeModule(
          roots[repository],
          candidate.repositories[repository].revision,
          path,
          base.timeouts.installMilliseconds,
        ),
      }));
  }
  for (const path of new Set([
    ...Object.keys(base.artifacts),
    ...RELOCK_ARTIFACTS,
  ])) {
    const corpus = path.startsWith("corpus/");
    const bytes = committedBytes(
      corpus ? roots["qa-corpus"] : quoin,
      candidate.repositories[corpus ? "qa-corpus" : "quoin"].revision,
      corpus ? path.slice("corpus/".length) : path,
    );
    // A separately routed corpus and Quoin's actual submodule must expose the same bytes.
    if (!bytes.equals(readFileSync(artifactPath(quoin, path))))
      throw new Error(`artifact ${path} differs from selected git object`);
    candidate.artifacts[path] = sha256(bytes);
  }
  // The inventory reader is executable source. Recheck that every input stayed
  // at the measured clean commit before allowing any candidate output.
  for (const name of names) {
    assertRepository(name, roots[name], candidate.repositories[name]);
  }
  return validateLockShape(candidate);
}

// TC-1591 / FR-043-AC-33: exclusive creation, never a lock overwrite.
export function writeCandidate(path, candidate) {
  writeFileSync(path, `${JSON.stringify(candidate, null, 2)}\n`, {
    flag: "wx",
  });
}

if (resolve(process.argv[1] ?? "") === fileURLToPath(import.meta.url)) {
  try {
    const options = parseArguments(process.argv.slice(2));
    const base = JSON.parse(
      readFileSync(
        options.lock ?? join(ROOT, "quality/verification-stack-lock.json"),
        "utf8",
      ),
    );
    const candidate = prepareCandidate(base, options.roots);
    writeCandidate(options.out, candidate);
    console.error(
      `Prepared candidate ${options.out}; no evidence or baseline was updated. Review quality/verification-stack.md before canonical replay.`,
    );
  } catch (error) {
    console.error(
      `verification-relock: ${error.message}\nSee quality/verification-stack.md.`,
    );
    process.exitCode = 1;
  }
}
