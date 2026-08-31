---
id: PLAN-006
title: "Governed graph adapters and portfolio"
type: Plan
status: active
relationships:
  - target: "ix://agent-ix/quoin/StR-007"
    type: references
  - target: "ix://agent-ix/quoin/FR-066"
    type: references
  - target: "ix://agent-ix/quoin/FR-067"
    type: references
---

# PLAN-006: Governed graph adapters and portfolio

## Requirements Summary

### Stakeholder Requirements

- [ ] **StR-007:** Assurance owners inspect identity-complete retained graph
      evidence without producer execution or aggregate verdicts.

### Functional Requirements

- [ ] **FR-066:** Exact versioned adapters validate Quire assurance exports and
      graph-quality observations, retaining raw bytes and constructing governed
      measurement collections.
- [ ] **FR-067:** Portfolio reporting preserves graph history, partitions,
      provenance, availability, compatible comparisons, and injected FR-062 views.

## Dependency Graph

- `FR-044 + producer contract fixtures + FR-062 input seam -> FR-066`
  because adapter output is a current measurement collection and the Quire
  adapter must hand the authoritative export to the graph-analysis owner.
- `FR-045 + FR-062 + FR-066 -> FR-067` because the portfolio extends the
  existing report with accepted collections and existing graph report objects.
- FR-062 is owned by Quoin #152. #281 defines only the narrow consumer seam and
  must bind to #152's exported types when stable; it must not publish a competing
  graph model or parse Quire/spec files.

### Cross-cutting constraints

- No adapter or report path executes a producer, Quire, Git, network, suite, or
  scorer operation, and portfolio construction performs no write.
- Canonical identities, attachment digests, population identities, availability
  states, and partition boundaries are preserved rather than inferred.

### The seams

FR-066 attaches under `src/measurement/` beside the existing plan validation and
atomic collection store. FR-067 extends `src/measurement/portfolio.ts` through
pure input/output functions. `src/commands/report.ts` parses mappings and injects
the stable FR-062 analyzer; all graph traversal stays in #152.

## Test Plan

### Adapter unit and property tests

- [ ] **TC-1293:** exact versioned registry and unknown-adapter refusal.
- [ ] **TC-1294:** Quire premise tuple acceptance/refusal before record exposure.
- [ ] **TC-1295:** field-for-field Quire handoff through the injected FR-062 seam.
- [ ] **TC-1296:** closed graph-quality schema and canonical observation id.
- [ ] **TC-1297:** exact scorer-byte digest and raw attachment retention.
- [ ] **TC-1298:** independently required invocation-attestation fields.
- [ ] **TC-1299:** active plan id/definition premise checks.
- [ ] **TC-1300:** bijective census normalization.
- [ ] **TC-1301:** bijective measured-result normalization and duplicate refusal.
- [ ] **TC-1302:** empty/unreadable/unsupported not-computed states.
- [ ] **TC-1303:** idempotence, collision refusal, and direct-intake compatibility.
- [ ] **TC-1304:** static no-execution/no-frontmatter boundary.

### Portfolio unit, property, and integration tests

- [ ] **TC-1305:** current graph-quality plan, provenance, population, and digests.
- [ ] **TC-1306:** complete readable history and incompatible historical records.
- [ ] **TC-1307:** partition preservation under permutations with no aggregation.
- [ ] **TC-1308:** availability states remain distinct from measurement state/zero.
- [ ] **TC-1309:** premise-by-premise comparison compatibility.
- [ ] **TC-1310:** raw record/scorer digest resolution in every view.
- [ ] **TC-1311:** byte-equivalent injected FR-062 reports and absent-input states.
- [ ] **TC-1312:** repository-local and collection-local corruption isolation.
- [ ] **TC-1313:** deterministic mapping/seed/store permutations and conflict refusal.
- [ ] **TC-1314:** historical/non-graph compatibility and no verdict.
- [ ] **TC-1315:** static read-only and consume-without-recompute boundary.
- [ ] **TC-1316:** retained producer-to-portfolio stakeholder flow.

## Remaining Work

### Track A: Critical Path (serial)

- **A1 = TASK-033** Adapter red contract suite — exit: TC-1293..TC-1304 fail only for absent behavior.
- **A2 = TASK-034** Adapter implementation — exit: exact contract intake constructs validated collections with no execution.
- **A3 = TASK-036** Pure portfolio implementation — exit: graph history and comparisons preserve every premise and localize failures.
- **A4 = TASK-037** Report command and FR-062 wiring — exit: parsed mappings inject #152 report objects without graph reconstruction.
- **Gate = TASK-038** Review and traceability — pass: TC-1293..TC-1316, full gates, code review, and mechanical gap analysis are clean.

### Track B: Parallel (independent)

- **B1 = TASK-035** Portfolio red contract suite — exit: TC-1305..TC-1315 fail only for absent graph portfolio behavior.

## Parallel Execution Summary

```text
Track A: TASK-033 -> TASK-034 ---------> TASK-036 -> TASK-037 -> TASK-038
Track B:             TASK-035 ---------/
External:                     #152 stable export ---------^
```

## Task File Mapping

| Task     | Track | Owns           | Verified by                                 | Status      |
| -------- | ----- | -------------- | ------------------------------------------- | ----------- |
| TASK-033 | A     | FR-066         | TC-1293..TC-1304                            | not_started |
| TASK-034 | A     | FR-066         | TC-1296, TC-1300, TC-1303                   | not_started |
| TASK-035 | B     | FR-067         | TC-1305..TC-1315                            | not_started |
| TASK-036 | A     | FR-067         | TC-1305, TC-1307..TC-1310, TC-1312..TC-1314 | not_started |
| TASK-037 | A     | FR-066, FR-067 | TC-1295, TC-1311, TC-1315                   | not_started |
| TASK-038 | Gate  | StR-007        | TC-1316                                     | not_started |

## Coordination Rules

- `src/measurement/graph-adapters.ts` is single-writer under TASK-034;
  `src/measurement/graph-portfolio.ts` is single-writer under TASK-036.
- TASK-037 consumes #152's stable exports only after their owner confirms them;
  until then, tests use the narrow injected seam without minting public graph types.
- No work crosses into Quoin #286, agent-skills, Filament, or quire-code-rs.
- No push, PR, issue/board mutation, or merge is part of this local plan.
