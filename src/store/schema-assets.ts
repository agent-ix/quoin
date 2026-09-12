import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

/** The three normative, versioned JSON Schema assets for FR-063..FR-065. */
export const CHANGE_ASSURANCE_SCHEMA_NAMES = [
  "change-assurance-record-v1.schema.json",
  "proof-attestation-v1.schema.json",
  "verification-receipt-v1.schema.json",
] as const;

export type ChangeAssuranceSchemaName =
  (typeof CHANGE_ASSURANCE_SCHEMA_NAMES)[number];

const here = dirname(fileURLToPath(import.meta.url));

/** Absolute path to a source-tree or packaged change-assurance schema. */
export function changeAssuranceSchemaPath(
  name: ChangeAssuranceSchemaName,
): string {
  return join(here, "schemas", name);
}

/** Read one normative change-assurance schema as parsed JSON. */
export function readChangeAssuranceSchema(
  name: ChangeAssuranceSchemaName,
): unknown {
  return JSON.parse(readFileSync(changeAssuranceSchemaPath(name), "utf8"));
}
