/**
 * Reusable change-assurance integrity contracts (FR-063..FR-065).
 *
 * Digests establish content integrity. Recorded actor labels are attribution;
 * neither is an authentication, authorization, signature, or identity claim.
 */

export {
  IntegrityError,
  assertDigest,
  blake3Hex,
  canonicalBytes,
  canonicalizeJcs,
  digestValue,
  parseStrictJson,
} from "./integrity.js";
export {
  recordBytes,
  hashIxFlowEvent,
  sealChangeRecord,
  validateDecision,
  verifyChangeRecord,
  verifyIxFlowChain,
  verifyLineage,
} from "./records.js";
export {
  attestationBytes,
  sealAttestation,
  verifyAttestation,
} from "./attestations.js";
export {
  CHANGE_ASSURANCE_INTEGRITY_BOUNDARY,
  attestationPath,
  changeAssuranceRoot,
  intakeAttestation,
  readAttestation,
  readChangeRecord,
  recordPath,
  recoverChangeAssuranceStaging,
  writeChangeRecord,
  type IntakeOptions,
} from "./store.js";
export { verifyChangeAssurance, verifyReceipt } from "./verify.js";
export {
  CHANGE_ASSURANCE_SCHEMA_NAMES,
  changeAssuranceSchemaPath,
  readChangeAssuranceSchema,
  type ChangeAssuranceSchemaName,
} from "./schema-assets.js";
export type * from "./types.js";
