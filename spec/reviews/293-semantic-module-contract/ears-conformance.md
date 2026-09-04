---
id: SR-125
title: "EARS conformance review of the semantic module contract"
type: SpecReview
analysis: ears-conformance
scope: "US-020, FR-070..FR-075, NFR-017"
review_set: all
---

# EARS conformance review of the semantic module contract

## Summary

The six functional requirements and one quality requirement carry 68 normative
statements (Description, Behavior bullets, Constraints). Quire's grammar check
(`quire validate --scope . "spec/**/*.md" --summary`, quire 0.31.0 / engine 0.46.0)
reports zero `[ears:*]` and zero `[quality:*]` findings on the reviewed files; every
statement has one `SHALL` and a named subject. The semantic read finds one high
finding where an unnamed manifest key collides with a closed key set, two medium
findings (an optional key missing from the closed set; an obligation hidden in a
parenthetical with no AC), and seven low pattern-fit notes, dominated by unwanted
conditions written as `When …` instead of `If … then …`. US-020 is a story and is
out of EARS scope.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-151 | high | FR-075 Behavior 7 "the `semantic` block SHALL declare the module version's compatibility posture (`strict`, `additive`, `declared-lossy`)" names no key; FR-070 Behavior 3 enumerates the optional keys (`exports`, `targets`, `mappings`) and FR-070-CON-2 closes the block with `additionalProperties: false`, so the posture key as written is rejected by FR-070-AC-3. Name the key and add it to FR-070's list. | FR-075; FR-070 |
| FND-152 | medium | FR-074 Behavior 5..7 read and gate `semantic.legacy_forms`, but FR-070 Behavior 3's optional-key list omits it; under FR-070-CON-2 the key is unknown. Add `legacy_forms` to the FR-070 enumeration. | FR-070; FR-074 |
| FND-153 | medium | FR-072 Behavior 1 hides a second obligation in a parenthetical: `sysml` and `fretish` are "admitted by the grammar but marked advisory until their frontends exist". No statement or AC says what "marked advisory" produces (FR-072-AC-2 covers only no-language and `tla`). Lift it to its own `If the language is `sysml` or `fretish`, then validation SHALL record an advisory finding …` statement with an AC. | FR-072 |
| FND-154 | low | Unwanted conditions written as `When …` rather than `If … then …`: FR-070 Behavior 5 (export names an undeclared type), 6 (`contract_version` out of range), 7 (duplicate `semantic.package`); FR-073 Behavior 5 (`$ref` at another semantic-core version). FR-071 and FR-072 use `If … then` for the same class of rejection; align. | FR-070; FR-073 |
| FND-155 | low | "When a module declares a `semantic` block" is a configuration state, not an event; EARS `Where …` (optional feature) fits: FR-070 Description, FR-073 Description and Behavior 6, FR-075 Description. FR-074 Behavior 5 already uses `Where …` correctly. | FR-070; FR-073; FR-075 |
| FND-156 | low | FR-071 and FR-072 Descriptions place the obligation on "the mapping contract" (the document itself): "SHALL define how …". The Behavior bullets carry the system obligations; recast the Description with Quire as subject or keep it as scope prose without `SHALL`. | FR-071; FR-072 |
| FND-157 | low | Two statements name no performing component: FR-071 Behavior 14 "The mapping SHALL record, per artifact, …" and FR-073 Behavior 7 "Validation of an artifact's extracted declaration set SHALL use …". Allocate both to Quire extraction / the validator. | FR-071; FR-073 |
| FND-158 | low | FR-075 Behavior 5 and 6 make an external actor the subject ("A dynamic consumer SHALL load …", "A static consumer SHALL find …"); the obligation is on the module and generated package. Restate as "The module SHALL load in a dynamic consumer with no generated package present" and "The generated package SHALL carry the same identities". | FR-075 |
| FND-159 | low | Trailing clauses carry content outside the `SHALL`: FR-071 Behavior 1 appends a definition ("… and a table with any other header is a legacy form"), FR-075 Behavior 7 appends an obligation ("which Quoin copies to the package manifest"). Split each into its own statement. | FR-071; FR-075 |
| FND-160 | low | FR-074 Behavior 9 "Quoin SHALL NOT edit, rewrite, or auto-migrate any artifact in a corpus repository" is a negative obligation; it is verifiable only through NFR-017's "corpus repository files written = 0" metric and NFR-017-AC-3. Accept as written; keep that cross-reference. | FR-074; NFR-017 |

## Statement classification

| Requirement | EARS form |
| --- | --- |
| FR-070 | Description: optional-feature stated as event-driven (FND-155). Behavior: 2 ubiquitous, 1 `MAY`, 4 event-driven (3 are unwanted conditions, FND-154), 1 ubiquitous on Quoin. CON-1..2 ubiquitous. |
| FR-071 | Description: ubiquitous on the document (FND-156). Behavior: 9 ubiquitous mapping rules, 4 `If … then` unwanted conditions, 1 ubiquitous with no agent (FND-157). CON-1..2 ubiquitous. |
| FR-072 | Description: ubiquitous on the document (FND-156). Behavior: 2 ubiquitous mapping rules, 4 `If … then` unwanted conditions, 1 ubiquitous on Quire; hidden advisory obligation (FND-153). CON-1 ubiquitous. |
| FR-073 | Description: optional-feature stated as event-driven (FND-155). Behavior: 2 ubiquitous, 1 `When` event, 1 `If … then`, 2 `When` (one unwanted condition, FND-154; one optional-feature, FND-155), 1 agentless ubiquitous (FND-157). CON-1..2 ubiquitous. |
| FR-074 | Description: event-driven (artifact validated under a semantic module). Behavior: 5 ubiquitous, 1 `Where …` optional feature, 1 `If … then` unwanted condition, 1 ubiquitous on Quoin, 1 `SHALL NOT` (FND-160). CON-1 ubiquitous. |
| FR-075 | Description: optional-feature stated as event-driven (FND-155). Behavior: 3 ubiquitous on Quoin, 1 `When` unwanted condition on `quoin install`, 2 ubiquitous on an external actor (FND-158), 1 ubiquitous with unnamed key and trailing obligation (FND-151, FND-159). CON-1..2 ubiquitous. |
| NFR-017 | Ubiquitous quality statement with a temporal bound (until sweep report and human promotion); five zero-threshold metrics make it measurable. Clean. |
| US-020 | User story; out of EARS scope. |
