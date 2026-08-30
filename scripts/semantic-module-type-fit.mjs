#!/usr/bin/env node
// SPDX-License-Identifier: AGPL-3.0-or-later
// Retained, read-only default-module audit runner for agent-ix/quoin#288.

import { createHash } from "node:crypto";
import { execFileSync } from "node:child_process";
import { lstatSync, readFileSync, readdirSync } from "node:fs";
import { dirname, join, relative, resolve, sep } from "node:path";
import { fileURLToPath } from "node:url";

import YAML from "yaml";

import {
  buildSemanticAudit,
  createArtifactFiles,
  serializeCanonical,
  verifyArtifactFiles,
  writeArtifactFiles,
} from "./lib/semantic-module-type-fit.mjs";

const scriptPath = fileURLToPath(import.meta.url);
const repoRoot = resolve(dirname(scriptPath), "..");

function parseArgs(argv) {
  const result = {
    output: join(repoRoot, "analysis", "semantic-module-type-fit"),
    reposRoot: resolve(repoRoot, ".."),
    timestamp: process.env.SOURCE_DATE_EPOCH
      ? new Date(Number(process.env.SOURCE_DATE_EPOCH) * 1000).toISOString()
      : new Date().toISOString(),
  };
  for (let index = 0; index < argv.length; index += 1) {
    const flag = argv[index];
    if (!["--output", "--repos-root", "--timestamp"].includes(flag))
      throw new Error(`unknown argument ${flag}`);
    const value = argv[index + 1];
    if (!value) throw new Error(`${flag} requires a value`);
    index += 1;
    if (flag === "--output") result.output = resolve(value);
    if (flag === "--repos-root") result.reposRoot = resolve(value);
    if (flag === "--timestamp")
      result.timestamp = new Date(value).toISOString();
  }
  return result;
}

function git(repo, args, encoding = "utf8") {
  return execFileSync("git", ["-C", repo, ...args], {
    encoding,
    maxBuffer: 128 * 1024 * 1024,
    stdio: ["ignore", "pipe", "pipe"],
  });
}

function sha256(content) {
  return `sha256:${createHash("sha256").update(content).digest("hex")}`;
}

function treeDigest(files) {
  return sha256(
    serializeCanonical(
      files
        .map((file) => ({ path: file.path, digest: sha256(file.content) }))
        .sort((left, right) => left.path.localeCompare(right.path)),
    ),
  );
}

function gitPaths(repo, revision, prefix = "") {
  const args = ["ls-tree", "-r", "--name-only", revision];
  if (prefix) args.push("--", prefix);
  return git(repo, args)
    .split("\n")
    .map((path) => path.trim())
    .filter(Boolean);
}

function gitFile(repo, revision, path) {
  return git(repo, ["show", `${revision}:${path}`], "buffer");
}

function gitTreeFiles(repo, revision, prefix = "", predicate = () => true) {
  return gitPaths(repo, revision, prefix)
    .filter(predicate)
    .map((path) => ({
      path: prefix ? path.slice(prefix.length).replace(/^\//, "") : path,
      content: gitFile(repo, revision, path).toString("utf8"),
    }));
}

function gitTreeFilesRetainingReadErrors(
  repo,
  revision,
  prefix = "",
  predicate = () => true,
) {
  const files = [];
  const errors = [];
  for (const path of gitPaths(repo, revision, prefix).filter(predicate)) {
    const relativePath = prefix
      ? path.slice(prefix.length).replace(/^\//, "")
      : path;
    try {
      files.push({
        path: relativePath,
        content: gitFile(repo, revision, path).toString("utf8"),
      });
    } catch {
      files.push({ path: relativePath, content: null });
      errors.push(`corpus-file:${relativePath}`);
    }
  }
  return { files, errors };
}

function filesystemFiles(root, current = root) {
  const rows = [];
  for (const name of readdirSync(current).sort()) {
    const path = join(current, name);
    const stat = lstatSync(path);
    if (stat.isSymbolicLink()) continue;
    if (stat.isDirectory()) rows.push(...filesystemFiles(root, path));
    if (stat.isFile())
      rows.push({
        path: relative(root, path).split(sep).join("/"),
        content: readFileSync(path),
      });
  }
  return rows;
}

function quireIdentity() {
  const value = execFileSync("quire", ["--version"], {
    encoding: "utf8",
  }).trim();
  const match = value.match(
    /^quire\s+(\S+)\s+\(cli\s+([^,]+),\s+engine\s+([^@)]+)@([^)]+)\)$/,
  );
  if (!match) throw new Error(`cannot parse Quire identity: ${value}`);
  return {
    cliVersion: match[1],
    cliCommit: match[2],
    engineVersion: match[3],
    engineCommit: match[4],
  };
}

function registryByName(userHome) {
  const registryPath = join(userHome, ".ix", "filament", "registry.json");
  const parsed = JSON.parse(readFileSync(registryPath, "utf8"));
  return new Map((parsed.plugins ?? []).map((record) => [record.name, record]));
}

function coreDataCensus(coreRepo, revision) {
  const inventory = JSON.parse(
    git(coreRepo, [
      "show",
      `${revision}:audit/filament-contract-census/inventory.json`,
    ]),
  );
  const missing = JSON.parse(
    git(coreRepo, [
      "show",
      `${revision}:audit/filament-contract-census/missing-contracts.json`,
    ]),
  );
  return {
    revision,
    records: inventory.records ?? [],
    missing: missing.missing ?? [],
  };
}

function main() {
  const args = parseArgs(process.argv.slice(2));
  const userHome = process.env.HOME;
  if (!userHome)
    throw new Error(
      "HOME is required to locate the installed read-only module store",
    );
  const defaultModulesText = readFileSync(
    join(repoRoot, "default-modules.yaml"),
    "utf8",
  );
  const defaultManifest = YAML.parse(defaultModulesText);
  const registry = registryByName(userHome);
  const modules = defaultManifest.entries.map((entry, declarationIndex) => {
    const repositoryName = entry.source.url.split("/").at(-1);
    const sourceRepo = join(args.reposRoot, repositoryName);
    let resolvedSha = null;
    let moduleFiles = [];
    let corpusFiles = [];
    let manifestText = "";
    let sourceContentDigest = null;
    let sourceClean = null;
    const acquisitionErrors = [];
    try {
      resolvedSha = git(sourceRepo, [
        "rev-parse",
        `${entry.source.ref}^{commit}`,
      ]).trim();
    } catch {
      acquisitionErrors.push("resolve-ref");
    }
    try {
      if (!resolvedSha) throw new Error("module ref did not resolve");
      moduleFiles = gitTreeFiles(sourceRepo, resolvedSha, entry.source.path);
      const manifestFile = moduleFiles.find(
        (file) => file.path === "manifest.yaml",
      );
      manifestText = manifestFile?.content ?? "";
      sourceContentDigest = treeDigest(moduleFiles);
      sourceClean = git(sourceRepo, ["status", "--porcelain"]).trim() === "";
    } catch {
      acquisitionErrors.push("module-tree");
    }
    try {
      if (!resolvedSha) throw new Error("module ref did not resolve");
      const corpus = gitTreeFilesRetainingReadErrors(
        sourceRepo,
        resolvedSha,
        "",
        (path) => path.toLowerCase().endsWith(".md"),
      );
      corpusFiles = corpus.files;
      acquisitionErrors.push(...corpus.errors);
    } catch {
      acquisitionErrors.push("corpus-tree");
    }
    const installedRoot = join(
      userHome,
      ".ix",
      "filament",
      "modules",
      entry.name,
    );
    let installedContentDigest = null;
    try {
      installedContentDigest = treeDigest(filesystemFiles(installedRoot));
    } catch {
      acquisitionErrors.push("installed-content");
    }
    const registryRecord = registry.get(entry.name);
    return {
      declarationIndex,
      resolvedSha,
      sourceContentDigest,
      installed: {
        sourcePath: `installed/${entry.name}`,
        sourceCommit: registryRecord?.sha ?? null,
        clean: sourceClean,
        contentDigest: installedContentDigest,
      },
      manifestText,
      moduleFiles,
      corpusFiles,
      excludedMarkdown: {},
      acquisitionErrors,
    };
  });
  const packageJson = JSON.parse(
    readFileSync(join(repoRoot, "package.json"), "utf8"),
  );
  const quireCorpusRepo = join(args.reposRoot, "quire-corpus");
  const coreDataRepo = join(args.reposRoot, "filament-core-data");
  const quireCorpusRevision = git(quireCorpusRepo, [
    "rev-parse",
    "origin/main^{commit}",
  ]).trim();
  const coreDataCensusRevision = git(coreDataRepo, [
    "rev-parse",
    "origin/main^{commit}",
  ]).trim();
  const input = {
    timestamp: args.timestamp,
    quoin: {
      commit: git(repoRoot, ["rev-parse", "HEAD"]).trim(),
      clean: git(repoRoot, ["status", "--porcelain"]).trim() === "",
      version: packageJson.version,
    },
    defaultModulesText,
    tools: { quire: quireIdentity() },
    externalEvidence: { quireCorpusRevision, coreDataCensusRevision },
    modules,
    coreDataCensus: coreDataCensus(coreDataRepo, "origin/main"),
    architecture: {
      revision: git(repoRoot, ["rev-parse", "HEAD"]).trim(),
      planes: ["meta", "definition", "execution-observation", "presentation"],
      authority: "docs/semantic-module-architecture/planes-and-authority.md",
      ownership:
        "docs/semantic-module-architecture/ownership-and-boundaries.md",
      decision: "docs/semantic-module-architecture/decision-ledger.md",
    },
    repositoryBoundaries: [
      ...defaultManifest.entries.map((entry) => entry.name),
      "quoin",
      "quire",
      "filament-core-data",
      "compiler",
      "generated-packages",
      "database",
      "api",
      "cli",
      "ui",
    ],
  };
  const audit = buildSemanticAudit(input);
  const files = createArtifactFiles(audit);
  const verification = verifyArtifactFiles(files);
  writeArtifactFiles(args.output, files);
  process.stdout.write(
    serializeCanonical({
      output: relative(repoRoot, args.output),
      verdict: audit.summary.verdict,
      counts: audit.summary.counts,
      contentIdentity: verification.contentIdentity,
    }),
  );
}

main();
