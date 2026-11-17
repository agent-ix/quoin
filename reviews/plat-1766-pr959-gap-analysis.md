---
id: SR-20261116075210
title: "Gap analysis — PLAT-1766 and PLAT-1614 test corrections"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quoin; quoin#959; changed population plans and dossier validation test"
review_set: subset
relationships:
  - target: ix://agent-ix/quoin/MP-250
    type: references
  - target: ix://agent-ix/quoin/MP-251
    type: references
---

## Summary

Compared the PR's changed population declarations and validation assertion against their owning measurement criteria and the PLAT-1766 / PLAT-1614 acceptance scope. The requested evidence paths are present; the installed Quoin has no `gap-analysis` command, so this is a scoped manual check rather than a whole-repository audit.

## Verdict

**PASS (scoped manual check)** — no new implementation or requirement-to-test gap was introduced in the PR diff.

## Assurance Context

AP-201 applies to the updated measured population; its population-integrity concern was checked against the authored sets and their exact-set test. AP-202 does not apply. Active exceptions: none found. Unavailable context: the Quoin gap-analysis subcommand is not present in the installed CLI, and the PR does not change or claim a new measurement record.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Coverage

Plan completion: not assessed

The existing `tc_3003` producer enumerates help-derived NFR-003 IDs and compares both plan sets exactly; MP-250 and MP-251 now include the five current-main additions. `tc_3004` separately validates MP-252's ix-flow failure population, which is unchanged. The PLAT-1614 ignored test validates generated documents, then asserts that `unknown-dossier-kind` is rejected with a field-specific diagnostic; this matches the acceptance scope described for that ticket and the external DossierKind enum.

`quire matrix --scope . --format json` reports untagged FR-131-AC-9, FR-131-AC-10, and FR-191-AC-9. Their source tests and trace annotations are unchanged by this PR; no criteria, test trace tags, or test-matrix rows were changed. This existing matrix behavior is therefore not a gap introduced by PR #959. No whole-repository reverse-gap audit is claimed because the installed Quoin CLI reports `command gap-analysis not found`.

## Verification

`quire validate` accepted both edited MeasurementPlans (exit 0, with an unrelated existing spec-artifacts-app unused-import warning). Independent YAML comparison found 156 unique IDs in each NFR-003 plan, exact equality between the sets, no removals, and exactly five additions relative to the PR base. Reviewer checks via `locked-build.sh` passed strict Clippy, the population suite (14 passed, 0 failed, 1 ignored), and the PLAT-1614 ignored validation test with the v0.34.0 module snapshot (1 passed). The current-main module repro is not compatible with installed Quire 0.45 / engine 0.60 because its manifest includes `traceability.trace_tags.formalizes`; that failure occurs before document validation and does not change the fixture's negative assertion.

## Examined scope

```yaml
scope:
  - id: MP-250
    path: spec/assurance/MP-250-nfr003-actionable-failures.md
    role: examined
    excerpt: "Each plan declares the complete command/error-class population from help enumeration."
  - id: MP-251
    path: spec/assurance/MP-251-nfr003-silent-failures.md
    role: examined
    excerpt: "MP-251 measures the same command/error-class population as MP-250."
  - id: MP-252
    path: spec/assurance/MP-252-nfr007-external-tool-failures.md
    role: context_only
    excerpt: "MP-252 measures ix-flow failure modes for workflow commands."
  - id: FR-131-AC-9
    path: spec/functional/FR-131-percentage-metrics-over-enumerated-population.md
    role: examined
    excerpt: "The declared set equals the help-derived command/error-class enumeration."
  - id: tc_3003_the_nfr003_populations_are_stated_by_class
    path: rust/crates/quoin-cli/tests/tc_3003_nfr_populations.rs
    role: examined
    excerpt: "The producer's observed population equals MP-250 and MP-251."
  - id: tc_3004_the_nfr007_population_is_stated_by_failure_mode
    path: rust/crates/quoin-cli/tests/tc_3003_nfr_populations.rs
    role: examined
    excerpt: "The producer's observed ix-flow population equals MP-252."
  - id: generated_frontmatter_validates_and_an_unknown_document_kind_does_not
    path: rust/crates/quoin-cli/tests/tc_1614_dossier_command.rs
    role: examined
    excerpt: "An unknown document_kind must fail archetype validation and name the field."
```
