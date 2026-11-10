/**
 * Emit a semantic module's JSON Schemas from `typespec/main.tsp`.
 *
 * Runs the official `@typespec/json-schema` emitter through `tsp compile`, keeps
 * only this module's namespace, rewrites any `$id` or `$ref` the emitter left
 * relative to an absolute URL under a DECLARED base, and renders the result
 * deterministically.
 *
 * The official emitter only: a wrong schema is fixed in `typespec/main.tsp` and
 * regenerated, never hand-edited. This is a DRIVER, not an emitter.
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
import { join, relative, resolve } from "node:path";

export const MIN_NODE_MAJOR = 20;
export const SCHEMA_HOST = "https://schemas.agent-ix.org";

export class GenerateError extends Error {}

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
 * The semantic-core base this module extends: the version of the installed
 * `@agent-ix/semantic-core` package unless one is given explicitly.
 */
export function semanticCoreBase(repoRoot, version) {
  if (version) return `${SCHEMA_HOST}/semantic-core/${version}/`;
  const pkg = resolve(
    repoRoot,
    "node_modules/@agent-ix/semantic-core/package.json",
  );
  if (!existsSync(pkg))
    fail("@agent-ix/semantic-core is not installed; run `make install`.");
  const installed = JSON.parse(readFileSync(pkg, "utf8")).version;
  return `${SCHEMA_HOST}/semantic-core/${installed}/`;
}

/**
 * One base per imported semantic module, read from the manifest's
 * `semantic.imports` block.
 */
export function importedBases(manifestText) {
  const block = manifestText.match(
    /^\s{2}imports:\s*(\{\s*\}|\n(?:\s{4}\S.*\n?)*)/m,
  );
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
function moduleBase(sourceDir, identity) {
  const source = readFileSync(resolve(sourceDir, "main.tsp"), "utf8");
  const declared = source.match(/@jsonSchema\("([^"]+)"\)/)?.[1];
  if (!declared) fail("typespec/main.tsp declares no @jsonSchema base");
  const expected = `${SCHEMA_HOST}/${identity}/`;
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

function compile(repoRoot, sourceDir, scratch) {
  const cli = resolve(
    repoRoot,
    "node_modules/@typespec/compiler/entrypoints/cli.js",
  );
  if (!existsSync(cli)) {
    fail(
      "@typespec/compiler is not installed; run `make install` before `make schemas`.",
    );
  }
  try {
    execFileSync(
      process.execPath,
      [cli, "compile", sourceDir, "--output-dir", scratch],
      {
        cwd: repoRoot,
        stdio: "pipe",
        encoding: "utf8",
      },
    );
  } catch (error) {
    const detail = [error.stdout, error.stderr]
      .filter(Boolean)
      .join("\n")
      .trim();
    fail(
      `tsp compile failed; the committed output was not touched.\n${detail}`,
    );
  }
}

/**
 * This module's namespace only: the emitter re-emits every imported library's
 * models beside them and those ship in their own packages.
 * `all` is a list of `[name, schema]`.
 */
export function filterModuleSchemas(all, base) {
  return all.filter(
    ([, schema]) =>
      typeof schema.$id === "string" && schema.$id.startsWith(base),
  );
}

/**
 * Rewrite a relative `$id`/`$ref` to an absolute one under a declared base: a
 * file this module emits resolves under the module base, a semantic-core model
 * under the semantic-core base, and an imported module's model under that
 * module's base at its exact version. Anything else fails.
 * `schemas` is a list of `[name, schema]`, mutated in place.
 */
export function absolutizeSchemas(
  schemas,
  base,
  moduleFiles,
  imports,
  coreBase,
) {
  const problems = [];
  const absolutize = (name, value) => {
    if (typeof value !== "string" || /^https?:\/\//.test(value)) return value;
    if (moduleFiles.has(value)) return `${base}${value}`;
    const qualified = value.match(/^([^/]+\/[^/]+)\/(.+)$/);
    if (qualified && imports.has(qualified[1])) {
      return `${imports.get(qualified[1])}${qualified[2]}`;
    }
    if (/^[A-Za-z][A-Za-z0-9]*\.json$/.test(value))
      return `${coreBase}${value}`;
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

export function render(schema) {
  return `${JSON.stringify(schema, null, 2)}\n`;
}

/**
 * Compile and normalize. Returns a Map of file name to rendered text.
 *
 * Options: `repoRoot` (holds node_modules), `sourceDir` (holds main.tsp),
 * `manifestPath`, `identity` (`org/repo`), `semanticCoreVersion` (optional;
 * defaults to the installed package's version).
 */
export function emit({
  repoRoot,
  sourceDir,
  manifestPath,
  identity,
  semanticCoreVersion,
}) {
  requireNode();
  const base = moduleBase(sourceDir, identity);
  const imports = importedBases(readFileSync(manifestPath, "utf8"));
  const scratch = mkdtempSync(join(tmpdir(), "semantic-module-emit-"));
  try {
    compile(repoRoot, sourceDir, scratch);
    const all = readdirSync(scratch)
      .filter((name) => name.endsWith(".json"))
      .sort()
      .map((name) => [
        name,
        JSON.parse(readFileSync(join(scratch, name), "utf8")),
      ]);
    const mine = filterModuleSchemas(all, base);
    if (mine.length === 0) {
      fail(
        `tsp compile emitted no schema under ${base}; the committed output was not touched.`,
      );
    }
    const moduleFiles = new Set(mine.map(([name]) => name));
    absolutizeSchemas(
      mine,
      base,
      moduleFiles,
      imports,
      semanticCoreBase(repoRoot, semanticCoreVersion),
    );
    return new Map(mine.map(([name, schema]) => [name, render(schema)]));
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

/** Differences between `rendered` and the committed `outputDir`, as a list of messages. */
export function checkOutput(rendered, outputDir, repoRoot) {
  const problems = [];
  for (const [name, text] of rendered) {
    const path = join(outputDir, name);
    if (readIfPresent(path) !== text) problems.push(relative(repoRoot, path));
  }
  let committed = [];
  try {
    committed = readdirSync(outputDir).filter((name) => name.endsWith(".json"));
  } catch {
    problems.push(
      `${relative(repoRoot, outputDir)} (missing; run \`make schemas\`)`,
    );
  }
  for (const name of committed) {
    if (!rendered.has(name)) {
      problems.push(`${relative(repoRoot, join(outputDir, name))} (stale)`);
    }
  }
  return problems;
}

export function writeOutput(rendered, outputDir) {
  mkdirSync(outputDir, { recursive: true });
  for (const name of readdirSync(outputDir)) {
    if (name.endsWith(".json") && !rendered.has(name)) {
      rmSync(join(outputDir, name));
    }
  }
  for (const [name, text] of rendered)
    writeFileSync(join(outputDir, name), text);
}
