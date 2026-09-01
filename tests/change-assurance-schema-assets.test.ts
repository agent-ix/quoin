/**
 * Normative JSON Schema and package-asset checks for Quoin #282.
 *
 * Trace: FR-063-AC-1
 * Trace: FR-064-AC-1
 * Trace: FR-065-AC-1
 * TC-1261, TC-1272, TC-1281
 */

import { execFileSync } from "node:child_process";
import { readFileSync } from "node:fs";
import { basename, join } from "node:path";

import {
  CHANGE_ASSURANCE_SCHEMA_NAMES,
  changeAssuranceSchemaPath,
  readChangeAssuranceSchema,
} from "../src/change-assurance/index.js";
import {
  CHANGE_ASSURANCE_SCHEMA_NAMES as EVIDENCE_SCHEMA_NAMES,
  readChangeAssuranceSchema as readEvidenceSchema,
} from "../src/evidence/index.js";

const SPEC_FILES = {
  "change-assurance-record-v1.schema.json":
    "spec/functional/FR-063-change-assurance-record-integrity.md",
  "proof-attestation-v1.schema.json":
    "spec/functional/FR-064-proof-attestation.md",
  "verification-receipt-v1.schema.json":
    "spec/functional/FR-065-change-assurance-verification.md",
} as const;

function normativeSchema(path: string): unknown {
  const text = readFileSync(path, "utf8");
  const match = /```json\n([\s\S]*?)\n```/.exec(text);
  if (!match) throw new Error(`no normative JSON Schema block in ${path}`);
  return JSON.parse(match[1]);
}

describe("change-assurance JSON Schema assets", () => {
  it("TC-1261/TC-1272/TC-1281 exactly match the normative specification blocks", () => {
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

  it("exposes the same normative assets through the retained-evidence seam", () => {
    expect(EVIDENCE_SCHEMA_NAMES).toEqual(CHANGE_ASSURANCE_SCHEMA_NAMES);
    for (const name of CHANGE_ASSURANCE_SCHEMA_NAMES) {
      expect(readEvidenceSchema(name)).toEqual(readChangeAssuranceSchema(name));
    }
  });

  it("copies every normative schema into the published dist asset directory", () => {
    execFileSync("node", ["scripts/copy-quire-schemas.mjs"]);
    for (const name of CHANGE_ASSURANCE_SCHEMA_NAMES) {
      expect(readFileSync(join("dist", "schemas", name))).toEqual(
        readFileSync(changeAssuranceSchemaPath(name)),
      );
    }
  });
});
