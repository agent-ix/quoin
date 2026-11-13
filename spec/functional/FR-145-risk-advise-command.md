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

- When a user runs `quoin risk advise <scope> [--format markdown|json] [--module DIR]...`, the command SHALL print the three sections `policy_cells`, `evidence_gaps` and `decision_queue`, in that order.
- The command SHALL read only the safety-module records below `<scope>` (hazards, hazardous situations, risk estimates, controls, residual risks, benefit-risk records, overall evaluations), the `acceptability_policy` documents they reference, the requirement and test documents that link to a control, and the anomaly documents of FR-136.
- The command SHALL read the documents with the quire engine's reader and SHALL check each `risk_estimate`, `acceptability_policy`, `residual_risk`, `benefit_risk` and `overall_residual_risk_evaluation` against the `body_extraction` its type's module declares, taking the module from each `--module` root or, with none given, from the installed modules.
- If a type it must check is declared by no loaded module, then the command SHALL exit 2 naming the document and the type, and the message SHALL name `--module DIR` and say that the module version must include the `residual_risk`, `benefit_risk` and `acceptability_policy` types.
- The command SHALL take the edges of a record from the frontmatter `relationships` only; a link in a table cell or elsewhere in the body is no edge.
- If the reader cannot load a file below `<scope>`, then the command SHALL name the file and the reason on stderr and still render, and the file is absent from the report.
- Each distinct failure (unreadable scope, a module that does not load, a type no loaded module declares, a record that breaks its contract) SHALL exit 2 with its own diagnostic code.
- If an `acceptability_policy`, `residual_risk`, `benefit_risk` or `overall_residual_risk_evaluation` breaks that contract, then the command SHALL exit 2 naming the record and the engine's reason, and print no report, because dropping the record would drop an undecided decision or a policy without a word.
- The command SHALL skip a `risk_estimate` whose frontmatter `phase` is not `pre_control` or `post_control`, or that breaks the contract of its type (no judgement table, a placeholder level), and `quire validate` reports each such estimate as invalid (`agent-ix/spec-objects-safety` FR-203, FR-303-AC-5), so the command is silent about it by design.
- The command SHALL NOT write any file or any record.
- The command SHALL store no score, region, finding or verdict.
- The command SHALL render Markdown by default and one JSON object with `--format json`.
- The JSON object SHALL have exactly the keys `attribution`, `policy_cells`, `evidence_gaps` and `decision_queue`.
- Every `findings` array in the JSON SHALL hold objects with exactly the keys `finding` (the token) and `names` (the ids or values the token names, `[]` when it names none).
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
| FR-145-AC-7 | A `residual_risk` whose Determination table has an unknown decision word, a `benefit_risk` whose header is not the declared one, and an `acceptability_policy` with an unfilled placeholder level each make the command exit 2 naming the record, and print no report. A `risk_estimate` with a `phase` outside the two members, and one with no judgement table, are skipped and exit zero. | Test |

## Dependencies

- **Upstream**: [FR-102](ix://agent-ix/spec-objects-safety/FR-102), [FR-104](ix://agent-ix/spec-objects-safety/FR-104); [FR-136](./FR-136-anomaly-record-mapping.md).
- **Context only**: [FR-144](./FR-144-soup-report.md), whose report conventions this follows; [FR-305](ix://agent-ix/spec-objects-safety/FR-305).
- **Downstream**: [FR-146](./FR-146-risk-advise-policy-cells.md), [FR-147](./FR-147-risk-advise-evidence-gaps.md), [FR-148](./FR-148-risk-advise-decision-queue.md), [NFR-031](../non-functional/NFR-031-risk-advice-states-and-never-decides.md).
