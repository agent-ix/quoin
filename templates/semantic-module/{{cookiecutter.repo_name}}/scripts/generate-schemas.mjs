#!/usr/bin/env node
/**
 * Emit this module's JSON Schemas from `typespec/main.tsp`.
 *
 * Runs the official `@typespec/json-schema` emitter through `tsp compile`, keeps
 * only this module's namespace, rewrites any `$id` or `$ref` the emitter left
 * relative to an absolute URL under a DECLARED base, writes
 * `<package>/schemas/` plus `toolchain.json`, and rewrites `manifest.yaml`'s
 * `data_schema.digest` values textually so the file's comments and YAML anchors
 * survive.
 *
 *   node scripts/generate-schemas.mjs            # regenerate
 *   node scripts/generate-schemas.mjs --check    # write nothing; fail on any difference
 *
 * The official emitter only: a wrong schema is fixed in `typespec/main.tsp` and
 * regenerated, never hand-edited here. This file is a DRIVER, not an emitter —
 * it is byte-identical in every repository the semantic-module template renders,
 * so a correction is one edit rather than a sweep.
 *
 * Node built-ins only, zero dependencies.
 */
import { execFileSync } from "node:child_process";
import { createHash } from "node:crypto";
import {
  existsSync,
  mkdirSync,
  mkdtempSync,
  readdirSync,
  readFileSync,
  rmSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join, relative, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const MIN_NODE_MAJOR = 20;
const SCHEMA_HOST = "https://schemas.agent-ix.org";
const SEMANTIC_CORE_VERSION = "{{ cookiecutter.semantic_core_version }}";
const SEMANTIC_CORE_BASE = `${SCHEMA_HOST}/semantic-core/${SEMANTIC_CORE_VERSION}/`;
const PACKAGE_IDENTITY = "{{ cookiecutter.org }}/{{ cookiecutter.repo_name }}";
const NORMALIZATION = {
  name: "absolute-id-and-ref",
  version: "1.0.0",
  issue: "https://github.com/agent-ix/quoin/issues/307",
};

const repoRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const sourceDir = resolve(repoRoot, "typespec");
const packageDir = resolve(repoRoot, "{{ cookiecutter.package_name }}");
const outputDir = resolve(packageDir, "schemas");
const toolchainPath = resolve(outputDir, "toolchain.json");
const manifestPath = resolve(packageDir, "manifest.yaml");

class GenerateError extends Error {}

function fail(message) {
  throw new GenerateError(message);
}

function requireNode() {
  const major = Number(process.versions.node.split(".")[0]);
  if (!Number.isFinite(major) || major < MIN_NODE_MAJOR) {
    fail(
      `Node ${MIN_NODE_MAJOR} or later is required by @typespec/compiler; this is Node ${process.versions.node}. ` +
        "Install it and re-run `make schemas`.",
    );
  }
}

function dependencyVersion(name) {
  const path = resolve(repoRoot, "node_modules", name, "package.json");
  if (!existsSync(path)) {
    fail(
      `${name} is not installed. The schema toolchain is a hard requirement of ` +
        "this command, not an optional extra: without it nothing is emitted and " +
        "a check that reported success would be reporting on bytes it never " +
        "produced. Run `make install` (or `npm install`) and re-run.",
    );
  }
  return JSON.parse(readFileSync(path, "utf8")).version;
}

/** The manifest `version`, read without a YAML parser so the file is never reserialized. */
function manifestVersion() {
  const match = readFileSync(manifestPath, "utf8").match(
    /^version:\s*["']?([^"'\s]+)["']?\s*$/m,
  );
  if (!match) fail(`${relative(repoRoot, manifestPath)} declares no top-level version`);
  return match[1];
}

/**
 * Every base a `$ref` may resolve against: this module's, semantic-core's, and
 * one per imported semantic module at the exact version the manifest records.
 * A reference matching none of them is a failure, not a guess.
 */
function importedBases() {
  const text = readFileSync(manifestPath, "utf8");
  const block = text.match(/^\s{2}imports:\s*(\{\s*\}|\n(?:\s{4}\S.*\n?)*)/m);
  const bases = new Map();
  if (!block || block[1].trim() === "{}") return bases;
  for (const line of block[1].split("\n")) {
    const entry = line.match(/^\s{4}([^:]+):\s*(\S+)\s*$/);
    if (!entry) continue;
    const identity = entry[1].trim().replace(/^["']|["']$/g, "");
    const version = entry[2].trim().replace(/^["']|["']$/g, "");
    bases.set(identity, `${SCHEMA_HOST}/${identity}/${version}/`);
  }
  return bases;
}

/** The `@jsonSchema` base declared by the source, checked against the manifest version. */
function moduleBase() {
  const source = readFileSync(resolve(sourceDir, "main.tsp"), "utf8");
  const declared = source.match(/@jsonSchema\("([^"]+)"\)/)?.[1];
  if (!declared) fail("typespec/main.tsp declares no @jsonSchema base");
  const version = manifestVersion();
  const expected = `${SCHEMA_HOST}/${PACKAGE_IDENTITY}/${version}/`;
  if (declared !== expected) {
    fail(
      "@jsonSchema base version and manifest version disagree:\n" +
        `  typespec/main.tsp: ${declared}\n` +
        `  manifest.yaml version: ${version} (expected base ${expected})\n` +
        "A version bump edits both in one commit.",
    );
  }
  return declared;
}

function compile(scratch) {
  const cli = resolve(repoRoot, "node_modules/@typespec/compiler/entrypoints/cli.js");
  if (!existsSync(cli)) {
    fail(
      "@typespec/compiler is not installed; run `make install` before `make schemas`.",
    );
  }
  try {
    execFileSync(process.execPath, [cli, "compile", sourceDir, "--output-dir", scratch], {
      cwd: repoRoot,
      stdio: "pipe",
      encoding: "utf8",
    });
  } catch (error) {
    const detail = [error.stdout, error.stderr].filter(Boolean).join("\n").trim();
    fail(`tsp compile failed; the committed output was not touched.\n${detail}`);
  }
}

/**
 * Rewrite a relative `$id`/`$ref` to an absolute one under a declared base: a
 * file this module emits resolves under the module base, a semantic-core model
 * under the semantic-core base, and an imported module's model under that
 * module's base at its exact version. Anything else fails.
 */
function normalize(schemas, base, moduleFiles, imports) {
  const rewritten = new Set();
  const problems = [];
  const absolutize = (name, value) => {
    if (typeof value !== "string" || /^https?:\/\//.test(value)) return value;
    rewritten.add(name);
    if (moduleFiles.has(value)) return `${base}${value}`;
    const qualified = value.match(/^([^/]+\/[^/]+)\/(.+)$/);
    if (qualified && imports.has(qualified[1])) {
      return `${imports.get(qualified[1])}${qualified[2]}`;
    }
    if (/^[A-Za-z][A-Za-z0-9]*\.json$/.test(value)) return `${SEMANTIC_CORE_BASE}${value}`;
    problems.push(
      `${name}: reference ${JSON.stringify(value)} matches no declared base ` +
        `(this module, semantic-core ${SEMANTIC_CORE_VERSION}, or an entry of semantic.imports)`,
    );
    return value;
  };
  const walk = (name, node) => {
    if (Array.isArray(node)) {
      for (const item of node) walk(name, item);
      return;
    }
    if (!node || typeof node !== "object") return;
    for (const key of ["$id", "$ref"]) {
      if (key in node) node[key] = absolutize(name, node[key]);
    }
    for (const [key, value] of Object.entries(node)) {
      if (key !== "$id" && key !== "$ref") walk(name, value);
    }
  };
  for (const [name, schema] of schemas) walk(name, schema);
  if (problems.length > 0) fail(problems.join("\n"));
  return [...rewritten].sort();
}

function render(schema) {
  return `${JSON.stringify(schema, null, 2)}\n`;
}

function emit() {
  requireNode();
  const base = moduleBase();
  const imports = importedBases();
  const scratch = mkdtempSync(join(tmpdir(), "semantic-module-emit-"));
  try {
    compile(scratch);
    const all = readdirSync(scratch)
      .filter((name) => name.endsWith(".json"))
      .sort()
      .map((name) => [name, JSON.parse(readFileSync(join(scratch, name), "utf8"))]);
    // This module's namespace only; the emitter re-emits every imported
    // library's models beside them and those ship in their own packages.
    const mine = all.filter(
      ([, schema]) => typeof schema.$id === "string" && schema.$id.startsWith(base),
    );
    if (mine.length === 0) {
      fail(
        `tsp compile emitted no schema under ${base}; the committed output was not touched.`,
      );
    }
    const moduleFiles = new Set(mine.map(([name]) => name));
    const rewrittenFiles = normalize(mine, base, moduleFiles, imports);
    const rendered = new Map(mine.map(([name, schema]) => [name, render(schema)]));

    const digests = new Map(
      [...rendered].map(([name, text]) => [
        name,
        `sha256:${createHash("sha256").update(text, "utf8").digest("hex")}`,
      ]),
    );
    const overall = createHash("sha256");
    for (const [name, text] of rendered) overall.update(`${name}\n${text}`);

    const toolchain = {
      compiler: {
        name: "@typespec/compiler",
        version: dependencyVersion("@typespec/compiler"),
      },
      emitter: {
        name: "@typespec/json-schema",
        version: dependencyVersion("@typespec/json-schema"),
      },
      semanticCore: {
        name: "@agent-ix/semantic-core",
        version: dependencyVersion("@agent-ix/semantic-core"),
      },
      base,
      imports: Object.fromEntries([...imports.entries()].sort()),
      normalization: {
        ...NORMALIZATION,
        applied: rewrittenFiles.length > 0,
        rewrittenFiles,
        note:
          rewrittenFiles.length === 0
            ? "no-op: the emitter left no relative $id or $ref"
            : "rewrote relative $id/$ref to a declared base",
      },
      files: [...rendered.keys()],
      digest: `sha256:${overall.digest("hex")}`,
    };
    return { rendered, toolchain: render(toolchain), digests };
  } finally {
    rmSync(scratch, { recursive: true, force: true });
  }
}

/**
 * Textual digest rewrite: replace the `digest:` line that follows each
 * `schema: schemas/<File>` line. Anchors, aliases and comments survive because
 * the file is never parsed or reserialized.
 */
function manifestWithDigests(digests) {
  const lines = readFileSync(manifestPath, "utf8").split("\n");
  const problems = [];
  const seen = new Set();
  let pending = null;
  const out = lines.map((line) => {
    const schema = line.match(/^(\s*)schema:\s*schemas\/(\S+)\s*$/);
    if (schema) {
      pending = schema[2];
      seen.add(pending);
      return line;
    }
    const digest = line.match(/^(\s*)digest:\s*(\S*)\s*$/);
    if (digest && pending) {
      const expected = digests.get(pending);
      if (!expected) {
        problems.push(`manifest references schemas/${pending}, which is not emitted`);
        pending = null;
        return line;
      }
      pending = null;
      return `${digest[1]}digest: ${expected}`;
    }
    return line;
  });
  // The reverse is deliberately NOT asserted here. The emitter also writes the
  // support models an exported type refers to, and those legitimately carry no
  // manifest reference. What must hold — that every EXPORTED type has one — is a
  // manifest fact, checked by the module's own suite against `semantic.exports`.
  if (problems.length > 0) fail(problems.join("\n"));
  return out.join("\n");
}

function readIfPresent(path) {
  try {
    return readFileSync(path, "utf8");
  } catch {
    return undefined;
  }
}

function check(rendered, toolchain, manifestText) {
  const problems = [];
  for (const [name, text] of rendered) {
    const path = join(outputDir, name);
    if (readIfPresent(path) !== text) problems.push(relative(repoRoot, path));
  }
  let committed = [];
  try {
    committed = readdirSync(outputDir).filter((name) => name.endsWith(".json"));
  } catch {
    problems.push(`${relative(repoRoot, outputDir)} (missing; run \`make schemas\`)`);
  }
  for (const name of committed) {
    if (name !== "toolchain.json" && !rendered.has(name)) {
      problems.push(`${relative(repoRoot, join(outputDir, name))} (stale)`);
    }
  }
  if (readIfPresent(toolchainPath) !== toolchain) {
    problems.push(relative(repoRoot, toolchainPath));
  }
  if (readIfPresent(manifestPath) !== manifestText) {
    problems.push(`${relative(repoRoot, manifestPath)} (data_schema.digest)`);
  }
  return problems;
}

function write(rendered, toolchain, manifestText) {
  mkdirSync(outputDir, { recursive: true });
  for (const name of readdirSync(outputDir)) {
    if (name.endsWith(".json") && name !== "toolchain.json" && !rendered.has(name)) {
      rmSync(join(outputDir, name));
    }
  }
  for (const [name, text] of rendered) writeFileSync(join(outputDir, name), text);
  writeFileSync(toolchainPath, toolchain);
  writeFileSync(manifestPath, manifestText);
}

function main() {
  const checking = process.argv.includes("--check");
  const { rendered, toolchain, digests } = emit();
  const manifestText = manifestWithDigests(digests);
  if (checking) {
    const problems = check(rendered, toolchain, manifestText);
    if (problems.length > 0) {
      console.error(
        `emitted schemas differ from the committed output:\n  ${problems.join("\n  ")}\n` +
          "Run `make schemas` and commit the result.",
      );
      process.exit(1);
    }
    console.log(`schemas-check: ${rendered.size} schema(s) match the committed output`);
    return;
  }
  write(rendered, toolchain, manifestText);
  console.log(
    `schemas: wrote ${rendered.size} schema(s) + toolchain.json to ${relative(repoRoot, outputDir)}`,
  );
}

try {
  main();
} catch (error) {
  if (error instanceof GenerateError) {
    console.error(error.message);
    process.exit(1);
  }
  throw error;
}
