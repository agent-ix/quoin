#!/usr/bin/env node
/**
 * Emit this module's JSON Schemas from `typespec/main.tsp`.
 *
 * Runs the official `@typespec/json-schema` emitter through `tsp compile`, keeps
 * only this module's namespace, rewrites any `$id` or `$ref` the emitter left
 * relative to an absolute URL under a DECLARED base, writes
 * `<package>/schemas/`.
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
const PACKAGE_IDENTITY = "{{ cookiecutter.org }}/{{ cookiecutter.repo_name }}";

const repoRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const sourceDir = resolve(repoRoot, "typespec");
const packageDir = resolve(repoRoot, "{{ cookiecutter.package_name }}");
const outputDir = resolve(packageDir, "schemas");
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

/**
 * The semantic-core version this module extends: the one declared pin, read from
 * `semantic.semantic_core` in the manifest. No second copy of it lives here.
 */
function semanticCoreBase() {
  const text = readFileSync(manifestPath, "utf8");
  const declared = text.match(/^\s{2}semantic_core:\s*(\S+)\s*$/m)?.[1];
  if (!declared) fail("manifest.yaml declares no semantic.semantic_core");
  return `${SCHEMA_HOST}/semantic-core/${declared}/`;
}

/**
 * Every base a `$ref` may resolve against: this module's, semantic-core's, and
 * one per imported semantic module.
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
    bases.set(identity, `${SCHEMA_HOST}/${identity}/`);
  }
  return bases;
}

/** The `@jsonSchema` base declared by the source, checked against the package identity. */
function moduleBase() {
  const source = readFileSync(resolve(sourceDir, "main.tsp"), "utf8");
  const declared = source.match(/@jsonSchema\("([^"]+)"\)/)?.[1];
  if (!declared) fail("typespec/main.tsp declares no @jsonSchema base");
  const expected = `${SCHEMA_HOST}/${PACKAGE_IDENTITY}/`;
  if (declared !== expected) {
    fail(
      "@jsonSchema base does not match the package identity:\n" +
        `  typespec/main.tsp: ${declared}\n` +
        `  expected base: ${expected}\n` +
        "The base carries no version.",
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
function normalize(schemas, base, moduleFiles, imports, coreBase) {
  const problems = [];
  const absolutize = (name, value) => {
    if (typeof value !== "string" || /^https?:\/\//.test(value)) return value;
    if (moduleFiles.has(value)) return `${base}${value}`;
    const qualified = value.match(/^([^/]+\/[^/]+)\/(.+)$/);
    if (qualified && imports.has(qualified[1])) {
      return `${imports.get(qualified[1])}${qualified[2]}`;
    }
    if (/^[A-Za-z][A-Za-z0-9]*\.json$/.test(value)) return `${coreBase}${value}`;
    problems.push(
      `${name}: reference ${JSON.stringify(value)} matches no declared base ` +
        `(this module, ${coreBase}, or an entry of semantic.imports)`,
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
    normalize(mine, base, moduleFiles, imports, semanticCoreBase());
    const rendered = new Map(mine.map(([name, schema]) => [name, render(schema)]));

    return rendered;
  } finally {
    rmSync(scratch, { recursive: true, force: true });
  }
}

function readIfPresent(path) {
  try {
    return readFileSync(path, "utf8");
  } catch {
    return undefined;
  }
}

function check(rendered) {
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
    if (!rendered.has(name)) {
      problems.push(`${relative(repoRoot, join(outputDir, name))} (stale)`);
    }
  }
  return problems;
}

function write(rendered) {
  mkdirSync(outputDir, { recursive: true });
  for (const name of readdirSync(outputDir)) {
    if (name.endsWith(".json") && !rendered.has(name)) {
      rmSync(join(outputDir, name));
    }
  }
  for (const [name, text] of rendered) writeFileSync(join(outputDir, name), text);
}

function main() {
  const checking = process.argv.includes("--check");
  const rendered = emit();
  if (checking) {
    const problems = check(rendered);
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
  write(rendered);
  console.log(
    `schemas: wrote ${rendered.size} schema(s) to ${relative(repoRoot, outputDir)}`,
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
