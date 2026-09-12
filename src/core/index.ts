/**
 * The quoin↔quoin-core boundary (quoin#373 FR-096).
 *
 * One import site for the subprocess contract. Nothing else lives here yet:
 * Stage 0 ports no domain logic, and `src/core/types.ts` — the schema-sourced
 * response types — is generated from `rust/crates/quoin-schemas`, not written
 * by hand (FR-097).
 */

export {
  CORE_EXIT,
  QUOIN_CORE_MAX_BUFFER,
  carriesPayload,
  coreVersion,
  quoinCoreExecutable,
  runCore,
  runCoreAllowFailure,
  type CoreDiagnostic,
  type CoreResult,
} from "./exec.js";
