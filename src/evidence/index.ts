/**
 * The evidence store — the artifact of record for what actually ran (FR-030).
 *
 * Every quoin verification surface used to recompute its view from scratch and
 * emit a report; nothing persisted what was checked, against which statement
 * hash, by which run. That is the "green matrix over dead links" failure mode,
 * already observed at ecosystem scale: 1,014 trace tags binding to nothing
 * while matrices read as covered (agent-ix/quire-rs#72).
 */

export {
  affirm,
  baselinePath,
  bind,
  bindingsPath,
  canonicalJson,
  gc,
  inspectionsPath,
  latestRun,
  listRecordedSuites,
  listRuns,
  readBaseline,
  readBindings,
  readRun,
  readRuns,
  runPath,
  short,
  storeRoot,
  suitesPath,
  writeBaseline,
  writeBindings,
  writeRun,
} from "./store.js";

export {
  RUNS_DIR,
  STORE_SCHEMA_VERSION,
  type Affirmation,
  type BaselineFile,
  type Binding,
  type BindingsFile,
  type RunEntry,
  type RunRecord,
} from "./types.js";

export {
  obligationsFrom,
  recordRun,
  type RecordOutcome,
  type RecordRequest,
} from "./record.js";

export {
  ADAPTERS,
  ADAPTER_NAMES,
  entriesAdapter,
  selectAdapter,
} from "./adapters/registry.js";
export { junitAdapter, qualifiedName } from "./adapters/junit.js";
export { cargoMutantsAdapter } from "./adapters/cargo-mutants.js";
export {
  AdapterError,
  type AdapterResult,
  type EvidenceAdapter,
} from "./adapters/types.js";
export {
  parseCargoAudit,
  parseSarif,
  type FindingResult,
} from "./adapters/sarif.js";
export type { Finding, FindingRecord } from "./types.js";
