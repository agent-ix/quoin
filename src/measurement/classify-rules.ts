/**
 * The authored classification rules (FR-090).
 *
 * These are rules a person authored after reading sampled documents, not
 * heuristics the measurement invented. Each carries the evidence that settled
 * it, because the difference between "the check misreads correct data" and
 * "the documents are wrong" is a question of fact that has to be answered by
 * opening files, and the answer belongs next to the rule it produced.
 *
 * Sampling, 2026-09-04, over the 300 engine failures in the pinned corpus:
 *
 *   missing (227)     `spec/tests.md` documents with no `test_cases` or
 *                     `functional_coverage` table at all. Sampled
 *                     agent-cli-daemon and agent-config-cookiecutter: the
 *                     tables are genuinely absent, not present in a form the
 *                     check could not read. Bad corpus.
 *
 *   assert (68)       `spec/tests.md` tables whose declared column set does
 *                     not match the archetype, and `Traces To` cells holding
 *                     an em dash where an id is required. Sampled
 *                     agent-duncan, catalog-service, cloud-manager-ui-services.
 *                     The columns really differ and the cell really holds a
 *                     placeholder. Bad corpus.
 *
 *   frontmatter (5)   `AssuranceProfile` documents in quire-rs carrying a
 *                     `concern` property the archetype does not allow, and
 *                     missing `id`/`severity` inside `impact_assessment`.
 *                     Sampled AP-201. The document was authored against a
 *                     different revision of the archetype than the pinned
 *                     module declares.
 *
 * None of the three is a rule defect, so none is widened. Every one is work
 * for the later normalization campaign, which this ticket does not perform.
 */

import type { Disposition, FindingClass } from "./partition.js";

export interface Rule {
  readonly reason: string;
  readonly classification: FindingClass;
  readonly evidence: string;
  readonly disposition: (repository: string) => Disposition;
}

/** The later campaign every corpus finding is deferred to. */
export const NORMALIZATION_CAMPAIGN = "corpus normalization campaign (post-Wave-4)";

export const RULES: readonly Rule[] = [
  {
    reason: "missing",
    classification: "missing-structure",
    evidence:
      "sampled agent-cli-daemon and agent-config-cookiecutter: the required TestMatrix tables are absent from the document, not present in an unreadable form",
    disposition: (repository) => ({
      owner: repository,
      kind: "deferred-to-later-campaign",
      nomination: NORMALIZATION_CAMPAIGN,
    }),
  },
  {
    reason: "assert",
    classification: "malformed-document",
    evidence:
      "sampled agent-duncan, catalog-service and cloud-manager-ui-services: declared column sets differ from the archetype and a Traces To cell holds an em dash where an id is required",
    disposition: (repository) => ({
      owner: repository,
      kind: "deferred-to-later-campaign",
      nomination: NORMALIZATION_CAMPAIGN,
    }),
  },
  {
    reason: "frontmatter",
    classification: "stale-module",
    evidence:
      "sampled quire-rs AP-201: the document carries a `concern` property the pinned AssuranceProfile archetype does not admit and omits fields it requires, so it was authored against a different revision",
    disposition: (repository) => ({
      owner: repository,
      kind: "deferred-to-later-campaign",
      nomination: NORMALIZATION_CAMPAIGN,
    }),
  },
];

/** The bracketed reason a Quire diagnostic ends with, or null. */
export function reasonOf(message: string): string | null {
  const m = /\[(\w[\w-]*)\]\s*$/.exec(message);
  return m ? m[1] : null;
}

/**
 * Applies the authored rules. A message whose reason no rule covers is left
 * for the partition to record as `unknown`: an authored rule set that silently
 * absorbs an unrecognised reason is indistinguishable from one that got it
 * right, and only one of those is true.
 */
export function ruleFor(message: string): Rule | null {
  const reason = reasonOf(message);
  if (reason === null) return null;
  return RULES.find((r) => r.reason === reason) ?? null;
}
