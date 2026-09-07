---
id: PLAN-010
title: "Explicit ordered audit module selection"
type: Plan
relationships:
  - target: "ix://agent-ix/quoin/FR-032"
    type: "references"
  - target: "ix://agent-ix/quoin/FR-043"
    type: "references"
---

# PLAN-010: Explicit ordered audit module selection

## Objective

Implement the bounded #350 B1 prerequisite: `evidence audit` supplies the same
ordered explicit module set to native Quire coverage and method catalog loading.
The audit remains read-only and does not execute or manufacture test evidence.

## Requirements in scope

- FR-032-AC-12: repeatable selection, catalog/coverage agreement, unchanged
  omitted and single-module selection, and native error propagation.

## Execution and verification

1. Bank TC-1598..TC-1600 before changing production code. Keep the argument
   transport double explicitly separate from the real native Quire fixture.
2. Obtain independent review of the immutable specification/control bank.
3. Make the existing flag repeatable and pass its identical ordered array to
   the two existing consumers; do not introduce an alternate audit evaluator.
4. Run the native two-module fixture under poisoned ambient settings, healthy
   single/omitted controls, malformed/missing module controls and an unsupported
   producer control that proves no discovery retry occurs. The fixture states
   real criteria with no evidence store: both must remain undischarged, not
   falsely healthy. It is not a historical corpus or a fabricated native run.
5. Run focused audit/catalog tests, normal build, typecheck/lint and the existing
   canonical gates. Report source-lock blockers rather than changing acceptance
   data to obtain a passing result. Independent review precedes publication.

## Scope boundaries and dependencies

Native multi-module qualification uses the actual CLI #405/#409 capability,
not the JSON contract version floor as a substitute for feature support.
The existing historical QA external Quoin producer and its flags are unchanged.
Tier-2 answer-key v3, cohort pins, missing bindings and environment-route history
remain unchanged; a later B2 needs a reviewed versioned routing contract.
Tier-1 legacy multi-module discovery is outside this slice, so B1 does not claim
whole-stack exact module isolation. Existing catalog collision/readability policy
is not redesigned here.

No accepted lock, evidence, baseline, default installation, producer expectation
or dependency version may change. Source merges, reviewed candidate policy and
canonical replay remain separate promotion gates.

## Completion gate

The immutable implementation passes its discriminating native and transport
controls, preserves compatibility, and has independent review. Banking tests or
qualifying this prerequisite alone does not promote the verification stack.
