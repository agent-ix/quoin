---
id: FR-031
title: "Catalog-driven verification-method advisor"
type: FR
relationships:
  - target: "ix://agent-ix/quoin/StR-004"
    type: "traces_to"
  - target: "ix://agent-ix/quire-rs/FR-054"
    type: "traces_to"
  - target: "ix://agent-ix/quoin/FR-030"
    type: "references"
---

# FR-031: Catalog-driven verification-method advisor

## Description

`quoin` SHALL recommend verification methods per obligation by matching the
**declared catalog's** applicability rules against facts about the requirement,
and SHALL flag where a recommendation disagrees with the authored method.

This is the capability the ADR-0011 mission names: help define the *testing
plan* for the software being specced, so the evidence store can then measure
that it happens.

### The proto-advisor was a prose table

`skills/spec-evidence-analysis` carried its own method list — `test | analysis |
inspection | demonstration` plus a handful of evidence kinds — declared in no
manifest and read by no code. The consequence was structural, not cosmetic:
`Verification` columns defaulted to `Test` by habit, and **nothing ever advised
DAST for an attack surface, monitors for a temporal property, or fault injection
for a reliability NFR**. Those recommendations were not wrong; they were
unreachable.

### Deterministic first, judgement second, and labelled

Rule matching is code. A method is recommended when any of its applicability
rules matches an observed fact, ranked by how many rules agree, with the method
id as a deterministic tiebreak — so the same obligation always yields the same
ordered advice.

Where no rule matches, the advisor reports **inconclusive** and stops. An
advisor that recommends `Test` because it found nothing is the habit being
replaced. LLM judgement belongs on that residue only, and is recorded as
judgement rather than as a verdict (the ADR-0010 discipline).

### Silence is not disagreement

A mismatch is only reported when the advisor had rules to go on. With nothing
matched, "the author chose `Test` and we recommend nothing" is silence — and
reporting it as a mismatch would bury the real ones under noise.

### An unobservable axis is skipped, not failed

The engine deliberately leaves the applicability axis set open (quire-rs
FR-054-CON-2), so a module may declare rules over axes this advisor has no facts
for. Such a rule contributes nothing rather than vetoing its method: that is a
gap in what can be observed, not grounds to drop a method a different rule
already matched.

### The catalog is read, never restated

`quoin catalog methods` merges the active modules' `verification_catalog`
first-wins — exactly as quire-rs merges it. If the two merges disagreed, the
advisor would recommend from one catalog while the auditor checked conformance
against another.

## Inputs

- The merged `verification_catalog` from the active modules
- Obligations from a validated `quire coverage --json` payload
- The FR-052 property shape, archetype and nearby object types, where available

## Outputs

- Per obligation: ranked recommendations with the rule and value that matched
  each, the normalized authored method, a mismatch flag, and an inconclusive flag
- `quoin catalog methods` — the merged catalog, human-readable or JSON
- A `SpecReview` with `analysis: evidence` carrying the obligations needing
  attention

## Behavior

- The advisor SHALL rank recommendations by matching-rule count, then by method
  id, so its output is reproducible.
- The advisor SHALL report the rule and value behind every recommendation. A
  recommendation whose reason is not shown cannot be argued with.
- The advisor SHALL normalize an authored cell before comparing it, so
  `Test (TC-707)` compares as `Test`.
- The advisor SHALL treat a match on a recommended method's **class** as
  agreement, because `Test` is a legitimate authored answer for any Test-class
  method.
- The advisor SHALL emit no finding for an obligation whose authored method
  agrees. A review listing every passing obligation is a review nobody reads to
  the end.
- Characteristic detection SHALL be lexical: every rule reads a fact about the
  text, never an inference about intent.

## Constraints

| ID | Constraint | Type | Validation |
|----|------------|------|------------|
| FR-031-CON-1 | The advisor SHALL NOT carry its own method list. Every method, class and evidence kind comes from module data; a restated table is the failure being closed. | Architecture | Test |
| FR-031-CON-2 | The advisor SHALL be advisory. It emits recommendations and flags; it changes no spec and blocks nothing. The human confirms, and the confirmed method is what the auditor later checks. | Architecture | Inspection |
| FR-031-CON-3 | The advisor SHALL NOT present an LLM conclusion as a deterministic result. Rule-matched recommendations and judged ones are distinguishable in the output. | Architecture | Inspection |

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-031-AC-1 | The merged catalog carries every declared method with its applicability rules intact, merges first-wins, reports a colliding id, and treats an undeclared catalog as empty. | Test (TC-129) |
| FR-031-AC-2 | An obligation whose spec declares an `attack_surface` object is recommended DAST, SAST and negative/abuse testing rather than defaulting to Test. | Test (TC-130) |
| FR-031-AC-3 | An obligation whose statement carries temporal phrasing is recommended runtime monitoring and model checking. | Test (TC-131) |
| FR-031-AC-4 | An obligation whose statement carries reliability phrasing is recommended fault injection. | Test (TC-132) |
| FR-031-AC-5 | A round-trip property shape is recommended property-based and metamorphic testing, and recommendations are ordered by matching-rule count. | Test (TC-133) |
| FR-031-AC-6 | An authored method no recommendation covers is flagged as a mismatch, and an authored method matching a recommended method's class is not. | Test (TC-134) |
| FR-031-AC-7 | An obligation matching no rule is reported inconclusive with no recommendations and no mismatch. | Test (TC-135) |
| FR-031-AC-8 | A method whose rule names an axis the advisor cannot observe is still recommended on the axes it can, with only the observable reasons reported. | Test (TC-136) |
| FR-031-AC-9 | A module whose `manifest.yaml` cannot be read or parsed is skipped and reported on the merged catalog, never thrown: a catalog missing one module's entries is still worth having, and the command that would crash is the one an operator runs to diagnose the module. | Test (TC-133) |
| FR-031-AC-10 | The advisor is reachable from a command: `quoin advise` derives the obligations from `quire coverage --json`, reads each criterion's FR-052 property shape from `quire properties --json`, and emits one recommendation set per obligation, with `--mismatch-only`, `--inconclusive-only` and `--json`. | Test (TC-150) |
| FR-031-AC-11 | A `quire properties` run that exits non-zero because some input document failed to resolve still contributes the shapes it did emit. Two untyped asset files must not cost the whole `property_shapes` axis. | Test (TC-150) |

## Dependencies

- **Upstream**: quire-rs [FR-054](ix://agent-ix/quire-rs/FR-054) (the catalog shape and merge), `spec-artifacts-process` FR-007 (the catalog content), [FR-029](./FR-029-consume-the-quire-json-contract.md) (the validated payload obligations arrive on)
- **Downstream**: [FR-030](./FR-030-evidence-store.md) (the confirmed method is what a recorded run discharges); the auditor (agent-ix/quoin#80) checks method conformance against the same catalog
