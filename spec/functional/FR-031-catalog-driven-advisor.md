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
| FR-031-AC-12 | Every `characteristics` value the active catalog declares is producible by some fact source, or is listed as exempt with a reason. A value asked for and never produced fails the check by name, alongside the methods it strands. | Test (TC-247) |
| FR-031-AC-13 | No method keyed solely on `characteristics` is unreachable by every possible statement, unless it is declared unreachable with a reason. | Test (TC-248) |
| FR-031-AC-14 | Characteristics are read from a statement's prose: a markdown link's **target** contributes nothing, its text does. | Test (TC-249) |
| FR-031-AC-15 | `high-criticality` is minted from the obligation's own declared criticality value, never from a threshold the engine chose. | Test (TC-250) |
| FR-031-AC-16 | The join check carries no exemption for a `characteristics` value the active catalog no longer declares. | Test (TC-251) |
| FR-031-AC-17 | A statement naming a magic-value comparison, constant-time behaviour, or equivalence with a reference implementation mints the corresponding characteristic; a function signature and a recorded-snapshot comparison mint neither of the first or last. | Test (TC-252) |
| FR-031-AC-18 | An obligation with a binding and no fault-detection score mints `fault-detection-unmeasured`; one whose weakest bound score is below 1 mints `fault-detection-failed`; one with no binding, and one whose evidence was not consulted, mint neither. | Test (TC-253) |

> **CR-026 note (the advisor reads evidence — 2026-08-19):** `concolic-execution`
> was the last catalog method no requirement could elicit (CR-025 took that from
> 7 of 33 to 1). It was keyed on `path-sensitive` and `hard-to-reach-branch`,
> both properties of the *implementation's* control flow, which no specification
> states.
>
> **Nobody reaches for concolic execution up front.** It path-explodes and it is
> slow. The industrial pattern is hybrid fuzzing: fuzz until the coverage curve
> flattens, hand the stuck branches to a solver, feed the solved inputs back as
> seeds (Driller 2016, then QSYM, SymCC, Fuzzolic). It is an escalation from a
> stalled campaign, so the triggers had to come from somewhere other than a
> sentence.
>
> **Three are stated and two are observed.** A magic-value comparison — a
> checksum, CRC or HMAC — is the classic wall a fuzzer cannot climb and a solver
> clears in one step. Constant-time behaviour and reference equivalence are proof
> obligations an author writes down. And the evidence store answers the plateau
> question directly: `fault-detection-unmeasured` when an obligation is exercised
> and nothing measures whether the exercise discriminates,
> `fault-detection-failed` when something measured it and a seeded fault
> survived.
>
> **`advise` still performs no I/O.** The command opens the store and hands the
> answer in, exactly as `propertyShape` and `archetype` arrive. `scoresFor` is the
> auditor's, reused rather than reimplemented, so the auditor's finding and the
> advisor's recommendation cannot disagree about the same run.
>
> **A trigger shared with a cheaper method distinguishes nothing.**
> `structured-input` was among the triggers for one module release and was
> removed: it is what `grammar-based-fuzzing` is keyed on, so both were
> recommended on identical evidence with no basis to choose between them.
> **[RAN]** over quire-rs's 640 obligations: 17 of 18 concolic recommendations
> came from it, and dropping it took the count to **1** — the single genuine
> `reference-equivalence` hit. The count is not the argument; the argument is
> that the trigger separated nothing.
>
> **Deliberately absent: `fuzz-plateau`**, the literal Driller trigger. Nothing
> records coverage over time and a proxy invented to stand in for it is the
> CR-014 failure — an open set whose membership has to be judged rather than
> read.
>
> **What the advisor still cannot say** is that this is an escalation. Ranking is
> by matched-rule count, so a method of last resort ties with a unit test. Filed
> as agent-ix/quire-rs#190; the guidance lives in the `spec-evidence-analysis` and
> `spec-fuzz` skills until the catalog can carry it.

> **CR-025 note (the catalog/fact join — 2026-08-19):** `advise` matches two
> things written by different people in different places — the **catalog**
> (module data declaring the values that trigger each method) and the **fact
> set** (engine code producing values from an obligation). **Nothing compared
> them.**
>
> `matchRules` skips an unknown *axis* deliberately (FR-054-CON-2 leaves the
> axis set open). It does not skip an unknown *value* on a known axis — that
> value simply never matches. And `inconclusive` is already a legitimate
> outcome, so an advisor that could never recommend `integration-testing` was
> indistinguishable from one that had nothing to say.
>
> **What the numbers count.** Distinct `characteristics` values declared by the
> installed catalog's 33 methods, against the values any quoin code path can
> produce:
>
> | | |
> |---|---|
> | declared, never producible | **40 of 60 → 9** |
> | methods unreachable by every statement | **7 of 33 → 1** |
>
> The second is the one that matters: it counts what an author actually loses.
> `integration-testing` and `mutation-testing` were both in the seven.
>
> **Two false-positive causes were found by reading flagged documents, not by
> inspecting the regexes.** Of 13 statements matching `stakeholder`, **12
> matched the directory name inside a link target** (`../stakeholder/StR-005…`)
> — so link targets are now stripped before matching, which cleans the twenty
> pre-existing characteristics too. Of 11 matching `safety`, **10 were
> `path-safety`** and none were the 25010 characteristic; this ecosystem uses
> the word as a suffix for memory/path/type safety, a different concept from
> freedom from harm, so the bare word is gone and `hazard`/`harm`/`injur`
> remain. Neither narrowing was made because the count was high — both were
> made because the regex was demonstrably naming something else.
>
> **`io-boundary` fires on 51 statements and that is correct.** 27 are driven by
> `stdout`, and for a CLI, stdout *is* the I/O boundary: "produces parseable
> JSON on stdout", "byte-identical stdout". A high count over a CLI-heavy corpus
> is a fact about the corpus, not a bad rule.
>
> **The 9 that remain are declared, not silenced**, each with a reason: four are
> facts `advise()` is never given (the trace graph, the bundle, the evidence
> store, the test environment), four are properties of the implementation's
> control flow or of testability that no author writes down, and one
> (`judgement-required`) is a second spelling of the working
> `no-executable-oracle`. `concolic-execution` remains the single unreachable
> method and awaits a decision: retire it, or key it on something a
> specification states.
>
> **`high-criticality` is honest about firing on nothing.** It is minted from
> the obligation's own value (`P0`/`high`/`critical`) rather than from a
> threshold this code picked — CR-008 deleted a hardcoded `["P0"]` for that
> reason. Measured earlier in this programme: 2,304 of 2,304 `Acceptance
> Criteria` tables carry no criticality column, so it mints for nothing today.
> `mutation-testing` is reachable in principle and unreached in practice, and
> the join check now shows that instead of hiding it.

## Dependencies

- **Upstream**: quire-rs [FR-054](ix://agent-ix/quire-rs/FR-054) (the catalog shape and merge), `spec-artifacts-process` FR-007 (the catalog content), [FR-029](./FR-029-consume-the-quire-json-contract.md) (the validated payload obligations arrive on)
- **Downstream**: [FR-030](./FR-030-evidence-store.md) (the confirmed method is what a recorded run discharges); the auditor (agent-ix/quoin#80) checks method conformance against the same catalog
