/** Read-only trace graph analyses (FR-045). */

export {
  analyzeChangeImpact,
  analyzeChurn,
  analyzeFanOut,
  buildTraceGraph,
  type ChangeImpactAnalysis,
  type ChurnAnalysis,
  type ChurnEvent,
  type ChurnRow,
  type DocumentEdge,
  type FanOutAnalysis,
  type FanOutRow,
  type GraphLimitation,
  type GraphLimitationKind,
  type ImplementationNode,
  type ObligationNode,
  type ObligationSuiteEdge,
  type TraceGraph,
  type TraceGraphInput,
} from "./analysis.js";
export { renderGraphAnalysis, type GraphAnalysis } from "./render.js";
