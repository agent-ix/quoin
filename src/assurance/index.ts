/**
 * The assurance-case view (FR-040).
 *
 * ADR-0011 L1: read-only over the evidence store and the spec. It collects
 * nothing and writes nothing.
 */

export {
  buildCase,
  requirementOf,
  type AssuranceCase,
  type CaseInput,
  type CaseNode,
  type NodeKind,
  type NodeStatus,
} from "./graph.js";
export { renderCase } from "./render.js";
export {
  buildAuthoredArgumentView,
  parseAssuranceArgument,
  renderAuthoredArgument,
  type AssuranceArgumentDefinition,
  type AssumptionView,
  type AuthoredArgumentView,
  type BuildAuthoredArgumentRequest,
  type ChallengeView,
  type CriterionView,
  type ReasoningView,
  type SufficiencyDecision,
} from "./argument.js";
export {
  buildDischargeReport,
  renderDischargeReport,
  type BuildDischargeRequest,
  type ClauseDischarge,
  type DirectDischargeFact,
  type DischargeAttestation,
  type DischargeFact,
  type DischargeReport,
  type DischargeState,
  type DispositionFact,
  type UnusedDischargeFact,
} from "./discharge.js";
