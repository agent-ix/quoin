---
id: SR-20261123190537
title: "Base review of SOUP promotion keeping SBOM supplier and licence"
type: SpecReview
analysis: base
scope: "agent-ix/quoin; spec/functional/FR-141-promote-sbom-components-to-soup-records.md, spec/functional/FR-041-sbom-inventory-evidence.md, spec/functional/FR-150-licence-and-ban-policy-findings.md, spec/spec.md"
review_set: subset
---

# Base review of SOUP promotion keeping SBOM supplier and licence

## Summary

Ticket: PLAT-1812 (PR agent-ix/quoin#1030). Base checklist over the spec-only change that makes `quoin soup promote`
write the SBOM-declared `supplier` and a single declared `license` onto created `soup_record` files, report
differences on existing records, and has the FR-041 reader expose the declared supplier. IDs are well formed and
contiguous (FR-141-AC-18..23, FR-141-CON-3/4, FR-041-AC-11), `quire validate` passes on the four changed files, no
ticket citation remains in the changed text (the PLAT-1515 reference was removed), and no standard text is quoted.
Checked against the reader in `rust/crates/quoin-evidence/src/adapters/sbom.rs`, the promotion planner in
`rust/crates/quoin-soup/src/soup/plan.rs`, and the owning module's `SoupRecord.json`, FR-009 and manifest on
`agent-ix/spec-objects-security` main.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Two FR-141 SHALLs contradict when the SBOM itself states the text `unknown` (a CycloneDX `supplier.name: "unknown"` or SPDX `licenseDeclared: "unknown"`): "write `supplier` ... as written" requires writing it, "SHALL NOT write the literal `unknown`" forbids it. The spec does not say which wins or whether the field is then absent and reported. | FR-141 Behavior (supplier/licence as written; no literal `unknown`), FR-141-AC-19 |
| FND-002 | low | FR-141-AC-21's "and the record still validates" (and AC-18's schema-validation clause) cannot fail on `supplier` or `license`: the owning module's `soup_record` body_extraction yields only id, title, type, purl and properties, so those frontmatter keys never reach `SoupRecord.json` under `quire validate`. The clause adds no check beyond the direct key assertions. | FR-141-AC-21, FR-141-AC-18 |
| FND-003 | low | The control-character omission applies to supplier and licence text, but only the supplier branch has an AC (AC-21); a licence containing a control character has no criterion. | FR-141 Behavior (control character in supplier or licence), FR-141-AC-21 |

## Verdict

Not mergeable as is, because the failure-domain review (separate SR) carries a high finding; this base lens alone
found one medium and two low defects. The design points the lead approved hold against the code and the owning
schema: supplier taken as written from CycloneDX `supplier.name` / SPDX `supplier`, absent/empty/NOASSERTION/NONE
mapped to absent; `license` written only for exactly one declared licence; created records never carry
`license_fact_ref`, so FR-009's `license`/`license_fact_ref` exclusivity holds by construction; existing records are
never modified, consistent with FR-141-AC-5's byte-identical rule and AC-7 idempotence. No other spec or doc in the
repository still states that supplier is not promoted. FR-150's added sentence is accurate.

## Examined

FR-141 Description, Behavior, AC-1..AC-23, CON-1..CON-4, Dependencies; FR-041 Description (licence and supplier
sections), AC-10, AC-11, CON-3; FR-150 Description change; spec/spec.md FR-141 line; NFR-030 (context);
spec-objects-security FR-009, FR-010, NFR-001, SoupRecord.json, soup_record skeleton and manifest body_extraction
(context).

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | Fix commit 19245c58 (round 1) |
| FND-002 | fixed | Fix commit 19245c58 (round 1) |
| FND-003 | fixed | Fix commit 19245c58 (round 1) |
