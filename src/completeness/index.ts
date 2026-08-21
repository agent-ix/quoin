/**
 * Declared-vocabulary completeness and its verdict policy (FR-037).
 *
 * The split ADR-0011 names: quire-rs computes coverage over the spec corpus,
 * quoin decides what a gap is worth and whether an excuse was earned.
 */

export {
  assessVocabulary,
  verdictFor,
  writtenReasonFor,
  type CompletenessFinding,
  type CompletenessReport,
  type DocumentClaims,
  type FindingKind,
  type Severity,
  type Verdict,
  type VocabularyRollup,
} from "./assess.js";
export {
  loadVocabularyCoverage,
  type VocabularyDeclaration,
  type VocabularyDeclarations,
} from "./declarations.js";
export {
  claimsFor,
  readBundleClaims,
  readBundleFrontmatter,
  type BundleDocument,
  type BundleRead,
  type BundleReadEvent,
  type BundleReadObserver,
  type FrontmatterRead,
} from "./bundle.js";
export {
  assessBundle,
  type AssessOptions,
  type BundleAssessment,
} from "./run.js";
