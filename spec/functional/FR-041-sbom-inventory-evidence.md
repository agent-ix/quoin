---
id: FR-041
title: "SBOM inventories as run evidence"
type: FR
relationships:
  - target: "ix://agent-ix/quoin/StR-004"
    type: "traces_to"
  - target: "ix://agent-ix/quoin/FR-033"
    type: "extends"
  - target: "ix://agent-ix/quoin/FR-040"
    type: "requires"
---

# FR-041: SBOM inventories as run evidence

## Description

The verification catalog declares `sca-sbom` and nothing could discharge it. `quoin` SHALL read
CycloneDX and SPDX inventories through the FR-033 adapter registry, so a supply-chain obligation has a
discharge path.

### The record shape was a real question, and FR-040 answered it

`agent-ix/quoin#116` was **deliberately deferred** — not dropped — because the shape depended on a
consumer that did not exist:

> If the answer is *"an SBOM's presence at HEAD discharges a supply-chain obligation"*, this is a thin
> freshness record and barely needs an adapter. If the answer is *"the assurance case argues over the
> SBOM's contents"*, the evidence is the cross-product of the SBOM and an advisory scan.

FR-040 is that consumer, and its answer is legible in what it renders. **An argument evidence leaf is one
line per obligation** — statement, supported-or-open, and the auditor's reason. It renders no
component list, and could not usefully: an assurance case is read by a person, and a thousand-row
inventory inside an argument is not an argument.

So the claim an SBOM supports is *"a complete inventory was produced at this commit"* — a **run
record**. Contents-level judgement, *component X at version Y carries advisory Z*, is finding-shaped
and already lands through `cargo-audit` and SARIF (FR-034).

**There is no third record type**, which is the outcome the deferral was protecting: building the
intake before the consumer existed is how a record type gets a shape nothing needs.

### One entry per component, and the reason is vacuity

The obvious alternative is a single entry carrying the component count in `score`. It is wrong, and
the way it is wrong is instructive: **an SBOM listing nothing would be indistinguishable from a
healthy one** without a new check to notice.

As entries, an empty inventory produces zero entries, and `vacuous-evidence` (FR-034) names it with
no new machinery — *a tool that ran and found nothing* is precisely what that check exists to catch.
The general rule earns its keep here: model the thing so the existing checks apply, rather than
summarise it and then re-derive what the summary lost.

### Identity is the purl, from wherever the format keeps it

CycloneDX carries `purl` at the component; SPDX keeps it in
`externalRefs[referenceType = purl]`. Reading `name` alone would give the same component two
identities depending on which tool produced the document, and a binding is only as good as the
identity it names.

Where a component carries neither purl nor name it is **dropped, not given one**. A fabricated symbol
binds to nothing *and* inflates the count that proves the inventory is non-empty — it would make an
unreadable SBOM look healthier than an empty one.

### An unrecognised document is refused

Zero entries is a real finding about the consumer's build. A file the adapter could not read must not
be able to masquerade as one, so a document that is neither CycloneDX nor SPDX raises rather than
returning nothing.

### quoin does not generate the SBOM

The consumer's CI produces it (ADR-0011 invariant 1). This reads what was produced.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-041-AC-1 | A CycloneDX document yields one passing entry per component, identified by purl. | Test (TC-231) |
| FR-041-AC-2 | An SPDX document is read through its purl external refs, not its package names. | Test (TC-232) |
| FR-041-AC-3 | An inventory listing nothing yields no entries, so the existing vacuity check names it. | Test (TC-233) |
| FR-041-AC-4 | A document that is neither format is refused, not read as empty. | Test (TC-234) |
| FR-041-AC-5 | A component with neither purl nor name is dropped rather than given an invented identity. | Test (TC-235) |
| FR-041-AC-6 | The adapter is selected by `--adapter sbom` and by the tools that emit these formats. | Test (TC-236) |

## Constraints

| ID | Constraint | Type | Validation |
|----|-----------|------|------------|
| FR-041-CON-1 | quoin SHALL NOT generate an SBOM. The consumer's CI produces it (ADR-0011 invariant 1). | Design | Inspection |
| FR-041-CON-2 | quoin SHALL NOT introduce a record type for inventories. An SBOM is a run record; advisory findings are FR-034's. | Design | Inspection |
| FR-041-CON-3 | quoin SHALL NOT judge the inventory's contents. Whether a component's version or licence passes is the auditor's and the consumer's gate policy. | Design | Inspection |
| FR-041-CON-4 | The adapter SHALL NOT declare an `evidenceKind`. The catalog and the suite registry already carry that vocabulary. | Design | Test (TC-231) |

## Dependencies

- **Upstream**: [FR-033](./FR-033-evidence-format-adapters.md) (the registry), [FR-040](./FR-040-assurance-case-view.md) (the consumer whose rendering settled the record shape), [FR-034](./FR-034-finding-shaped-evidence.md) (where advisory findings land)
- **Downstream**: none. This closes the adapter family opened by `agent-ix/quoin#91`.
