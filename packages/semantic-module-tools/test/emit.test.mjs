import assert from "node:assert/strict";
import {
  existsSync,
  mkdirSync,
  mkdtempSync,
  rmSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test from "node:test";
import { main } from "../src/cli.mjs";
import { semanticCoreBase } from "../src/emit.mjs";
import {
  GenerateError,
  checkOutput,
  writeOutput,
  absolutizeSchemas,
  filterModuleSchemas,
  importedBases,
} from "../src/index.mjs";

const BASE = "https://schemas.agent-ix.org/agent-ix/spec-x/";
const CORE = "https://schemas.agent-ix.org/semantic-core/0.4.3/";
const IMPORT = "agent-ix/spec-y";

test("filterModuleSchemas keeps only schemas under this module's base", () => {
  const all = [
    ["A.json", { $id: `${BASE}A.json` }],
    ["FieldDecl.json", { $id: "FieldDecl.json" }],
    ["B.json", { $id: "https://schemas.agent-ix.org/agent-ix/spec-y/B.json" }],
    ["C.json", {}],
  ];
  assert.deepEqual(
    filterModuleSchemas(all, BASE).map(([name]) => name),
    ["A.json"],
  );
});

test("absolutizeSchemas rewrites module, core and imported references", () => {
  const schema = {
    $id: "A.json",
    properties: {
      own: { $ref: "B.json" },
      core: { $ref: "FieldDecl.json" },
      imported: { $ref: `${IMPORT}/Thing.json` },
      done: { $ref: "https://example.org/x.json" },
    },
    allOf: [{ $ref: "B.json" }],
  };
  const schemas = [["A.json", schema]];
  absolutizeSchemas(
    schemas,
    BASE,
    new Set(["A.json", "B.json"]),
    importedBases(`semantic:\n  imports:\n    ${IMPORT}: 1.2.3\n`),
    CORE,
  );
  assert.equal(schema.$id, `${BASE}A.json`);
  assert.equal(schema.properties.own.$ref, `${BASE}B.json`);
  assert.equal(schema.properties.core.$ref, `${CORE}FieldDecl.json`);
  assert.equal(
    schema.properties.imported.$ref,
    "https://schemas.agent-ix.org/agent-ix/spec-y/Thing.json",
  );
  assert.equal(schema.properties.done.$ref, "https://example.org/x.json");
  assert.equal(schema.allOf[0].$ref, `${BASE}B.json`);
});

test("absolutizeSchemas fails naming a reference that matches no declared base", () => {
  const schemas = [["A.json", { $id: "A.json", $ref: "other/mod/Z.json" }]];
  assert.throws(
    () =>
      absolutizeSchemas(schemas, BASE, new Set(["A.json"]), new Map(), CORE),
    (error) =>
      error instanceof GenerateError &&
      error.message.includes('"other/mod/Z.json"'),
  );
});

test("importedBases reads nothing from an empty imports block", () => {
  assert.equal(importedBases("semantic:\n  imports: {}\n").size, 0);
  assert.equal(importedBases("semantic:\n  targets: []\n").size, 0);
});

test("semanticCoreBase uses an explicit version without reading node_modules", () => {
  const repo = mkdtempSync(join(tmpdir(), "smt-core-"));
  try {
    assert.equal(
      semanticCoreBase(repo, "9.8.7"),
      "https://schemas.agent-ix.org/semantic-core/9.8.7/",
    );
    assert.throws(
      () => semanticCoreBase(repo, undefined),
      (error) =>
        error instanceof GenerateError &&
        error.message.includes("not installed"),
    );
    const dir = join(repo, "node_modules/@agent-ix/semantic-core");
    mkdirSync(dir, { recursive: true });
    writeFileSync(
      join(dir, "package.json"),
      JSON.stringify({ version: "1.2.3" }),
    );
    assert.equal(
      semanticCoreBase(repo, undefined),
      "https://schemas.agent-ix.org/semantic-core/1.2.3/",
    );
  } finally {
    rmSync(repo, { recursive: true, force: true });
  }
});

test("checkOutput reports missing, differing and stale files; writeOutput removes stale ones", () => {
  const repo = mkdtempSync(join(tmpdir(), "smt-out-"));
  const out = join(repo, "schemas");
  try {
    const rendered = new Map([
      ["A.json", "{}\n"],
      ["B.json", "{}\n"],
    ]);
    assert.ok(
      checkOutput(rendered, out, repo).some((p) => p.includes("missing")),
    );
    mkdirSync(out);
    writeFileSync(join(out, "A.json"), "{ }\n");
    writeFileSync(join(out, "Old.json"), "{}\n");
    writeFileSync(join(out, "notes.txt"), "keep\n");
    const problems = checkOutput(rendered, out, repo);
    assert.ok(problems.includes(join("schemas", "A.json")));
    assert.ok(problems.includes(join("schemas", "B.json")));
    assert.ok(problems.includes(`${join("schemas", "Old.json")} (stale)`));
    writeOutput(rendered, out);
    assert.deepEqual(checkOutput(rendered, out, repo), []);
    assert.equal(existsSync(join(out, "Old.json")), false);
    assert.equal(existsSync(join(out, "notes.txt")), true);
  } finally {
    rmSync(repo, { recursive: true, force: true });
  }
});

function runCli(args) {
  const errors = [];
  const original = console.error;
  console.error = (message) => errors.push(String(message));
  try {
    return { code: main(args), stderr: errors.join("\n") };
  } finally {
    console.error = original;
  }
}

test("the CLI exits 2 without --package-dir and on an unknown option", () => {
  const missing = runCli([]);
  assert.equal(missing.code, 2);
  assert.match(missing.stderr, /--package-dir is required/);
  assert.equal(runCli(["--bogus"]).code, 2);
});

test("the CLI exits 1 naming package.json when the repo root has none", () => {
  const repo = mkdtempSync(join(tmpdir(), "smt-cli-"));
  try {
    mkdirSync(join(repo, "typespec"));
    writeFileSync(join(repo, "manifest.yaml"), "semantic:\n  imports: {}\n");
    const result = runCli(["--repo-root", repo, "--package-dir", "."]);
    assert.equal(result.code, 1);
    assert.match(result.stderr, /package\.json/);
    assert.match(result.stderr, /--repo-root/);
  } finally {
    rmSync(repo, { recursive: true, force: true });
  }
});
