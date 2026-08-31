/** Read-only evidence graph analyses (FR-062). */

export {
  DEFAULT_RELATION_KINDS,
  analyzeChangeImpact,
  analyzeChurn,
  analyzeFanOut,
  type AuditorVerdict,
  type BindingInput,
  type ChangeImpactAnalysis,
  type ChangeImpactRow,
  type ChurnAnalysis,
  type ChurnEvent,
  type ChurnRow,
  type FanOutAnalysis,
  type FanOutRow,
  type GraphAnalysis,
  type GraphAnalysisInput,
  type GraphAnalysisState,
  type GraphGap,
  type GraphGapKind,
  type GraphReportBase,
  type ImpactBinding,
  type ImpactObligation,
  type ImpactPath,
  type ImpactPathEdge,
  type OwnedObligation,
} from "./analysis.js";
export {
  parseAcceptedAssurancePremises,
  parseAuditEnvelope,
  validateAcceptedAssurancePremises,
  validateAuditIdentity,
  type AuditEnvelope,
  type GraphInputResult,
  type GraphInputViolation,
} from "./input.js";
export {
  loadGraphAnalysisInput,
  type GraphLoadFailure,
  type GraphLoadOptions,
  type GraphLoadResult,
} from "./load.js";
export { renderGraphAnalysis, renderGraphAnalysisJson } from "./render.js";
