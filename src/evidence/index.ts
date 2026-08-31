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
  latestMockInspection,
  latestMockInspections,
  latestRun,
  latestRuns,
  latestScans,
  listRecordedSuites,
  listMockInspections,
  listRuns,
  readBaseline,
  readBindings,
  readMockInspection,
  readMockInspections,
  readRun,
  readRuns,
  runPath,
  mockInspectionPath,
  short,
  StoreReadError,
  storeRoot,
  suitesPath,
  writeBaseline,
  writeBindings,
  writeMockInspection,
  writeRun,
} from "./store.js";

export {
  MOCK_INSPECTIONS_DIR,
  MUTATION_SCORE_METRIC,
  RUNS_DIR,
  STORE_SCHEMA_VERSION,
  type Affirmation,
  type BaselineFile,
  type Binding,
  type BindingsFile,
  type MockInjection,
  type MockInspectionRecord,
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
export {
  inspectMockInjections,
  mockInspectionInput,
} from "./mock-inspection.js";

// Change-assurance extends the FR-030 retained-evidence boundary. Re-export its
// versioned schemas here as PLAN-005's evidence-facing public seam.
export {
  CHANGE_ASSURANCE_SCHEMA_NAMES,
  changeAssuranceSchemaPath,
  readChangeAssuranceSchema,
  type ChangeAssuranceSchemaName,
} from "../change-assurance/schema-assets.js";
