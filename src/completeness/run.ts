/**
 * Assessing one bundle against every declared vocabulary (FR-037).
 *
 * One function so the command is a thin shell over it, and so the criteria
 * stated over `quoin completeness` and the unit assertions exercise the same
 * path rather than two that agree by inspection.
 */

import {
  assessVocabulary,
  verdictFor,
  type CompletenessFinding,
  type CompletenessReport,
  type VocabularyRollup,
} from "./assess.js";
import { readBundleClaims } from "./bundle.js";
import {
  loadVocabularyCoverage,
  type VocabularyDeclaration,
} from "./declarations.js";

export interface AssessOptions {
  /** Bundle root — the directory whose documents are read. */
  bundleRoot: string;
  /** Promote an admitted gap to a failing verdict. */
  strict?: boolean;
  /** Module roots to read declarations from; defaults to the installed set. */
  moduleRoots?: string[];
}

export interface BundleAssessment extends CompletenessReport {
  bundleRoot: string;
  /** Declarations found, so a report of zero findings can be told from zero checks. */
  vocabularies: string[];
  /** Declarations whose vocabulary could not be resolved. */
  unresolved: Array<{ name: string; reason: string }>;
  /** Documents whose frontmatter could not be parsed. */
  unreadable: Array<{ path: string; reason: string }>;
}

/**
 * Assess a bundle and return the report the command prints.
 *
 * **Zero declarations is reported, not passed.** A repository whose module set
 * declares no vocabulary coverage has not been checked, and printing `PASS`
 * over it would be the "green matrix over dead links" result this program was
 * created to stop. `vocabularies: []` makes the difference visible in both the
 * human and JSON output.
 */
export function assessBundle(options: AssessOptions): BundleAssessment {
  const { declarations, unresolved } = loadVocabularyCoverage(
    options.moduleRoots,
  );
  const findings: CompletenessFinding[] = [];
  const rollups: VocabularyRollup[] = [];
  const unreadable: Array<{ path: string; reason: string }> = [];
  const seen = new Set<string>();

  for (const declaration of declarations as VocabularyDeclaration[]) {
    const read = readBundleClaims(options.bundleRoot, declaration);
    for (const entry of read.unreadable) {
      // The same document is unreadable once per declaration; report it once.
      if (seen.has(entry.path)) continue;
      seen.add(entry.path);
      unreadable.push(entry);
    }
    const assessed = assessVocabulary(declaration, read.documents);
    rollups.push(assessed.rollup);
    findings.push(...assessed.findings);
  }

  findings.sort(
    (a, b) =>
      a.vocabulary.localeCompare(b.vocabulary) ||
      a.kind.localeCompare(b.kind) ||
      a.value.localeCompare(b.value),
  );

  return {
    bundleRoot: options.bundleRoot,
    vocabularies: declarations.map((d) => d.name),
    unresolved,
    unreadable,
    rollups,
    findings,
    verdict: verdictFor(findings, options.strict ?? false, declarations.length),
  };
}
