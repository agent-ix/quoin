---
id: SR-17081
title: "Spec review of quoin PR 816 (PLAT-1403 MP-219, MP-229, MP-230 population text and FR-131-AC-13/14)"
type: SpecReview
analysis: base
scope: "agent-ix/quoin@a0b62b2c; spec/assurance/MP-219-span-breadth.md, spec/assurance/MP-229-jev-ears-gate.md, spec/assurance/MP-230-jev-gap-analysis-gate.md, spec/functional/FR-131-percentage-metrics-over-enumerated-population.md (AC-13, AC-14)"
review_set: subset
date: '2026-11-11'
---

# Spec review of quoin PR 816

## Summary

Ticket: PLAT-1403. I applied the integrity and EARS sub-checks inline, scoped to the changed plan bodies and the two new criteria.

## Verdict

**The MP-219 decision rule did not change.** `comparator: ge` and `threshold: 1.0` are untouched, and so are `population_set`, `protected_apparatus` and the negative controls. Nothing was weakened to let 32/40 pass, and the change in label count changes no threshold.

The removed "at least 60 / five shapes / three repositories" prose described the corpus before the owner's own #569 (2026-09-20) reduced it to the repositories' own specs. #717 later removed the ceremony floors. I re-measured the replacement text against the file and it is accurate: 40 labels, 20 with spans and 20 refusals, 8 `property` values, 2 repositories.

**MP-229 and MP-230 are not claimed as measured.** Both say "no live verdict is claimed", which agrees with the PR body.

**Stale labels.** The question was whether a label that has gone stale should count as not matched, or get a separate `stale-label` classification. Not matched is the honest behaviour this plan prescribes:

- `error_model` names "a repository drifts from the frozen set".
- Comparison and Enforcement says repository drift blocks promotion.
- A separate classification that left drifted labels out of the denominator would be a plan change, and would weaken the gate.

The follow-up decision belongs to the owner and is out of scope for this PR. Either re-review the label set against the current specs under a new `definition_version`, or retire MP-219. The label file must not be relabelled in place.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | MP-219's Comparison and Enforcement still lists "breadth-floor failure" as a blocker. This PR removed the only text that stated a breadth floor, so the clause now refers to nothing. Delete the words or state the floor | spec/assurance/MP-219-span-breadth.md:65 |
| FND-002 | low | MP-229's and MP-230's `sampling` still reads "reads the already-collected upstream observations". The new Population text says the live gate run produces the observation from the bars it computes inline. Align `sampling` with the producer | spec/assurance/MP-229-jev-ears-gate.md:13, spec/assurance/MP-230-jev-gap-analysis-gate.md:13 |
