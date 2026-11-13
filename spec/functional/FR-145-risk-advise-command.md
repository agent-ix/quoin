---
id: FR-145
title: "quoin risk advise: a read-only advisory report that guides a risk decision without making it"
type: FR
relationships:
  - target: "ix://agent-ix/spec-objects-safety/StR-001"
    type: "traces_to"
  - target: "ix://agent-ix/spec-objects-safety/FR-104"
    type: "depends_on"
  - target: "ix://agent-ix/spec-objects-safety/FR-102"
    type: "depends_on"
  - target: "ix://agent-ix/spec-objects-safety/FR-305"
    type: "references"
  - target: "ix://agent-ix/quoin/FR-136"
    type: "depends_on"
  - target: "ix://agent-ix/quoin/FR-030"
    type: "depends_on"
  - target: "ix://agent-ix/quoin/FR-144"
    type: "references"
---

# FR-145: quoin risk advise: a read-only advisory report that guides a risk decision without making it

## Description

Tracked by Linear PLAT-1651 (report sections PLAT-1652, PLAT-1653, PLAT-1654). `quoin risk advise` reads the
risk records of a repository and the policies they reference, and prints guidance for the person who decides a
residual risk. It is a view, as the SOUP report of FR-144 is: it persists nothing, adds no record type and
changes no record.

It is separate from `quoin risk-summary` (`agent-ix/spec-objects-safety` FR-305), which lists the chain per
hazard and situation and shows each estimate's `policy_region`. This report is organised per estimate and per
undecided record, and it adds what FR-305 does not carry: the policy cell with the region's flag and the reason
a region is missing (FR-146), the evidence gaps and open anomalies behind each post-control estimate (FR-147),
and the queue of undecided decisions (FR-148). Where this report and FR-305 both show a value, the value is the
same, because both read the same links and the same policy.

Acceptance of a residual risk is a human decision (`agent-ix/spec-objects-safety` FR-104). The report names the
policy as the source of every region and the named person as the source of every decision, and it offers no
verdict. The wording and no-verdict rules that bind all three sections are NFR-031.

## Behavior

- When a user runs `quoin risk advise <scope> [--format markdown|json]`, the command SHALL print the three sections `policy_cells`, `evidence_gaps` and `decision_queue`, in that order.
- The command SHALL read only the safety-module records below `<scope>` (hazards, hazardous situations, risk estimates, controls, residual risks, benefit-risk records, overall evaluations), the `acceptability_policy` documents they reference, the requirement and test documents that link to a control, the retained evidence records of the evidence store (FR-030), and the anomaly documents of FR-136.
- The command SHALL NOT write any file, any evidence-store entry or any record.
- The command SHALL store no score, region, finding or verdict.
- The command SHALL render Markdown by default and one JSON object with `--format json`.
- The JSON object SHALL have exactly the keys `attribution`, `policy_cells`, `evidence_gaps` and `decision_queue`.
- The `attribution` string SHALL state that regions come from the project's policy and that decisions come from the named person.
- If `<scope>` contains no risk record, then the command SHALL print the three sections empty and the attribution, and exit zero.
- The command SHALL exit zero whenever it renders, whatever the sections contain.
- For identical inputs, each format SHALL render byte-identical output.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-145-AC-1 | Over a fixture chain, `--format json` parses as one object whose keys equal `attribution`, `policy_cells`, `evidence_gaps` and `decision_queue` exactly. | Test |
| FR-145-AC-2 | After the command runs over a fixture repository, no file is created, changed or removed, including under the evidence store. | Test |
| FR-145-AC-3 | Two runs over the same inputs give byte-identical Markdown and byte-identical JSON. | Test |
| FR-145-AC-4 | Over an empty scope the command exits zero and prints the three sections empty with the attribution. | Test |
| FR-145-AC-5 | The attribution string names the policy as the source of regions and the named person as the source of decisions. | Test |
| FR-145-AC-6 | The JSON holds no `hazard`/`situation` row-per-pairing structure and no `harms` key; the only chain ids are those named by the key tables of FR-146 to FR-148. | Test |
| FR-145-AC-7 | A fixture with an evidence-store record naming a criterion is read, and the same fixture with the store removed gives the gap of FR-147-AC-2, showing the store is on the read list. | Test |

## Dependencies

- **Upstream**: `agent-ix/spec-objects-safety` FR-102, FR-104; [FR-136](./FR-136-anomaly-record-mapping.md); [FR-030](./FR-030-evidence-store.md).
- **Context only**: [FR-144](./FR-144-soup-report.md), whose report conventions this follows; `agent-ix/spec-objects-safety` FR-305.
- **Downstream**: [FR-146](./FR-146-risk-advise-policy-cells.md), [FR-147](./FR-147-risk-advise-evidence-gaps.md), [FR-148](./FR-148-risk-advise-decision-queue.md), [NFR-031](../non-functional/NFR-031-risk-advice-states-and-never-decides.md).
