---
id: FR-189
title: "Formalize-requirement workflow"
type: FR
relationships:
  - target: "ix://agent-ix/quoin/StR-007"
    type: "traces_to"
  - target: "ix://agent-ix/quoin/FR-188"
    type: "depends_on"
  - target: "ix://agent-ix/quoin/FR-021"
    type: "depends_on"
---

# FR-189: Formalize-requirement workflow

## Description

Quoin SHALL ship an ix-flow workflow that drafts a QSL formalization for a
requirement, rejects drafts that fail QSL check or contradict approved rules
before any person reviews them, and reaches `approved` only through a `hitl`
gate whose invariant requires the [FR-188](./FR-188-formal-rule-approval-state-from-signed-commits.md)
`approved` state. Origin: VER-98.

The skill is `formalize-requirement`, laid out like `skills/spec-review/`
(`SKILL.md` plus `workflows/<name>/def.yaml`), with
`ix-flow-invariant-provider` set to `quoin workflow-invariants`. Invariants are
served by the existing `quoin.workflow-invariants/v1` provider
([FR-021](./FR-021-launch-ix-flow-runs.md)).

Analysing real QSL drafts in the `analysed` phase depends on QSL-1900
(QSL emitting `analysis_claim` nodes and a contract package); this FR defines
nothing of it.

## Behavior

- The workflow definition SHALL name its phases exactly `drafting`, `checked`,
  `analysed`, `awaiting_approval`, `approved`, `rejected` and `stopped`.
- The transitions SHALL be:
  - `drafting` → `checked` (auto), guarded by invariant `qsl_check_passes`.
  - `checked` → `analysed` (auto), guarded by invariant
    `analysis_consistent_with_approved`.
  - `analysed` → `awaiting_approval` (auto).
  - `awaiting_approval` → `approved` (`hitl`), guarded by invariant
    `formal_rule_approved`.
  - `awaiting_approval` → `rejected` (`hitl`).
  - A failure in `checked` or `analysed` → `drafting`, incrementing a retry
    counter and carrying the diagnostics.
  - `drafting` → `stopped` (terminal) after 3 failed attempts; the 4th
    consecutive failure ends in `stopped`.
- Invariant `qsl_check_passes` SHALL pass when QSL `check` on the draft file
  exits clean (quire-driver `quire-cli`, `Operation::Check`).
- The workflow SHALL guard `checked` → `analysed` with invariant `analysis_consistent_with_approved`, defined by VER-100, which
  passes only when the draft does not contradict the approved rules.
- Invariant `formal_rule_approved` SHALL pass only when
  `quoin formal-rules status` reports `approved` for every symbol the run
  drafted.
- The workflow and the agent SHALL NOT create the approval. The ix-flow gate
  acknowledgement is never an approval, and an agent identity never satisfies
  it (precedent: engineering-assurance FR-062, FR-048). The `hitl` gate pauses
  for the person; the invariant passes only after the person's signed commit
  exists.
- The workflow definition SHALL define no transition into `approved` other than
  the `hitl` gate.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-189-AC-1 | Given a scripted drafting step that yields a rule that passes QSL check, the run reaches `awaiting_approval` | Test |
| FR-189-AC-2 | Given a draft that fails QSL check, the run returns to `drafting` with the diagnostics and an incremented retry counter | Test |
| FR-189-AC-3 | Given a drafting step that fails check on four consecutive attempts, the run ends in `stopped` | Test |
| FR-189-AC-4 | Given a run at `awaiting_approval` whose gate is acknowledged and no signed approval commit exists, advancing to `approved` fails invariant `formal_rule_approved` | Test |
| FR-189-AC-5 | Given the same run after a person's signed approval commit lands, advancing to `approved` passes | Test |
| FR-189-AC-6 | The `def.yaml` holds no transition into `approved` other than the one with the `hitl` gate | Inspection |
| FR-189-AC-7 | The phase names and transitions in `def.yaml` equal the set fixed above | Inspection |
| FR-189-AC-8 | Given a commit signed by an agent identity not in the allowed-signers file, invariant `formal_rule_approved` fails | Test |
| FR-189-AC-9 | `def.yaml` names `quoin workflow-invariants` as the `ix-flow-invariant-provider` and the provider answers each of the three invariants | Test |

## Dependencies

- **Upstream**: [FR-188](./FR-188-formal-rule-approval-state-from-signed-commits.md),
  [FR-021](./FR-021-launch-ix-flow-runs.md), VER-100 (the analysis invariant),
  QSL-1900 (precondition for analysing real QSL drafts).
