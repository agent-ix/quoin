---
id: SR-20261116075214
title: "Scope-boundary analysis — PLAT-1766 measurement populations"
type: SpecReview
analysis: scope-boundary
scope: "agent-ix/quoin; MP-250, MP-251, MP-252; PR quoin#959"
review_set: subset
relationships:
  - target: ix://agent-ix/quoin/MP-250
    type: references
  - target: ix://agent-ix/quoin/MP-251
    type: references
  - target: ix://agent-ix/quoin/MP-252
    type: references
---

## Summary

Checked the boundary between the NFR-003 CLI argument-error populations and MP-252's ix-flow external-tool failures. The new routes belong to MP-250/251; MP-252 correctly remains unchanged.

## Verdict

**PASS** — no population boundary was crossed or broadened.

## Assurance Context

AP-201 applies to the declared measurement population. AP-202 does not. Active exceptions: none found. Unavailable context: no new empirical measurement result is claimed.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Coverage

MP-250/251 measure CLI commands crossed with argument failure classes enumerated from Quoin help. The new `formal-rules` and `formalize` routes add only the applicable unknown-argument cases to that boundary. MP-252 measures commands whose help advertises launching an ix-flow workflow crossed with twelve ix-flow failure modes. `formal-rules` is not such a workflow command, and the unchanged `tc_3004` independently checks the MP-252 set. No external-tool failure class or workflow boundary was changed.

## Examined scope

```yaml
scope:
  - id: MP-250
    path: spec/assurance/MP-250-nfr003-actionable-failures.md
    role: examined
    excerpt: "The population is the command x NFR-003 error-class enumeration."
  - id: MP-251
    path: spec/assurance/MP-251-nfr003-silent-failures.md
    role: examined
    excerpt: "The population is the same NFR-003 command/error-class enumeration."
  - id: MP-252
    path: spec/assurance/MP-252-nfr007-external-tool-failures.md
    role: examined
    excerpt: "The population is every workflow command crossed with each way ix-flow can fail."
  - id: FR-131-AC-9
    path: spec/functional/FR-131-percentage-metrics-over-enumerated-population.md
    role: examined
    excerpt: "NFR-003 metrics use command x error-class pairs enumerated from the binary's help text."
  - id: FR-131-AC-10
    path: spec/functional/FR-131-percentage-metrics-over-enumerated-population.md
    role: examined
    excerpt: "NFR-007 uses the workflow-command x ix-flow failure-mode enumeration."
  - id: tc_3004_the_nfr007_population_is_stated_by_failure_mode
    path: rust/crates/quoin-cli/tests/tc_3003_nfr_populations.rs
    role: examined
    excerpt: "The observed ix-flow failure modes equal the MP-252 set."
```
