/**
 * The quire↔quoin contract surface (FR-029).
 *
 * One import site for everything a consumer needs to read quire JSON safely:
 * the pinned contract, the version premise, the validators, and the models the
 * validators make true.
 */

export {
  QUIRE_CONTRACT,
  checkVersionPremise,
  compareVersions,
  parseCliVersion,
  readSchema,
  schemaHash,
  schemaPath,
  type SchemaName,
  type VersionPremiseFailure,
} from "./contract.js";

export {
  parseCoverage,
  parseProperties,
  validateCoverage,
  validateProperties,
  type ContractViolation,
  type ValidationResult,
} from "./validate.js";

export type {
  AcShape,
  CoverageDiagnostic,
  CoverageReport,
  CoverageTotals,
  Criterion,
  CriteriaCounts,
  CriterionObligation,
  Extraction,
  GroupCounts,
  NoSymbolRow,
  Obligation,
  PropertiesDocument,
  PropertiesReport,
  PropertyShape,
  PropertySpan,
  SharedTraceId,
  SharedTraceSymbol,
  StatusLie,
  UnbackedRow,
  UntrackedSymbol,
  VocabularyValueRecord,
} from "./types.js";

export {
  QUIRE_MAX_BUFFER,
  quireVersion,
  runQuire,
  runQuireAllowFailure,
} from "./exec.js";
