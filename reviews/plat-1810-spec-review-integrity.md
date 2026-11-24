---
id: SR-20261124001343
title: "integrity review of the dossier risk content, safety class, run identity and records-of-processing check/export spec change"
type: SpecReview
analysis: integrity
scope: "agent-ix/quoin; spec/functional/FR-191-dossier-command.md, spec/functional/FR-192-dossier-srs-content.md, spec/functional/FR-193-dossier-architecture-content.md, spec/functional/FR-194-dossier-trace-matrix-content.md, spec/functional/FR-196-dossier-verification-report-content.md, spec/functional/FR-199-dossier-risk-management-content.md, spec/functional/FR-207-dossier-records-of-processing-content.md (PR #1038)"
review_set: subset
---

# integrity review of PR #1038

## Summary

Tickets: PLAT-1810, PLAT-1813, PLAT-1814. This review checked internal consistency,
completeness and testability of every changed statement and AC across the seven files.
The safety-class changes in FR-192 and FR-194 and the FR-191 gap list are consistent.
FR-199 has gaps in what "authored text" means, what decided records it renders, and
how AC-14 can be met when text is shown verbatim; FR-193 and FR-196 have undefined
terms.

## Findings

| ID      | Severity | Summary | Refs | Escape Cause |
| ------- | -------- | ------- | ---- | ------------ |
| FND-001 | medium | The FR-199 source contract does not say which authored text it exposes or in which fields. A hazard holds a `title`, a `## Condition` section and an Assessment `Rationale`; a situation and a harm hold a `title` and a `## Description` section. In the summary, `situation` and `harms` are bare ids, so "situations and harms carry these as optional text fields" has nothing to attach to. The sibling lead needs the source section per kind (Condition, Description), whether the `title` is shown, and the key names and shapes (for example harm entries as `{harm, text}`). | spec/functional/FR-199-dossier-risk-management-content.md:39-47,74; spec-objects-safety skeletons hazard, hazardous_situation, harm | missing-requirement |
| FND-002 | medium | FR-199 renders a `benefit_risk` record's decision, decider, rationale and residual-risk refs, and an overall evaluation's decision, decider and rationale, but omits the `Benefit` cell of the benefit-risk Determination table and the `Product` cell and considered-record `references` of the overall evaluation. The benefit is what the person weighed against the residual risks; a benefit-risk entry without it shows a decision without its subject. | spec/functional/FR-199-dossier-risk-management-content.md:43-47,60-64,77; spec-objects-safety skeletons benefit_risk, overall_residual_risk_evaluation | missing-requirement |
| FND-003 | medium | "The document SHALL NOT report a residual-risk, benefit-risk or overall record as accepted or approved" contradicts the closed decision vocabulary `undecided`, `accepted`, `not_accepted`: the document shows the recorded decision, which is the word `accepted`. State the rule as: the document states no acceptance or approval of its own and shows only the decision word and decider the record holds. | spec/functional/FR-199-dossier-risk-management-content.md:79 | wrong-requirement |
| FND-004 | medium | FR-199-AC-14 fails a correct implementation: it forbids any `accepted` or `approved` the decision text does not contain, but the document must also render rationales and hazard, situation and harm texts verbatim, and those may contain either word ("accepted clinical practice"). Restrict the check to text the tool itself writes (headings, labels, gap details), or require the fixture's other texts to avoid the words. | FR-199-AC-14; spec/functional/FR-199-dossier-risk-management-content.md:74 | wrong-requirement |
| FND-005 | medium | An `overall_residual_risk_evaluation` record whose decision is `undecided` yields no gap: `no_overall_evaluation` fires only when no record exists, `residual_risk_undecided` covers residual risks only, and the `Undecided decisions` section lists only residual and benefit-risk queue entries. DOS-RISK can therefore read `complete` while the overall decision is open. Either emit a gap for an undecided overall evaluation (stating the absence is not a decision) or state explicitly that it is shown without a gap. | spec/functional/FR-199-dossier-risk-management-content.md:78,90; FR-199-AC-13 | missing-requirement |
| FND-006 | medium | FR-193 says the document shows the class "for each part and each other item that records a `software_safety_class`". "Other item" is undefined; only the `part` carries the field (spec-objects-architecture FR-012), and the `unassigned` and gap rules cover parts only. Drop "and each other item" so the rule is part-level, as the author intends. | spec/functional/FR-193-dossier-architecture-content.md:37 | wrong-requirement |
| FND-007 | medium | "The newest retained run file for that criterion" is undefined. Runs belong to suites; a criterion can be bound to several suites, and a run of a bound suite may hold no entry for the criterion's bound symbols. Say which runs count (runs of a suite bound to the criterion that hold an entry for a bound symbol), how "newest" is decided (record `timestamp`, file name as tie-breaker, as `RetainedRun` orders them), and what is shown when several suites are bound. FR-196-AC-12 depends on this. | spec/functional/FR-196-dossier-verification-report-content.md:55; FR-196-AC-12 | missing-requirement |
| FND-008 | low | FR-196 Inputs list six values (timestamp, tool name, tool version, outcome, source commit, path) while Behavior says five (tool name and version together). The run record holds one `tool` string ("tool and version, as the adapter reported them"); splitting it would be computing. Say "tool as recorded" as one value in both places. | spec/functional/FR-196-dossier-verification-report-content.md:31,55 | wrong-requirement |
| FND-009 | low | A criterion with no binding or no run evidence already gets `unbacked_criterion` and/or `no_evidence`; FR-196 adds `run_identity_unrecorded` for the same absence (FR-196-AC-11). State whether both apply, or limit the new gap to criteria that have a retained run file with missing values. | spec/functional/FR-196-dossier-verification-report-content.md:58; FR-196-AC-11 | wrong-requirement |
| FND-010 | low | FR-199-AC-1 was weakened from "no gap" to "no `risk_chain_incomplete` gap" because the fixture now draws `no_overall_evaluation`. Add an overall evaluation record to the complete fixture and keep "no gap", so the AC still proves a complete chain is complete. | FR-199-AC-1 | wrong-requirement |
| FND-011 | low | The order of entries in `## Benefit-risk records` and `## Overall residual risk evaluation`, and what each section shows when the corpus has no such record, are unstated; FR-199-AC-7 requires byte-identical output, so two implementers would pick different orders. State dossier id order and a `none` line. | spec/functional/FR-199-dossier-risk-management-content.md:60-64 | missing-requirement |
| FND-012 | low | "`--version` supplied alongside only that kind SHALL NOT appear in its output" reads as if `--version` could appear in DOS-ROP when another kind is also selected. The preceding clause ("neither require nor emit a version for it") already covers every case; drop "alongside only that kind" or say "whatever else is selected". | spec/functional/FR-191-dossier-command.md:80 | wrong-requirement |

## Verdict

Not mergeable on this lens alone because of the medium findings in FR-199 (PLAT-1810:
FND-001..005, FND-010, FND-011), FR-193 and FR-196 (PLAT-1813: FND-006..009) and
FR-191 (PLAT-1814: FND-012). Consistent and clean: the FR-191 gap-kind list, gap order,
subject and no-id sort rules for all three new kinds; FR-192-AC-7..9 and
FR-194-AC-7..8 (testable, no inference); FR-193-AC-7..8; FR-196-AC-3 amendment
(subject to dependency FND-003); FR-199-AC-9..13; the DOS-ROP bundle order and its
consistency with FR-191-AC-16 for incomplete documents.
