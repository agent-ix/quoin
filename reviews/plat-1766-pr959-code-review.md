---
id: SR-20261116075209
title: "Code review — PLAT-1766 and PLAT-1614 test corrections"
type: SpecReview
analysis: code-review
scope: "agent-ix/quoin; quoin#959; MP-250, MP-251, tc_1614_dossier_command.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quoin/MP-250
    type: references
  - target: ix://agent-ix/quoin/MP-251
    type: references
---

## Summary

Reviewed the frozen PR #959 diff against base `6f057d52` at head `b552e45c`, including the PLAT-1766 population correction and PLAT-1614 validation fixture. The changes are test and measurement-plan data only; no defect was found.

## Verdict

**PASS** — the population declarations match the new CLI routes, and the negative dossier fixture now uses a value outside the current kind enum.

## Assurance Context

AP-201 (`spec/assurance/AP-201-finding-quality.md`) applies because the change updates a measured population declaration. The plans, FR-131-AC-9, NFR-003 metrics, source enumerator, and retained test were available. No new collection or changed result is asserted. AP-202 does not apply. Active exceptions: none found. Unavailable context: no new measurement record was supplied or claimed.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Coverage

MP-250 and MP-251 each add exactly five IDs relative to the PR base: four routes from the newly registered `formal-rules` command and `formalize:unknown-flag`. Both declarations contain 156 unique IDs, remain identical, and remove no prior member. `formal-rules` is not one of MP-252's ix-flow workflow commands; the unchanged `tc_3004` checks that distinct population. The dossier test replaces `verification-report`, which is allowed, with `unknown-dossier-kind`; the current DossierKind schema excludes that value, and the test requires both validation failure and a `document_kind` diagnostic. The test remains ignored for its external Quire/module prerequisites and was run explicitly.

Rust review found no production-code, API, safety, or lifecycle changes. The changed Rust expression preserves the existing test's meaningful negative assertion. The test's existing `#[trace("FR-191")]` tag is unchanged; no traceability behavior is introduced by this diff.

## Verification

Independent source review and YAML parsing confirmed equal, duplicate-free population sets with only the five expected additions. `quire validate` accepted both changed MeasurementPlans (exit 0; it emitted an unrelated existing unused-import warning for spec-artifacts-app). Through `locked-build.sh`, reviewer checks passed: `cargo fmt --all -- --check`, strict Clippy for quoin-cli tests/all features, `tc_3003_nfr_populations` (14 passed, 0 failed, 1 ignored), and the ignored PLAT-1614 test against the v0.34.0 spec-artifacts-process snapshot (1 passed). An initial run against the separate current-main module repro failed before the assertion because installed Quire 0.45 / engine 0.60 rejects that checkout's `formalizes` manifest field; the fixture test itself passed against the module release used in the PR evidence. The final aggregate gate remains with the lead.

## Examined scope

```yaml
scope:
  - id: MP-250
    path: spec/assurance/MP-250-nfr003-actionable-failures.md
    role: examined
    excerpt: "The population is the complete set of command x error-class pairs enumerated from the binary's help text."
  - id: MP-251
    path: spec/assurance/MP-251-nfr003-silent-failures.md
    role: examined
    excerpt: "The population is the same command x error-class pairs as MP-250."
  - id: MP-252
    path: spec/assurance/MP-252-nfr007-external-tool-failures.md
    role: context_only
    excerpt: "The population covers workflow commands crossed with the ways ix-flow can fail."
  - id: FR-131-AC-9
    path: spec/functional/FR-131-percentage-metrics-over-enumerated-population.md
    role: examined
    excerpt: "Each plan's declared set equals the command/error-class enumeration from the binary's help text."
  - id: tc_3003_the_nfr003_populations_are_stated_by_class
    path: rust/crates/quoin-cli/tests/tc_3003_nfr_populations.rs
    role: examined
    excerpt: "The producer's examined IDs equal the authored populations for MP-250 and MP-251."
  - id: generated_frontmatter_validates_and_an_unknown_document_kind_does_not
    path: rust/crates/quoin-cli/tests/tc_1614_dossier_command.rs
    role: examined
    excerpt: "Generated document kinds validate; an unrecognized document_kind fails validation and names that field."
  - id: DossierKind
    path: spec-artifacts-process/spec_artifacts_process/schemas/DossierKind.json
    role: examined
    excerpt: "The current enum includes verification-report and excludes unknown-dossier-kind."
```
