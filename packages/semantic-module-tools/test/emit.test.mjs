import assert from "node:assert/strict";
import test from "node:test";
import {
  GenerateError,
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
    () => absolutizeSchemas(schemas, BASE, new Set(["A.json"]), new Map(), CORE),
    (error) => error instanceof GenerateError && error.message.includes('"other/mod/Z.json"'),
  );
});

test("importedBases reads nothing from an empty imports block", () => {
  assert.equal(importedBases("semantic:\n  imports: {}\n").size, 0);
  assert.equal(importedBases("semantic:\n  targets: []\n").size, 0);
});
