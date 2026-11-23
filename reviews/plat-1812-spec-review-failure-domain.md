---
id: SR-20261123190539
title: "Failure-domain review of SOUP promotion keeping SBOM supplier and licence"
type: SpecReview
analysis: failure-domain
scope: "agent-ix/quoin; spec/functional/FR-141-promote-sbom-components-to-soup-records.md, spec/functional/FR-041-sbom-inventory-evidence.md"
review_set: subset
---

# Failure-domain review of SOUP promotion keeping SBOM supplier and licence

## Summary

Ticket: PLAT-1812 (PR agent-ix/quoin#1030). Unstated inputs and edge cases for writing SBOM supplier and licence
onto created records and reporting differences on existing ones, traced through the real reader
(`read_components`, `cyclone_dx_licences`, `spdx_licence`) and the promotion planner (`plan`). One case makes the
spec mandate a wrong record value; three leave the result undefined.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | The FR-041 reader exposes a CycloneDX `licenses` entry it cannot read (no `id`/`name`, empty or non-string text, e.g. `{"license":{"url":"https://example.org/l"}}` or `{"license":{"name":""}}`) as kind `expression` whose text is the JSON of the entry. FR-141 says exactly one declared licence is written "as written, for every licence kind", so promotion would write `license: "{\"license\":{\"url\":...}}"` — an SBOM fragment, not a licence the SBOM stated (FR-009 there refuses a key carrying SBOM document text). FR-141 cannot even tell this case apart: the reader gives it the same kind as a real expression, and FR-041 does not specify the fallback. FR-041 needs to expose it distinctly and FR-141 needs to leave `license` absent and report it. | FR-141 Behavior (exactly one declared licence), FR-041 licence section, FR-141-AC-18 |
| FND-002 | medium | A repeated purl is one record (merged duplicate), but the spec does not say whose supplier and licence it carries when the two components declare different ones (e.g. an SPDX document listing one purl twice with different `licenseDeclared`). The current planner keeps the first occurrence in document order; the spec neither states that nor reports the conflict, so the record silently carries one of two SBOM statements. | FR-141 Behavior (repeated purl writes one record), FR-141-AC-2 |
| FND-003 | medium | The supplier-difference statement covers "differs" and "the SBOM states one and the record states none", but not the converse: an existing record states a supplier and the SBOM states none. Whether that is a reported difference is left to the reader of "differs"; AC-22 does not test it. | FR-141 Behavior (existing record supplier difference), FR-141-AC-22 |
| FND-004 | medium | The licence-difference statement compares against "the single licence the SBOM states". For an existing record whose `license` is set while the SBOM now declares several licences, none, or one containing a control character, the spec does not say whether a difference is reported; AC-22 covers only the single-licence case. | FR-141 Behavior (existing record licence difference), FR-141-AC-22 |

## Verdict

Not mergeable until FND-001 is fixed; FND-002..004 should be settled in the same PR since each is one sentence
plus an AC clause. The rest of the failure surface is covered: absent, empty, whitespace-only, NOASSERTION and NONE
suppliers; several licences; control characters; `license_fact_ref` on existing records; and no-overwrite with
byte-identical files.

## Examined

FR-141 Description (supplier, licence, existing records), Behavior, AC-18..AC-23, CON-3, CON-4; FR-041 licence and
supplier sections, AC-10, AC-11; the FR-041 reader and FR-141 planner code (context); spec-objects-security FR-009
(context).
