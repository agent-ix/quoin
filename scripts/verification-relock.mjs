#!/usr/bin/env node

import { execFileSync } from "node:child_process";
import { readFileSync, realpathSync, writeFileSync } from "node:fs";
import { dirname, isAbsolute, join, relative, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import {
  assertRemoteRevision,
  assertRepository,
  cliSelectsEngine,
  qaCorpusCounts,
  sha256,
  validateLockShape,
} from "./verification-stack.mjs";

const ROOT = dirname(dirname(fileURLToPath(import.meta.url)));
export const SOURCE_NAMES = [
  "quoin",
  "quire",
  "quire-cli",
  "qa-corpus",
  "filament-ide-rs",
  "spec-artifacts-process",
  "spec-artifacts-iso",
];
const SCHEMAS = [
  "assurance-v1.schema.json",
  "coverage-v1.schema.json",
  "properties-v1.schema.json",
];
const RELOCK_ARTIFACTS = [
  "scripts/verification-relock.mjs",
  "scripts/verification-relock-selftest.mjs",
];

function git(root, ...args) {
  return execFileSync("git", ["-C", root, ...args], {
    timeout: 120_000,
    maxBuffer: 128 * 1024 * 1024,
    stdio: ["ignore", "pipe", "pipe"],
  });
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
      if (!SOURCE_NAMES.includes(name) || !isAbsolute(path)) {
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

// TC-1589 / FR-043-AC-32: candidate preparation is not evidence promotion.
export function prepareCandidate(base, roots) {
  validateLockShape(base);
  if (
    Object.keys(base.repositories).sort().join() !==
    [...SOURCE_NAMES].sort().join()
  ) {
    throw new Error(
      "candidate requires exactly the seven known source repositories",
    );
  }
  const candidate = structuredClone(base);
  for (const name of SOURCE_NAMES) {
    if (!roots[name] || !isAbsolute(roots[name])) {
      throw new Error(`missing explicit absolute root for ${name}`);
    }
    const revision = git(roots[name], "rev-parse", "HEAD").toString().trim();
    const source = { ...base.repositories[name], revision };
    assertRepository(name, roots[name], source);
    candidate.repositories[name] = source;
  }
  const quoin = roots.quoin;
  const engine = roots.quire;
  const engineRevision = candidate.repositories.quire.revision;
  cliSelectsEngine(
    readFileSync(join(roots["quire-cli"], "Cargo.toml"), "utf8"),
    readFileSync(join(roots["quire-cli"], "Cargo.lock"), "utf8"),
    engineRevision,
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
  const contract = readFileSync(join(quoin, "src/quire/contract.ts"), "utf8");
  const revision = /sourceRevision:\s*"([0-9a-f]{40})"/.exec(contract)?.[1];
  if (!revision)
    throw new Error("vendored contract has no exact sourceRevision");
  assertRemoteRevision("vendored Quire contract", engine, revision);
  for (const name of SCHEMAS) {
    const bytes = readFileSync(join(quoin, "src/quire/schemas", name));
    const expectedHash = new RegExp(
      `"${name.replaceAll(".", "\\.")}":\\s*"([0-9a-f]{64})"`,
    ).exec(contract)?.[1];
    if (sha256(bytes) !== `sha256:${expectedHash}`) {
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
  const inventory = JSON.parse(
    execFileSync("python3", [join(roots["qa-corpus"], "bounds.py"), "--json"], {
      cwd: roots["qa-corpus"],
      encoding: "utf8",
      timeout: base.timeouts.corpusMilliseconds,
      maxBuffer: 128 * 1024 * 1024,
      stdio: ["ignore", "pipe", "pipe"],
    }),
  );
  candidate.cohorts.qaCorpus = qaCorpusCounts(inventory);
  for (const path of new Set([
    ...Object.keys(base.artifacts),
    ...RELOCK_ARTIFACTS,
  ])) {
    candidate.artifacts[path] = sha256(readFileSync(artifactPath(quoin, path)));
  }
  // The inventory reader is executable source. Recheck that every input stayed
  // at the measured clean commit before allowing any candidate output.
  for (const name of SOURCE_NAMES) {
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
