---
id: SR-132
title: "Evidence analysis of the semantic-module cookiecutter"
type: SpecReview
analysis: evidence
scope: "StR-008; US-021; FR-076..FR-083; NFR-018; NFR-019; TC-1400..TC-1448"
review_set: all
---

## Summary

`quoin advise --repo .` was run over the full repository and filtered to this
slice's obligations (65 AC/NFR-Measurement rows). It found zero genuine
mismatches at the AC level: every authored `Test` tag is a recommendation the
catalog's `applicability` rules also reach, mostly via the `example`
property-shape (`bdd-spec-by-example` / `unit-testing`). `quire coverage` does
not model Constraint rows as obligations at all, so the eight `-CON-` rows in
this slice (FR-076-CON-1/2, FR-077-CON-1/2, FR-078-CON-1/2, FR-079-CON-1/2,
FR-080-CON-1/2, FR-081-CON-1/2, FR-082-CON-1/2, FR-083-CON-1/2) and the three
StR-008 validation criteria were judged by hand against the same catalog and
against `quoin catalog methods`.

Two classes of real problems surfaced. First, both NFRs' Measurement tables use
`Method` values (`Rendered-tree scan`, `Byte comparison`) that exist nowhere in
`quoin catalog methods` and nowhere else in this repository's NFR files, all
seventeen of which (NFR-001..NFR-017) author `Method` as one of the four
declared classes. Second, six Test Matrix rows under FR-079 (TC-1420, TC-1421,
TC-1422, TC-1423, TC-1424, TC-1426) carry `Type: Unit` or `Type: Integration`
for criteria `quoin advise` itself tags with a `universal` or `round-trip`
property-shape and recommends `property-based-testing` / `metamorphic-testing`
(`Property`) for — the exact pattern this analysis was asked to watch for
("every rendered skeleton", "every exported type", "round-trips"). One
Constraint-level inconsistency (FR-080-CON-2 vs FR-080-AC-1) and one borderline
Constraint (FR-076-CON-1) are noted at low severity.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
|----|----------|---------|------|------|
| FND-001 | medium | NFR-018's six Measurement rows all author Method as Rendered-tree scan a value absent from quoin catalog methods and from the four-class catalog Inspection Analysis Demonstration Test. Every other NFR in the repo NFR-001 through NFR-017 authors Method as one of the four classes. The NFRs own Verification section describes an automated gate with injected negative cases which is Test. | NFR-018-M-1, NFR-018-M-2, NFR-018-M-3, NFR-018-M-4, NFR-018-M-5, NFR-018-M-6 | wrong-requirement |
| FND-002 | medium | NFR-019's four Measurement rows author Method as Byte comparison two rows or Rendered-tree scan two rows neither of which is a catalog value or matches the sibling NFR convention of using Inspection Analysis Demonstration or Test. | NFR-019-M-1, NFR-019-M-2, NFR-019-M-3, NFR-019-M-4 | wrong-requirement |
| FND-003 | medium | FR-079-AC-1 AC-2 AC-3 and AC-6 quantify over every exported type or every rendered skeleton or no rendered artifact and quoin advise recommends property-based-testing Property for each via the universal property shape rule yet their Test Matrix rows TC-1420 TC-1421 TC-1423 and TC-1422 are typed Unit. | FR-079-AC-1, FR-079-AC-2, FR-079-AC-3, FR-079-AC-6, TC-1420, TC-1421, TC-1422, TC-1423 | wrong-requirement |
| FND-004 | medium | FR-079-AC-8 asserts every property of every exported model and every golden record round trips from its skeleton and quoin advise tags this obligations property shape as round-trip recommending metamorphic-testing or property-based-testing Property or golden-approval-testing Snapshot yet its Test Matrix row TC-1426 is typed Integration which names neither the property nor the snapshot shape. | FR-079-AC-8, TC-1426 | wrong-requirement |
| FND-005 | low | FR-079-AC-4 states every rendered negative fixture is refused and each for its own distinct reason a universal claim quoin advise recommends property-based-testing Property for yet its Test Matrix row TC-1424 is typed Integration. | FR-079-AC-4, TC-1424 | wrong-requirement |
| FND-006 | low | FR-080-CON-2 requires the rendered dev-quire command to name the tracking issue for the missing wheel and is authored Inspection while FR-080-AC-1 asserts the identical fact naming the provisioning command and the tracking issue and is authored Test verified by TC-1428. The same verifiable behavior is classified two different ways in the same requirement. | FR-080-CON-2, FR-080-AC-1, TC-1428 | wrong-requirement |
| FND-007 | low | FR-076-CON-1 requires the template to contain no copy of the schema emitter the Quire runtime or the semantic-core grammar and is authored Inspection while the sibling constraint FR-076-CON-2 no npmrc making the same shape of no rendered surface contains X claim is authored Test TC-1412 and verified by a scan. Confirm whether copy is a judgment call reserved for a reviewer or a scannable fact. | FR-076-CON-1, FR-076-CON-2, TC-1412 | wrong-requirement |

