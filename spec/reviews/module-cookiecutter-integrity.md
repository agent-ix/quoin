---
id: SR-130
title: "Integrity analysis of the semantic-module cookiecutter"
type: SpecReview
analysis: integrity
scope: "StR-008; US-021; FR-076..FR-083; NFR-018; NFR-019; TC-1400..TC-1448"
review_set: all
---

# SR-130: Integrity analysis of the semantic-module cookiecutter

## Summary

The slice (StR-008, US-021, FR-076..FR-083, NFR-018, NFR-019, TC-1400..TC-1448)
is well-formed on the whole: every FR implements US-021, every FR's acceptance
criteria carry a test reference, and the negative-path aborts for `module_kind`,
`license`, `imported_modules`, and `generated_targets` are each stated and each
tested. Ten findings surface below. Two are high severity and both are Hidden
Assumption Probe hits: the spec never states a minimum-version and
detection/error contract for the external CLIs it delegates to (cookiecutter,
`tsp`, `quire validate`, npm, poetry), and it never separates an interactive
scaffolding mode from a scripted/CI one despite the probe table flagging both
patterns as relevant. The remaining findings are a validation gap on the
`mixed` + empty `imported_modules` combination, a contradiction between FR-082
and StR-008's own stated scope, a missing tie-break rule for the maintained-
repository drift comparison, two traceability gaps in the relationship
metadata, a systemic non-atomicity pattern across several Test Matrix rows,
and two lower-severity terminology/ownership gaps.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | high | No NFR or FR in scope states a minimum version, a detection method, or a user-facing absent-tool error for any of the external CLIs the FRs delegate to: the TypeSpec compiler `tsp` (FR-077), `quire validate` (FR-082, FR-083), and npm and poetry (FR-081). FR-080 does this correctly for the Quire engine wheel but the pattern was not carried to the other three tools. | FR-077; FR-081; FR-082; FR-083; NFR-018; NFR-019 | missing-requirement |
| FND-002 | high | No FR states that the scaffolding command has both an interactive human-at-terminal mode and a scripted or CI mode with explicit flags. FR-076 Inputs are described only as answered prompts; there is no `--no-input`, argument-file, or environment-variable equivalent for unattended rendering, which the gate itself needs to render three variants unattended (FR-083). | FR-076; FR-083 | missing-requirement |
| FND-003 | medium | FR-076 states "When module_kind is mixed, the rendered semantic.imports SHALL carry at least one imported package identity" as a rendered-output invariant, but unlike every other invalid-input case in the same FR (bad license, bad module_kind, malformed imported_modules entry, out-of-registry target), there is no paired "if module_kind is mixed and imported_modules is empty, then abort naming..." bullet. The behavior is asserted with no owning validation rule. | FR-076 | missing-requirement |
| FND-004 | medium | FR-082 requires the rendered spec/ tree to carry "the functional requirements that describe the rendered module's own contract," while StR-008's Context states "the template does not attempt to generate a module's types." A rendered repository has no domain vocabulary at render time, so it is unresolved what content those generated functional requirements assert, or whether FR-082 means the generic semantic-module contract rather than the module's own domain contract. | FR-082; StR-008 | wrong-requirement |
| FND-005 | medium | FR-083's drift check requires comparing the conformance contract against "the surfaces the maintained semantic-module repositories carry," but no requirement states how those repositories are located, fetched, or version-pinned for the comparison, nor a tie-breaking rule for when two maintained repositories disagree on which surfaces exist. This is the Hidden Assumption Probe's multi-source lookup pattern. | FR-083; StR-008-VC-3 | missing-requirement |
| FND-006 | medium | StR-008's frontmatter `satisfied_by` relationships name only FR-076 and FR-083, but StR-008's own Validation Criteria table cites FR-078 and FR-080 by test reference (TC-1410, StR-008-VC-2) and FR-083 again (TC-1418, StR-008-VC-3). The declared relationship graph undercounts the FRs the stakeholder requirement's own criteria depend on. | StR-008 | correct-requirement-no-evidence |
| FND-007 | medium | NFR-019's relationships and Dependencies list FR-077 and FR-083 as constrained, but omit FR-076, even though NFR-019's Statement explicitly covers "rendering a variant twice" — FR-076's rendering process, not only FR-077's schema re-emission — and TC-1403 tests FR-076-AC-4's render-determinism claim directly. | NFR-019; FR-076 | correct-requirement-no-evidence |
| FND-008 | medium | Several Test Matrix rows bundle two or three unrelated assertions under one TC id, so a single test failure does not localize to which acceptance criterion broke: TC-1402 (mixed-sections presence, imports-with-version mapping, and empty-imports-yields-empty-mapping are three separate claims), TC-1403 (single-sourcing across variants and render-to-render determinism are unrelated claims), TC-1417 (nine-key completeness, schema validation, and exports-equal-declared-types are three claims), TC-1430 (no engine dependency and warnings-as-errors are unrelated), and TC-1440 (no retired marker present, vocabulary conformance, and injected-marker gate failure are three claims). | TC-1402; TC-1403; TC-1417; TC-1430; TC-1440 | wrong-requirement |
| FND-009 | low | FR-076 introduces `generated_targets`, "a subset of the filament-core-data target registry," and requires an undeclared-but-not-emitted target to be recorded in the rendered README and Test Matrix. No FR in scope states what emitting a target beyond JSON Schema actually produces; FR-077 only specifies JSON Schema emission. The semantics of a `generated_targets` entry other than the schema target are unowned by any requirement in this slice. | FR-076; FR-077 | missing-requirement |
| FND-010 | low | FR-080-CON-1 and TC-1427 introduce the term "a strict expected failure with a paired control" for the engine-absent case without defining it anywhere in FR-080's Behavior, which otherwise only specifies that the suite "SHALL fail naming the provisioning command" and "SHALL NOT call a skip." It is unclear whether this term names a distinct test-runner marker (e.g., an xfail) that the "no skip" rule would otherwise prohibit, or is merely descriptive prose for the same failing test. | FR-080 | wrong-requirement |
