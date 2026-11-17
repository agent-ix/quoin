---
id: FR-209
title: "Agent design-gate workflow for boundary-crossing rules"
type: FR
relationships:
  - target: "ix://agent-ix/quire-rs/FR-096"
    type: "depends_on"
  - target: "ix://agent-ix/spec-objects-security/FR-025"
    type: "depends_on"
  - target: "ix://agent-ix/spec-objects-architecture/FR-015"
    type: "depends_on"
  - target: "ix://agent-ix/quoin/FR-021"
    type: "depends_on"
---
# FR-209: Agent design-gate workflow for boundary-crossing rules

## Description

Quoin SHALL provide an ix-flow `design-gate` skill that lets an untrusted agent
propose and revise a data-flow design, evaluates each proposal with the static
flow-rule evaluator, and reserves acceptance for a person at a `hitl` gate.
The workflow SHALL use the evaluator and remediation data shipped by quire-rs
FR-096 and spec-objects-security FR-025. A passing design has exactly zero
denials in the evaluator's JSON report. Agent output, command text, and a run
transcript SHALL NOT establish a pass or acceptance.

The skill SHALL live at `skills/design-gate/SKILL.md`, with its workflow at
`skills/design-gate/workflows/design-gate/def.yaml`, following the installed
`skills/spec-review/` layout and Quoin's
`quoin.workflow-invariants/v1` provider. The workflow checker reads the
`quire.flow-report/v1` JSON produced by `quire relations --flow --json`.
Quoin SHALL NOT derive a verdict from console text or from the command's exit
status alone.

An accepted workflow state records a person's decision in the HITL gate. It is
not a flow-check pass, signed record, release approval, or analysis evidence.
Signed-record verification remains a separate optional capability. This
workflow SHALL NOT invoke that capability, enable signing for Quoin commits,
tags, or releases, configure signers, or request signing keys. No workflow
test requires or performs signing.

## Inputs

- A design scope containing architecture `part`, `connection`, and `data_sink`
  objects and their resolved `carries`, `crosses`, and `delivers_to` relations.
- The module set and design root that the run is to check.
- A scripted design agent that can propose or revise design artifacts from
  structured denial details. The agent is an untrusted proposer.
- A person who can accept or reject at the workflow's HITL gate.

## Outputs

- The workflow state and the last valid `quire.flow-report/v1` JSON report.
- On a denial, a machine-readable detail for each denial, derived from that
  report and supplied to the scripted agent for revision.
- A person's HITL decision when the run reaches a human gate.

The workflow SHALL NOT write a compliance claim, signed record, or analysis
evidence artifact. A transcript is a run log only.

## Behavior

### Evaluator binding

- The `dataflow_rules_pass` invariant SHALL invoke
  `quire relations --scope <design-root> --flow --json --module <module-root>`
  with one explicit `--module` for every configured rule module. Explicit
  module paths SHALL replace ambient module discovery.
- The invariant SHALL accept only a JSON object whose `schema` is exactly
  `quire.flow-report/v1` and whose `rules`, `facts`, and `denials` members have
  the shapes defined by quire-rs FR-096. It SHALL bind to the JSON members;
  it SHALL NOT scrape or interpret text output.
- The invariant SHALL pass only when `denials` is an empty array. A nonempty
  array SHALL fail as `dataflow_denied`, regardless of denial severity or
  evaluator exit status.
- For each denial, the agent detail SHALL include the report's `rule`,
  `check`, `subject`, and `matched` values; the matched data categories
  resolved from the referenced classification facts' `regulated_categories`;
  and every remediation's `name`, `description`, and `change` value as
  supplied by FR-025 and PLAT-1479. The workflow SHALL preserve the report's
  denial and remediation ordering and SHALL NOT invent or apply a remediation.
- If the evaluator cannot be launched, exits without a usable report, emits
  malformed JSON, reports another schema, or omits a required member, the
  invariant SHALL fail as `dataflow_check_unavailable`. It SHALL NOT pass,
  retry the design agent as though a denial had been found, or infer a result
  from stdout/stderr text.

### Rule-data modification guard

- At run start, the workflow SHALL record the contents and paths of the rule
  declarations that govern the selected design scope: module declarations of
  `required_relations` and `traceability.flow_rules`, and their declared rule
  data under the PLAT-1537 module contract.
- Before every evaluator invocation and before either acceptance transition,
  the invariant SHALL compare those paths and contents with the run-start
  values, including staged, unstaged, added, deleted, and renamed files.
- If a run changes any guarded rule declaration or rule-data file, the
  invariant SHALL fail as `rules_modified`; that run SHALL NOT pass even if
  the changed rules produce zero denials. The workflow SHALL return the
  guarded path list to the person and SHALL NOT ask the agent to revise the
  rules.

### Loop and human acceptance

- The workflow definition SHALL name its phases exactly `proposing`,
  `checking`, `revising`, `passed`, `escalated`, `accepted`, and `rejected`.
- The transitions SHALL be:
  - `proposing` → `checking` (auto).
  - `checking` → `passed` (auto), guarded by `dataflow_rules_pass`.
  - `checking` → `revising` (auto), when the valid report contains one or more
    denials; the denial details SHALL be supplied to the agent.
  - `revising` → `checking` (auto), after the agent submits a revised design.
  - `passed` → `accepted` or `rejected` (`hitl`).
  - `escalated` → `accepted` or `rejected` (`hitl`).
  - `accepted` and `rejected` SHALL be terminal phases.
- The workflow procedure SHALL count evaluator checks per run. The initial
  check is attempt one; `N = 3` means three retries after that initial
  attempt. After the fourth failed check, the workflow SHALL enter
  `escalated` without another agent retry. A zero-denial check reaches
  `passed` even if it is the fourth check.
- A failed evaluator invocation is unavailable, not a failed design attempt.
  It SHALL stop automatic revision and surface `dataflow_check_unavailable`
  for human handling; it SHALL NOT consume the retry allowance or pass.
- The workflow SHALL require an explicit person's decision at every
  transition into `accepted` or `rejected`, using `defaultGate: hitl`. The
  person SHALL see the last valid report, the unresolved denials if any, and
  whether the run reached `passed` or `escalated` before deciding.
- If a person accepts from `escalated`, the workflow SHALL retain the
  unresolved denials and SHALL continue to identify the flow result as
  denied; `accepted` SHALL identify only the person's decision and SHALL NOT
  be rendered as a passing evaluator verdict.
- The agent SHALL NOT accept its own proposal, acknowledge a HITL gate, alter
  a recorded human decision, or transition the run to `accepted` or
  `rejected`.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-209-AC-1 | Given a `quire.flow-report/v1` report with `denials: []`, `dataflow_rules_pass` passes; given any report with one or more denials, it fails as `dataflow_denied`, including when the denial severity is `warning` and the evaluator exits 0. | Test |
| FR-209-AC-2 | A seeded connection carrying a classification with `regulated_categories: [health]` across an `untrusted` boundary, with no protecting `encrypt-in-transit` control, produces a denial. The scripted agent receives the denial's rule, subject, matched classification and boundary, data category `health`, and `encrypt-in-transit` remediation name, description, and change. After the agent proposes the corresponding control, the next report has no denials and the run reaches `passed`. | Test |
| FR-209-AC-3 | A report with the wrong schema identifier, malformed JSON, missing `denials`, missing `matched` data, an unavailable evaluator, or a nonzero evaluator failure without a usable report fails as `dataflow_check_unavailable`; none reaches `passed` or is treated as a design denial. | Test |
| FR-209-AC-4 | If the run changes a guarded `required_relations` declaration, `traceability.flow_rules` declaration, or declared rule-data file in staged, unstaged, added, deleted, or renamed form, the workflow returns `rules_modified` naming the affected paths and never reaches `passed`, including when the modified rules produce zero denials. | Test |
| FR-209-AC-5 | A scripted agent that never resolves a denial receives four evaluator checks total: the initial proposal and three retries. After the fourth failed check the run reaches `escalated`, with no fifth agent retry. A scripted agent that resolves the denial on its fourth check reaches `passed`. | Test |
| FR-209-AC-6 | The workflow definition has exactly the phases and transitions in Behavior; every transition into `accepted` or `rejected` is gated by `hitl`, and neither terminal phase has an outgoing transition. | Test |
| FR-209-AC-7 | The agent cannot acknowledge a HITL gate or transition the workflow into either terminal phase. A run with no human acknowledgement remains at its prior phase. | Test |
| FR-209-AC-8 | A person accepting an escalated run leaves its valid denials in the retained report and the run output states both `accepted` for the human decision and `dataflow_denied` for the evaluator result; it does not state that the evaluator passed. | Test |
| FR-209-AC-9 | Running the scripted workflow tests creates no signature, signed commit, signer configuration, allowed-signers file, signing key, or release/tag configuration. The workflow and tests do not invoke signed-record verification. | Inspection |

## Dependencies

- **Upstream**: quire-rs FR-096 (`quire.flow-report/v1` and static flow-rule
  evaluation); spec-objects-security FR-025 (remediation vocabulary and
  `traceability.flow_rules`); spec-objects-architecture FR-015 (sink and
  `delivers_to` model); Quoin FR-021 (ix-flow invocation and workflow
  invariants).
- **Related capability**: Engineering Assurance FR-037 provides signed-record
  verification output for features that explicitly use it. Quoin
  [FR-188](./FR-188-formal-rule-approval-state-from-signed-commits.md) consumes
  that output for formal-rule approval. This workflow does not invoke that
  verifier or formal-rule approval state, configure signing, or require a
  signed record.
