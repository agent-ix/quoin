/**
 * The catalog-driven test-plan advisor (FR-031).
 *
 * The capability the ADR-0011 mission names: help define the TESTING PLAN for
 * the software being specced, so the evidence store then measures that it
 * happens.
 */

export {
  loadMethodCatalog,
  methodClasses,
  type MethodCatalog,
  type VerificationMethod,
} from "./methods.js";

export {
  advise,
  characteristicsOf,
  mintableCharacteristics,
  type Advice,
  type MatchReason,
  type ObligationEvidence,
  type ObligationFacts,
  type Recommendation,
} from "./advise.js";
