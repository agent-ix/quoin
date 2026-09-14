/**
 * Normative JSON Schema and package-asset checks for Quoin #282.
 *
 * Since quoin#503 the bytes live in `rust/crates/quoin-change-assurance/`
 * and are compiled into `quoin-core`. What is measured here is that the three
 * statements of each schema agree: the normative block in the FR document, the
 * asset the engine emits, and the copy `scripts/copy-schemas.mjs` places in the
 * npm package.
 *
 * Trace: FR-063-AC-1
 * Trace: FR-064-AC-1
 * Trace: FR-065-AC-1
 */

import { execFileSync } from "node:child_process";
import { readFileSync } from "node:fs";
import { basename, join } from "node:path";

import {
  CHANGE_ASSURANCE_SCHEMA_NAMES,
  changeAssuranceSchemaPath,
  coreChangeAssuranceSchemaNames,
  readChangeAssuranceSchema,
  readChangeAssuranceSchemaText,
} from "../src/core/change-assurance-schemas.js";

const SPEC_FILES = {
  "change-assurance-record-v1.schema.json":
    "spec/functional/FR-063-change-assurance-record-integrity.md",
  "proof-attestation-v1.schema.json":
    "spec/functional/FR-064-proof-attestation.md",
  "verification-receipt-v1.schema.json":
    "spec/functional/FR-065-change-assurance-verification.md",
} as const;

const ASSET_ROOT = "rust/crates/quoin-change-assurance/schemas";

function normativeSchema(path: string): unknown {
  const text = readFileSync(path, "utf8");
  const match = /```json\n([\s\S]*?)\n```/.exec(text);
  if (!match) throw new Error(`no normative JSON Schema block in ${path}`);
  return JSON.parse(match[1]);
}

describe("change-assurance JSON Schema assets", () => {
  it("/TC-1272/TC-1281 exactly match the normative specification blocks", () => {
    expect(CHANGE_ASSURANCE_SCHEMA_NAMES).toEqual(Object.keys(SPEC_FILES));
    for (const name of CHANGE_ASSURANCE_SCHEMA_NAMES) {
      const schema = readChangeAssuranceSchema(name);
      expect(schema).toEqual(normativeSchema(SPEC_FILES[name]));
      expect((schema as { $id: string }).$id).toBe(
        `https://agent-ix.github.io/quoin/schemas/${name}`,
      );
      expect(basename(changeAssuranceSchemaPath(name))).toBe(name);
    }
  });

  /**
   * The hand-written name list and the engine's vocabulary are the same list.
   *
   * The list is duplicated on purpose — an oclif flag declares its `options` at
   * class-definition time, where no subprocess may be spawned — so this is the
   * assertion that keeps the duplicate honest.
   */
  it("declares the same asset names the engine ships", () => {
    expect(coreChangeAssuranceSchemaNames()).toEqual([
      ...CHANGE_ASSURANCE_SCHEMA_NAMES,
    ]);
  });

  it("emits each asset byte-for-byte as the crate holds it", () => {
    for (const name of CHANGE_ASSURANCE_SCHEMA_NAMES) {
      expect(readChangeAssuranceSchemaText(name)).toBe(
        readFileSync(join(ASSET_ROOT, name), "utf8"),
      );
    }
  });

  it("copies every normative schema into the published dist asset directory", () => {
    execFileSync("node", ["scripts/copy-schemas.mjs"]);
    for (const name of CHANGE_ASSURANCE_SCHEMA_NAMES) {
      expect(readFileSync(join("dist", "schemas", name))).toEqual(
        readFileSync(join(ASSET_ROOT, name)),
      );
    }
  });
});
