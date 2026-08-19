/**
 * Verdict policy over declared-vocabulary coverage (FR-037).
 *
 * quire-rs FR-059 answers *"which declared values does no document claim?"*.
 * That is a deterministic fact about the spec, and it lives in the engine.
 *
 * **What it does not answer is whether an exclusion was earned.** The engine
 * accepts a bare list — `quality_attributes_not_applicable: [safety, compliance]`
 * — and every value in it stops being reported. Measured on this repository:
 * 7 findings before, **5 after adding that one line**, with no reason written
 * anywhere. So the cheapest way to make the check quiet is to excuse everything,
 * and nothing notices.
 *
 * That is the policy question ADR-0011 leaves to quoin, and it is what this
 * file decides: an exclusion is a **claim about the product** and must carry a
 * written reason, in the same document, naming the value.
 */

import type { VocabularyDeclaration } from "./declarations.js";

export type FindingKind =
  "unowned" | "unjustified-exclusion" | "undeclared-exclusion";

export type Severity = "medium" | "high";

export interface CompletenessFinding {
  /** Declaration this concerns, e.g. `quality-characteristics`. */
  vocabulary: string;
  /** The vocabulary value, e.g. `safety`. */
  value: string;
  kind: FindingKind;
  severity: Severity;
  message: string;
  /** Document carrying the exclusion, for the two exclusion kinds. */
  document?: string;
}

export interface VocabularyRollup {
  vocabulary: string;
  /** Values in the declared enum. */
  declared: number;
  /** Values some document claims. */
  owned: number;
  /** Values a document excuses, whether or not the excuse is justified. */
  excused: number;
  /** Values neither claimed nor excused. */
  unowned: number;
}

/**
 * `UNCHECKED` is not a fourth flavour of pass.
 *
 * A bundle whose module set declares no vocabulary has not been assessed, and
 * `PASS` over it is the green-matrix-over-dead-links result this program was
 * created to stop — the first draft of this file printed exactly that, and the
 * criterion written to forbid it caught it.
 */
export type Verdict = "PASS" | "CONDITIONAL" | "FAIL" | "UNCHECKED";

export interface CompletenessReport {
  rollups: VocabularyRollup[];
  findings: CompletenessFinding[];
  verdict: Verdict;
}

/** One document's declared-vocabulary frontmatter, as read from the bundle. */
export interface DocumentClaims {
  /** Path, relative to the bundle root. */
  path: string;
  /** Values this document claims via the declaration's `field`. */
  claims: string[];
  /** Values this document excuses via `justified_absence_field`. */
  excuses: string[];
  /** Raw body text, searched for the written reason behind each excuse. */
  body: string;
}

/**
 * Reasons that are not reasons.
 *
 * A rationale cell reading `-` or `TBD` is an author acknowledging the question
 * and declining to answer it, which is exactly what an unjustified exclusion is.
 * Listed explicitly rather than caught by a length floor: a floor teaches people
 * to pad, and `n/a` and a forty-character sentence saying nothing are the same
 * problem measured differently.
 */
const NON_ANSWERS = new Set([
  "-",
  "—",
  "n/a",
  "na",
  "none",
  "tbd",
  "todo",
  "?",
  "",
]);

/**
 * The smallest number of words that can state a reason.
 *
 * "controls no hardware" is three. Two cannot carry subject and predicate, and
 * one is a label. This is a floor on *structure*, not on length — it rejects
 * `safety: no` without inviting padding, which a character count does.
 */
const MIN_REASON_WORDS = 3;

/**
 * Assess one vocabulary declaration against what the bundle's documents say.
 *
 * Note the asymmetry, which is the policy: **`unowned` is medium and an
 * unjustified exclusion is high.** Saying nothing about reliability is an
 * admitted gap that a reader can see. Excusing it without a reason is an
 * assertion of completeness with nothing behind it, and it removes the finding
 * that would have prompted the work.
 */
export function assessVocabulary(
  declaration: VocabularyDeclaration,
  documents: DocumentClaims[],
): { rollup: VocabularyRollup; findings: CompletenessFinding[] } {
  const declared = new Set(declaration.values);
  const owned = new Set<string>();
  const excused = new Set<string>();
  const findings: CompletenessFinding[] = [];

  for (const document of documents) {
    for (const value of document.claims) {
      if (declared.has(value)) owned.add(value);
    }
    for (const value of document.excuses) {
      if (!declared.has(value)) {
        // A typo'd exclusion excuses nothing — the real value keeps reporting —
        // while reading, to its author, as handled. Worth naming on its own.
        findings.push({
          vocabulary: declaration.name,
          value,
          kind: "undeclared-exclusion",
          severity: "high",
          document: document.path,
          message:
            `'${value}' is excused under '${declaration.justifiedAbsenceField}' but is not one of ` +
            `the ${declared.size} values '${declaration.name}' declares, so it excuses nothing`,
        });
        continue;
      }
      excused.add(value);
      const reason = writtenReasonFor(value, document.body);
      if (reason === null) {
        findings.push({
          vocabulary: declaration.name,
          value,
          kind: "unjustified-exclusion",
          severity: "high",
          document: document.path,
          message:
            `'${value}' is excused under '${declaration.justifiedAbsenceField}' with no written reason; ` +
            `state why it does not apply in a table row naming '${value}'`,
        });
      }
    }
  }

  for (const value of declaration.values) {
    if (owned.has(value) || excused.has(value)) continue;
    findings.push({
      vocabulary: declaration.name,
      value,
      kind: "unowned",
      severity: "medium",
      message:
        `no document claims '${value}' for '${declaration.field}', and nothing records it under ` +
        `'${declaration.justifiedAbsenceField ?? "a justified-absence field"}'`,
    });
  }

  return {
    rollup: {
      vocabulary: declaration.name,
      declared: declared.size,
      owned: owned.size,
      excused: excused.size,
      unowned: declaration.values.filter(
        (v) => !owned.has(v) && !excused.has(v),
      ).length,
    },
    findings,
  };
}

/**
 * The written reason for excusing `value`, or `null` when there is none.
 *
 * A table row whose first cell names the value and whose remaining cells carry
 * a real sentence. A table is required rather than free prose because the value
 * name will occur in passing — "safety" appears in any document discussing
 * safety — and a mention is not a justification.
 */
export function writtenReasonFor(value: string, body: string): string | null {
  for (const line of body.split("\n")) {
    const trimmed = line.trim();
    if (!trimmed.startsWith("|")) continue;
    const cells = trimmed
      .split("|")
      .slice(1, -1)
      .map((cell) => cell.trim().replace(/^`|`$/g, "").trim());
    if (cells.length < 2) continue;
    if (cells[0].toLowerCase() !== value.toLowerCase()) continue;
    for (const cell of cells.slice(1)) {
      if (isReason(cell)) return cell;
    }
  }
  return null;
}

function isReason(cell: string): boolean {
  const normalized = cell.toLowerCase().replace(/[.\s]+$/, "");
  if (NON_ANSWERS.has(normalized)) return false;
  return cell.split(/\s+/).filter(Boolean).length >= MIN_REASON_WORDS;
}

/**
 * The verdict over every finding.
 *
 * `--strict` promotes an admitted gap to a failure; it does not invent one. The
 * default is advisory because a new check landing as a hard error across a
 * corpus teaches people to disable it, and a disabled check reports nothing
 * forever — which is the outcome this whole area exists to prevent.
 */
export function verdictFor(
  findings: CompletenessFinding[],
  strict: boolean,
  vocabulariesChecked = 1,
): Verdict {
  if (vocabulariesChecked === 0) return "UNCHECKED";
  if (findings.some((f) => f.severity === "high")) return "FAIL";
  if (findings.length === 0) return "PASS";
  return strict ? "FAIL" : "CONDITIONAL";
}
