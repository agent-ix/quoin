#!/usr/bin/env node

import { execFileSync } from "node:child_process";
import { readFileSync, realpathSync, writeFileSync } from "node:fs";
import { dirname, isAbsolute, join, relative, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import ts from "typescript";
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

/** Read the exported literal, not comments or executed TypeScript code. */
export function parseContract(source) {
  const file = ts.createSourceFile(
    "contract.ts",
    source,
    ts.ScriptTarget.Latest,
    true,
    ts.ScriptKind.TS,
  );
  if (file.parseDiagnostics.length)
    throw new Error("vendored contract contains invalid TypeScript");
  const declarations = file.statements
    .filter(
      (statement) =>
        ts.isVariableStatement(statement) &&
        statement.modifiers?.some(
          (modifier) => modifier.kind === ts.SyntaxKind.ExportKeyword,
        ),
    )
    .flatMap((statement) =>
      statement.declarationList.declarations
        .filter(
          (declaration) =>
            ts.isIdentifier(declaration.name) &&
            declaration.name.text === "QUIRE_CONTRACT",
        )
        .map((declaration) => ({
          declaration,
          constant: Boolean(
            statement.declarationList.flags & ts.NodeFlags.Const,
          ),
        })),
    );
  if (declarations.length !== 1 || !declarations[0].constant)
    throw new Error("expected one direct exported const QUIRE_CONTRACT");
  function object(node) {
    while (
      node &&
      (ts.isAsExpression(node) || ts.isParenthesizedExpression(node))
    )
      node = node.expression;
    if (!node || !ts.isObjectLiteralExpression(node))
      throw new Error("QUIRE_CONTRACT requires literal objects");
    const properties = new Map();
    for (const property of node.properties) {
      if (
        !ts.isPropertyAssignment(property) ||
        !(ts.isIdentifier(property.name) || ts.isStringLiteral(property.name))
      )
        throw new Error("QUIRE_CONTRACT has an ambiguous property");
      const name = property.name.text;
      if (properties.has(name))
        throw new Error(`duplicate QUIRE_CONTRACT property ${name}`);
      properties.set(name, property.initializer);
    }
    return properties;
  }
  const properties = object(declarations[0].declaration.initializer);
  const revision = properties.get("sourceRevision");
  if (
    !revision ||
    !ts.isStringLiteral(revision) ||
    !/^[0-9a-f]{40}$/.test(revision.text)
  )
    throw new Error("vendored contract has no exact literal sourceRevision");
  const hashes = object(properties.get("hashes"));
  const result = {};
  for (const name of SCHEMAS) {
    const value = hashes.get(name);
    if (
      !value ||
      !ts.isStringLiteral(value) ||
      !/^[0-9a-f]{64}$/.test(value.text)
    )
      throw new Error(`vendored schema has no exact literal hash: ${name}`);
    result[name] = value.text;
  }
  return { revision: revision.text, hashes: result };
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
      "src/quire/contract.ts",
    ).toString(),
  );
  assertRemoteRevision("vendored Quire contract", engine, revision);
  for (const name of SCHEMAS) {
    const bytes = committedBytes(
      quoin,
      candidate.repositories.quoin.revision,
      `src/quire/schemas/${name}`,
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
  committedBytes(
    roots["qa-corpus"],
    candidate.repositories["qa-corpus"].revision,
    "bounds.py",
  );
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
