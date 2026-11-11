---
id: SR-20261111201426
title: "Gap analysis of quoin PR 815 (PLAT-1401 NFR populations)"
type: SpecReview
analysis: base
scope: "agent-ix/quoin@4fe06bc; spec/functional/FR-131, FR-107-AC-3, FR-108 reason table, spec/non-functional/NFR-002/003/007/008, spec/assurance/MP-216..219, MP-228..230, MP-244, MP-250..254, spec/matrix.md"
review_set: subset
---

# Gap analysis of quoin PR 815

## Summary

Ticket: PLAT-1401. Plan completion: not assessed. `quoin matrix --json` (quoin 0.30.0) was
run on the PR head.

- FR-131-AC-7..11, NFR-003-M-1/M-2, NFR-007-M-1 and NFR-008-M-1/M-2 are all `tagged`.
- `criterion-tagged-outside-row` occurs 0 times.
- No id collides with origin/main or with open PRs: #812 uses TC-2900/2901, #813 uses
  TC-3100..3102, and #814 uses TC-3200..3206. MP-250..254 are unused elsewhere.

NFR claims were checked against the built binary (`rust/target/debug/quoin`) by writing
probe manifests through `QUOIN_MODULE_PATHS`. The EA schema was read at
engineering-assurance origin/main 4e8df92. Neither the Makefile nor CI runs `quire validate`
on plans.

## Verdict

**NOT MERGEABLE as claimed (one high).**

- **NFR-008 is marked Covered, but quoin violates it on a reachable input** (FND-001).
- **The rule is not toothless, but two of the seven exemptions are not honest** (FND-002).
  The exemption is a deliberate, reviewable, single-string choice, so a new proportion
  plan is forced to choose a set or an exemption. That is enough. Do not add a date
  cutoff; it would be ceremony. Review exemption reasons the way plan text is reviewed.
- **NFR-003 Partly and NFR-007 Partly are honest.** NFR-003's population comes from the
  binary's help and uses one clap parser, so it is close to pass-by-construction. It is
  still a real drift and regression detector, and it is better than the error-code taxonomy
  the brief suggested, which is pass-by-construction. The deviation is accepted.
- **The spec states a false schema premise** (FND-004). Shape check for the EA PR: quoin's
  `population_set` is an object `{ids: [...]}`, not a bare array. It is closed, with `ids`
  as the only member, and `ids` is a non-empty array of unique, non-empty strings.
  `population_exempt` is a string that must contain a non-whitespace character; quoin
  refuses `"   "`, so `minLength: 1` alone diverges and needs `pattern: "\\S"`.
- **The #807 bounds hold.** Refusal output is capped at 5 ids, 64 characters and 100
  findings. The plan sets declare 18, 18, 4, 7 and 4 ids. The PR changes no census, pin or
  help surface.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | spec/matrix.md marks NFR-008 `Covered`, but quoin violates NFR-008-M-1. Measured: a `manifest.yaml` that is empty, `~` or a bare scalar makes `quoin catalog list --json` exit 0 and print a catalog without that module, which is the "partial catalog" NFR-008 forbids. The cause is `let Some(manifest) = manifest.as_object() else { continue; }` in quoin-catalog. MP-253's population leaves these kinds out, and its text describes the empty file as "accepted by the YAML reader" when the module is actually dropped. Downgrade to Partly and file the defect, or add the kinds and fix the code | spec/matrix.md:152, spec/assurance/MP-253-nfr008-corrupt-manifests.md, rust/crates/quoin-catalog/src/lib.rs:175 |
| FND-002 | medium | The MP-229 and MP-230 exemptions say "the denominator is bars read from other plans observations, not a set of ids". Each plan's own `population` names a fixed set of bars: MP-222, MP-223, MP-224 and MP-231 for MP-229, and MP-222, MP-223 and MP-224 for MP-230. That set is enumerable as `population_set.ids`. The exemption silences the rule exactly where a set fits | spec/assurance/MP-229-jev-ears-gate.md:16, spec/assurance/MP-230-jev-gap-analysis-gate.md:16 |
| FND-003 | low | The MP-219 exemption ("this plan does not enumerate its ids") is a convenience exemption. The frozen label set in the protected `bench/span-breadth-v1-labels.json` is enumerable. Protecting the apparatus limits the harm, but the reason states a choice, not why the set cannot be enumerated | spec/assurance/MP-219-span-breadth.md:16 |
| FND-004 | medium | The FR-131 Behavior bullet says `population_set` sits "in a member the engineering-assurance plan schema already leaves free-form". That is false. EA `$defs.statistical_design` has `additionalProperties: false` and does not list `population_set` or `population_exempt`, so `quire validate` refuses MP-250..254 and the seven exempt plans until the EA PR lands. No quoin gate runs it, so nothing is red. Reword the bullet to cite the EA dependency | spec/functional/FR-131-percentage-metrics-over-enumerated-population.md:34 |
