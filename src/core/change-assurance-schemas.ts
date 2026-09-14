import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

import { runCore } from "./exec.js";
import type { SchemaAssetPayload } from "./types.js";

/**
 * The three normative, versioned JSON Schema assets for FR-063..FR-065.
 *
 * The bytes live in `rust/crates/quoin-change-assurance/schemas/` and are
 * compiled into `quoin-core` (quoin#503). They used to sit in
 * `src/store/schemas/`, beside a reader that the store cutover deletes — a
 * normative contract whose home disappears with a port stage is a contract
 * nobody owns.
 *
 * This list is a literal rather than a question asked of the engine because
 * `quoin change-assurance schema` declares it as an oclif flag's `options` at
 * class-definition time, where spawning a subprocess is not available. It is
 * not allowed to drift: `tests/change-assurance-schema-assets.test.ts` asserts
 * it equals the vocabulary `change_assurance.schema` reports.
 */
export const CHANGE_ASSURANCE_SCHEMA_NAMES = [
  "change-assurance-record-v1.schema.json",
  "proof-attestation-v1.schema.json",
  "verification-receipt-v1.schema.json",
] as const;

export type ChangeAssuranceSchemaName =
  (typeof CHANGE_ASSURANCE_SCHEMA_NAMES)[number];

const here = dirname(fileURLToPath(import.meta.url));

/**
 * Absolute path to a **packaged** change-assurance schema.
 *
 * Packaged only, and that is the change quoin#503 made. The bytes now live
 * under `rust/`, which the npm package does not ship; `scripts/copy-schemas.mjs`
 * copies them to `dist/schemas/` at build, and the bundler flattens this module
 * into `dist/`, so the join resolves in an installed package and names a file
 * that does not exist in a source checkout.
 *
 * Anything that needs the bytes in both trees asks
 * {@link readChangeAssuranceSchema}, which goes to the engine that has them
 * compiled in.
 */
export function changeAssuranceSchemaPath(
  name: ChangeAssuranceSchemaName,
): string {
  return join(here, "schemas", name);
}

/**
 * The exact text of one normative change-assurance schema.
 *
 * From `quoin-core`, which carries the assets compiled in, so a source run and
 * an installed package answer identically and both answer with the file the
 * sealing code is tested against.
 */
export function readChangeAssuranceSchemaText(
  name: ChangeAssuranceSchemaName,
): string {
  const payload = runCore("change_assurance.schema", {
    name,
  }) as SchemaAssetPayload;
  if (payload.schema === null || payload.schema === undefined) {
    throw new Error(`quoin-core ships no schema asset named ${name}`);
  }
  return payload.schema;
}

/** Read one normative change-assurance schema as parsed JSON. */
export function readChangeAssuranceSchema(
  name: ChangeAssuranceSchemaName,
): unknown {
  return JSON.parse(readChangeAssuranceSchemaText(name));
}

/** Every asset name `quoin-core` ships, as the engine itself reports them. */
export function coreChangeAssuranceSchemaNames(): string[] {
  return (runCore("change_assurance.schema", {}) as SchemaAssetPayload).schemas;
}
