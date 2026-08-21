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
  latestRuns,
  latestScans,
  measurementPath,
  listMeasurementPaths,
  listRecordedSuites,
  listRuns,
  readBaseline,
  readBindings,
  readRun,
  readRuns,
  readMeasurement,
  runPath,
  short,
  storeRoot,
  suitesPath,
  writeBaseline,
  writeBindings,
  writeRun,
  writeMeasurement,
} from "./store.js";

export {
  compareMeasurementSets,
  measurementKey,
  measurementTrend,
  queryMeasurements,
  renderMeasurementComparisonJson,
  renderMeasurementComparisonMarkdown,
  type MeasurementComparison,
  type MeasurementDelta,
  type MeasurementIncompatibility,
  type MeasurementPolicy,
  type MeasurementQuery,
  type SamplingCountMismatch,
  type MeasurementTrend,
  type MeasurementTrendPoint,
} from "./comparison.js";

export {
  MUTATION_SCORE_METRIC,
  MEASUREMENTS_DIR,
  RUNS_DIR,
  STORE_SCHEMA_VERSION,
  type Affirmation,
  type BaselineFile,
  type Binding,
  type BindingsFile,
  type RunEntry,
  type RunRecord,
  type MeasurementDistribution,
  type MeasurementQuantile,
  type MeasurementRecord,
} from "./types.js";

export {
  MEASUREMENT_RECORD_SCHEMA,
  MeasurementValidationError,
  measurementSchemaPath,
  readMeasurementSchema,
  validateMeasurementRecord,
} from "./measurement.js";

export {
  obligationsFrom,
  recordRun,
  type RecordOutcome,
  type RecordRequest,
} from "./record.js";

export {
  ADAPTERS,
  ADAPTER_NAMES,
  FINDING_ADAPTERS,
  entriesAdapter,
  selectAdapter,
  selectFindingAdapter,
  type FindingAdapter,
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
export {
  latestScan,
  listScans,
  readScan,
  readScans,
  scanIsVacuous,
  scanPath,
  writeScan,
} from "./store.js";
export { parseAuditScript } from "./adapters/audit-script.js";
export { parseSbom, sbomAdapter } from "./adapters/sbom.js";
export { parseAgentEval, agentEvalAdapter } from "./adapters/agent-eval.js";
