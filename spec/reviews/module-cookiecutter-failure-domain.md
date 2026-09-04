---
id: SR-129
title: "Failure-domain analysis of the semantic-module cookiecutter"
type: SpecReview
analysis: failure-domain
scope: "StR-008; US-021; FR-076..FR-083; NFR-018; NFR-019"
review_set: all
---

## Summary

This slice specifies a cookiecutter template that renders a Quire semantic
module (artifact, object, or mixed) with a TypeSpec schema source, a
deterministic emit pipeline, a `semantic` manifest block, typed skeletons, and
a render-and-conform gate. The requirements are unusually strict about strict
failure over silent skipping (FR-080), and about residue and determinism
(NFR-018, NFR-019). Even so, several trust boundaries, identity keys, and
graph-shaped surfaces are left implicit rather than stated: the set of
"maintained semantic-module repositories" FR-083's drift check compares
against is never pinned to anything reviewable; FR-077's two defined `$ref`
bases do not cover the imported-package case FR-076 itself introduces; nothing
declares a uniqueness key for exported type names across `artifact_types`,
`object_types`, and imported packages, or for `imported_modules` entries
themselves; the engine-presence check in FR-080 never extends to
engine-version compatibility; and NFR-019's determinism claim is explicitly
scoped to same-OS renders, leaving cross-OS non-determinism unaddressed for a
template maintainers will not all render on one platform. None of these are
visible from reading any single requirement; each surfaces only when two or
three are read together against the ground-truth shape in
`spec-objects-business` and `spec-artifacts-iso`.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | FR-083's conformance-drift check compares the template's contract against "the maintained semantic-module repositories," but no requirement names, pins, or versions that repository set; the comparison target can change between two runs of the same gate with no declared file recording what it was compared against, undermining both the gate's reviewability and NFR-019's determinism goal. | FR-083; NFR-019; StR-008 |
| FND-002 | high | FR-077 defines exactly two valid targets for an absolutized `$id` or `$ref`, this module's base or the semantic-core base, but FR-076 permits `imported_modules` and FR-078 requires `semantic.imports` entries for them; neither FR-077 nor FR-078 states what base a `$ref` into an imported package's types should carry, leaving mixed-variant cross-package references undefined. | FR-076; FR-077; FR-078 |
| FND-003 | medium | No requirement defines a uniqueness key for exported type names across a mixed variant's `artifact_types` and `object_types` sections, or between this module's own exports and an imported package's exports; a name collision at either boundary is not detected, rejected, or even acknowledged as possible. | FR-076; FR-078; FR-081 |
| FND-004 | medium | `imported_modules` entries are validated only for their own `<org>/<repo>@<exact-version>` shape; nothing requires the org/repo portion to be unique across entries, so two entries naming the same package at two different versions collapse silently into one `semantic.imports` key with no reported conflict. | FR-076; FR-078 |
| FND-005 | medium | FR-080 fails the suite only when the ambient Quire engine cannot be imported or does not expose `extract_semantic`; it never checks that engine's version against the rendered manifest's `contract_version` or `semantic_core` version, so a present but incompatible engine can produce a false-green run rather than the strict failure this requirement otherwise insists on. | FR-080 |
| FND-006 | medium | FR-076 requires every declared-but-unemitted target to be recorded in both the rendered README and the rendered Test Matrix, and FR-078 requires `semantic.targets` to reflect the toolchain; no acceptance criterion checks that these three declarations of "what this module emits" agree, so they can drift apart with nothing to catch it. | FR-076; FR-078 |
| FND-007 | medium | NFR-019 scopes its determinism guarantee to "two renders on machines whose only difference is the working directory and the clock," which excludes cross-OS sources of non-determinism such as line-ending normalization, file-mode bits, and path-separator handling inside absolute `$id` URLs, even though nothing in this slice restricts maintainers to rendering on one operating system. | NFR-019 |
| FND-008 | medium | NFR-018's residue scan requires zero "credential or token matches" and zero "private-registry publication defaults," and FR-083 requires the gate to assert the same, but no requirement names or references the pattern set that decides what counts as a match; the verification method restates the requirement's own words rather than defining the ruleset, so two implementations of the scan could disagree on the same rendered tree. | NFR-018; FR-083 |
| FND-009 | low | FR-076's abort behavior for a rejected `license`, `module_kind`, `imported_modules` entry, or `generated_targets` entry never states whether a partially rendered directory is deleted on abort; a failed render could leave residue on disk that would itself violate NFR-018 if the directory were inspected rather than discarded. | FR-076; NFR-018 |
| FND-010 | low | FR-079-AC-4 requires each negative fixture be refused "for its own distinct reason," but no requirement defines a canonical identifier or registry for a declared failure mode; distinctness depends on comparing free-text diagnostic strings, so two authors of two rendered variants could label one violation two different ways, or two distinct violations could coincidentally emit the same diagnostic, with no structural check for either case. | FR-079 |
| FND-011 | low | FR-079's skeleton, mapping, and golden-record round-trip checks give no termination guarantee for an exported type that is self-referential or part of a reference cycle, the shape the ground-truth `allowed_links` graphs actually contain (an `entity` referencing `entity`, a `state_machine` transitioning to `state_machine`); nothing states how the round-trip assertion behaves over such a cycle. | FR-079 |
