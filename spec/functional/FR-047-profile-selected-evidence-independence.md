---
id: FR-047
title: "Profile-selected evidence independence"
type: FR
relationships:
  - target: "ix://agent-ix/quoin/StR-004"
    type: "traces_to"
  - target: "ix://agent-ix/quoin/FR-030"
    type: "extends"
  - target: "ix://agent-ix/quoin/FR-046"
    type: "references"
---

# FR-047: Profile-selected evidence independence

## Description

Quoin records separation facts on the relationship between one obligation and one evidence
suite, then evaluates only the dimensions an AssuranceProfile selects for an exact
obligation. The available dimensions are actor, implementation/toolchain, technique, data
source, and review path.

An independence policy is a normalized projection of the profile rather than a second
project-wide criticality table. It names the profile, exact obligations, selected
dimensions, and rationale. Quoin refuses a policy naming an obligation the current Quire
payload does not contain; a typo or stale projection must not become a check that evaluates
nothing.

For one requirement to be satisfied, two distinct obligation-to-suite relationships must
state different non-empty values on **every** selected dimension. A second invocation of
the same evidence cannot satisfy the other side. Shared or missing lineage remains visible
in the assessment and prevents satisfaction only when the profile selected that dimension.

`quoin evidence record --lineage` transcribes the lineage JSON and assigns it to every
binding created by that run. The values are facts supplied by the caller, not a Quoin
verdict. `quoin evidence audit --independence-policy` opens an obligation whose selected
separation is absent. `quoin assurance --independence-policy` renders successful and
insufficient assessments as context; a successful separation assessment does not itself
make a claim supported.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-047-AC-1 | Quoin validates the five optional lineage dimensions strictly and requires at least one stated dimension when lineage is supplied. | Test (TC-303) |
| FR-047-AC-2 | Quoin validates a normalized policy with one profile, unique requirement ids, exact obligation ids, non-empty unique dimensions, and rationale; the commands refuse policy obligations absent from the current Quire payload. | Test (TC-303) |
| FR-047-AC-3 | One evidence relationship cannot satisfy both sides of an independence requirement; two distinct suites must differ on every selected dimension. | Test (TC-304) |
| FR-047-AC-4 | Shared and missing lineage values remain visible and prevent a false independent result when their dimension is selected. | Test (TC-305) |
| FR-047-AC-5 | With no independence policy, the auditor preserves its existing output and behavior. | Test (TC-306) |
| FR-047-AC-6 | A selected obligation receives an `insufficient-independence` finding until two qualifying evidence relationships exist, after which the finding clears. | Test (TC-307) |
| FR-047-AC-7 | Recording a run persists supplied lineage on each binding; a later run omitting lineage clears the old lineage rather than silently carrying it forward. | Test (TC-308) |
| FR-047-AC-8 | Assurance output names the profile, requirement, obligation, status, selected dimensions, stated values, and missing suites without treating independence as claim support by itself. | Test (TC-309) |

## Constraints

| ID | Constraint | Type | Validation |
|----|-----------|------|------------|
| FR-047-CON-1 | Quoin SHALL NOT infer independence from an organizational role, tool name, vendor, or the presence of two symbols. | Design | Inspection |
| FR-047-CON-2 | Quoin SHALL NOT apply an independence requirement when no profile projection requests one. | Design | Test (TC-306) |
| FR-047-CON-3 | Quoin SHALL NOT collapse the five dimensions into a score or a generic independent/not-independent badge detached from an obligation and policy. | Design | Inspection |
| FR-047-CON-4 | Quoin SHALL NOT parse AssuranceProfile Markdown; Quire or the authoring workflow owns projection into the normalized policy boundary. | Design | Inspection |

## Dependencies

- **Upstream**: [FR-030](./FR-030-evidence-store.md) supplies the binding relationship;
  [FR-046](./FR-046-evidence-producer-trust.md) keeps shared producer/configuration lineage
  visible for uses that rely on automated evidence.
- **Downstream**: [FR-040](./FR-040-assurance-case-view.md) renders the result; coding-agent
  and `ix-flow` workflows can supply the profile projection and human approval gate.
