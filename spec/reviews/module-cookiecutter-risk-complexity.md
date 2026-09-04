---
id: SR-133
title: "Risk and complexity analysis of the semantic-module cookiecutter"
type: SpecReview
analysis: risk-complexity
scope: "StR-008; US-021; FR-076..FR-083; NFR-018; NFR-019"
review_set: all
---

## Summary

This slice specifies one cookiecutter template rendering Quire semantic-module
repositories in `artifact`, `object`, or `mixed` variants. The template's shape
is well grounded for two of the three variants: `spec-objects-business` and
`spec-artifacts-iso` converged on it independently by hand. That grounding does
not extend uniformly across the slice. The underlying semantic-module contract
(FR-070 through FR-075) is still moving while five more repositories migrate to
it concurrently, so FR-078's manifest block and FR-077's emission pipeline are
specified against a target that can shift under the template. FR-080's
verification suite is designed to fail hard rather than skip when the Quire
engine is absent, which is the right call given that the wheel exposing
`extract_semantic` (`agent-ix/quire-rs#392`) is unpublished — but it also means
the suite's actual semantic assertions cannot be demonstrated end-to-end until
that wheel ships. FR-076's `generated_targets` input accepts any entry from the
filament-core-data target registry while only one of five declared targets has
a working emitter, pushing correctness onto rendering-time bookkeeping that
records what is declared-but-not-emitted. The mixed variant, unlike the other
two, has no prior hand migration to converge from. Dual Python/npm packaging
parity (FR-081), rendering and re-emission determinism (NFR-019), and the
self-referential drift check comparing the template against live maintained
repositories (FR-083) are all real complexity independent of contract
volatility. The largest standing hazard named in the spec itself — a template
nobody instantiates drifting silently from the contract — is mitigated only by
FR-083's drift check, which itself depends on those same external repositories
staying reachable and representative.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | The semantic-module contract underlying the template is still moving while five repositories migrate to it concurrently, so the template hard-codes a manifest and emission shape against a target that can shift after this slice ships. | FR-078, FR-077, US-021 |
| FND-002 | high | FR-080's verification suite is a hard dependency on the Quire wheel exposing extract_semantic, which is unpublished under agent-ix/quire-rs#392, so the suite's semantic assertions cannot be demonstrated end-to-end until that wheel ships and only its failure path is exercisable today. | FR-080, StR-008 |
| FND-003 | high | The mixed variant combines an artifact and an object manifest section and at least one imported module, but unlike the other two variants it has no prior hand migration that converged on this shape, so its correctness rests on inference from the other two rather than on independent confirmation. | FR-076, FR-078 |
| FND-004 | medium | FR-076 accepts any generated_targets entry present in the filament-core-data target registry while only one of five declared targets has a working emitter, so honest non-emission reporting in the rendered README and Test Matrix is new bookkeeping that has never been exercised against a real multi-target registry. | FR-076 |
| FND-005 | medium | FR-081 requires byte-identical manifest, schema, and skeleton content across a Python wheel and an npm tarball built by two different packaging toolchains, a class of parity defect that is easy to introduce during either toolchain's own maintenance and easy to miss without a dedicated comparison test. | FR-081 |
| FND-006 | medium | Deterministic rendering and schema re-emission depend on avoiding timestamps, unsorted map iteration, and embedded absolute paths, a well known failure class that breaks every downstream byte-comparison gate at once and typically fails silently rather than loudly. | NFR-019, FR-077, FR-083 |
| FND-007 | high | The principal risk the spec itself names is a maintained-but-never-instantiated template drifting from the contract silently, and the only structural guard against it is FR-083's own render-and-compare gate, which is exactly as reliable as the template's discipline in keeping that gate current. | US-021, FR-083 |
| FND-008 | medium | FR-083's conformance-drift check requires Quoin's own gate to read the real maintained semantic-module repositories at CI time, coupling this repository's build to the availability, access, and pace of external repositories it does not own. | FR-083 |
| FND-009 | medium | Each exported type carries a typed skeleton, a sysml-fence alternate, ocl invariants, one negative fixture per declared failure mode, a legacy-form fixture, and for artifact types a mapping declaration plus a golden record, all of which must agree with one another and with the emitted schema. | FR-079 |
| FND-010 | low | Textual manifest-digest rewriting must preserve every comment and YAML anchor across regeneration, a narrower determinism requirement layered on top of the broader byte-identical rendering guarantee. | FR-078, NFR-019 |
| FND-011 | medium | FR-082's validity-by-construction claim for the rendered Test Matrix depends on the spec_artifacts_process archetype's status vocabulary staying exactly as the template assumes, so a later archetype change elsewhere can invalidate a previously conforming render without touching the template itself. | FR-082 |

