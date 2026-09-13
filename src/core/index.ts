/**
 * The quoin↔quoin-core boundary (quoin#373 FR-096).
 *
 * One import site for the subprocess contract. `./types.js` is GENERATED from
 * the Rust boundary types by `make types` and re-exported here (FR-097); it is
 * never hand-written, and `quoin-schemas` fails `make rust-gate` if the file
 * and the Rust types disagree.
 */

export {
  CORE_TYPES_PROVENANCE,
  PROTOCOL_VERSION,
  type Diagnostic,
  type PingPayload,
  type PingRequest,
} from "./types.js";

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
