---
id: SR-20261113203416
title: "Spec review — FR-160 amendments in PLAT-1579 per-class evidence floors (quoin PR 880)"
type: SpecReview
analysis: base
scope: "agent-ix/quoin; spec/functional/FR-160-per-class-evidence-floors.md (amended), spec/functional/FR-161-per-class-obligations-report.md (context)"
review_set: subset
---

## Summary

Ticket: PLAT-1579. PR head `46d853e4`, diff `git diff origin/main...HEAD`. The spec change is FR-160
only: it states the policy file shape (`class_field` plus a `classes` mapping, with a YAML sketch),
says an unknown key is refused, adds "declares without an `evidence_kind`" to the refusal causes,
and widens the required-method join from "run `evidence_kind`" to "run or finding-shaped scan
record". `quire validate --scope . spec/functional/FR-160-per-class-evidence-floors.md` exits 0.

The join widening fits what FR-160 means. The floor asks for "at least one bound evidence
relationship" of the method's kind. A `FindingRecord` carries its own `evidence_kind` and is bound
evidence the FR-032 ladder already accepts. With a run-only join, a catalog method whose evidence
comes only from a scanner could never be met, so its floor would always fail. That is a defect the
amendment removes, not a looser rule. The new refusal for a method declared without an
`evidence_kind` follows FR-160's own rule ("a floor that silently does not apply reads as a passing
gate"): such a method's floor could never be met. The policy shape matches what the code
deserialises. No standard is named or quoted.

## Verdict

The amendments are justified and consistent with FR-160's intent. Two low wording gaps remain:
the "unknown key anywhere" sentence is wider than the code, and the tie case of "higher ratio wins"
is not stated.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | "An unknown key anywhere in the file is refused" is wider than the code. `ClassFloors.obligations` is `Vec<serde_json::Value>`, so anything under a class's `obligations` (FR-161's key) is accepted unchecked by `evidence audit` and `matrix`. FR-161-AC-8 refuses a bad entry only under `quoin class obligations`, so one file can pass the audit and later be refused by FR-161. Fix: limit the sentence to keys outside `obligations` (FR-161 validates those), or validate the entries here. | spec/functional/FR-160-per-class-evidence-floors.md:56; rust/crates/quoin-auditor/src/audit/class.rs:69 |
| FND-002 | low | "When a criticality floor applies as well, the higher ratio is evaluated and the finding names the source that set it" does not say what happens on a tie, when both sources set the same ratio. The code picks criticality (`floor_for`'s doc comment says so) and no AC or test covers the tie. Fix: state the tie rule in FR-160 (criticality on a tie) and cover it in FR-160-AC-5. | spec/functional/FR-160-per-class-evidence-floors.md:79; rust/crates/quoin-auditor/src/audit/scores.rs:229-252 |

## Review Scope

| Unit | Role | Result |
| --- | --- | --- |
| FR-160 policy file shape paragraph and YAML sketch | examined | Matches `ClassPolicy`/`ClassFloors` (deny_unknown_fields); FND-001 on the unknown-key sentence |
| FR-160 refusal paragraph (adds "declares without an evidence_kind") | examined | Justified by FR-160's silent-floor rule |
| FR-160 Required methods bullet (run or finding-shaped scan record) | examined | Justified, not an overreach (see Summary) |
| FR-160 Mutation floor bullet | examined | FND-002 |
| FR-160-AC-1..AC-13, CON-1, CON-2 | context_only | Unchanged; read for the amendment's consistency |
| FR-161 policy `obligations` entries, FR-161-AC-8 | context_only | Read for FND-001 |


## Disposition Round 1 Evidence

Replacement reviewer: `gpt-6-astra`; run `9118bc4b-9e8a-4c1f-9f5b-4d09820fcc63`; Codex session
`01a1d6e9-d820-7821-83d5-1b05bd42a929`. Sole reviewer for this disposition.
Ticket: PLAT-1579; PR: agent-ix/quoin#880. Reviewed pushed head
`fb4c9ab7ab65bdbcc5bcf0b97150b99d600d0a1b` and fixes `f2f627dd` and `fb4c9ab7`.
Final diff: `git diff origin/main...HEAD`, merge base `63acd34f8a571f4f51541433df86757360670b30`;
verified origin/main `2975c419cfefdf38745386e3b72c3bb63c3c0613` against the remote.
All 27 changed files and the three committed review artifacts were inspected.

Current outcome: **2 fixed; no open findings in this artifact.** No new findings. Original findings and prior review text above
are retained verbatim; the following rows are this round's current dispositions.

Verification: source, test-assertion, spec and pushed-commit inspection; `git diff --check
origin/main...HEAD` passed. No Cargo build/test, mutation run, computed matrix rerun or aggregate
gate ran in this pass. Earlier test/mutation claims above remain historical, not results for
this head. The aggregate gate belongs to the code lead. Tools: quoin `0.30.1`, quire `0.44.0`
(engine `0.59.1`), spec-artifacts-process `v0.31.0`. Validation uses this artifact's explicit
scratch path with `quire validate --scope .`.

## Dispositions

### Round 1

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | f2f627dd narrows the unknown-key statement to exclude obligations entries and assigns their validation to FR-161. This matches ClassFloors.obligations and FR-161-AC-8. |
| FND-002 | fixed | f2f627dd states criticality on a tie in both the mutation-floor description and AC-5; the new 0.7/0.7 test asserts the same source. No ratio or floor behavior was weakened. |

```yaml
dispositions:
- fnd: FND-001
  outcome: fixed
  reason: f2f627dd narrows the unknown-key statement to exclude obligations entries and assigns their validation
    to FR-161. This matches ClassFloors.obligations and FR-161-AC-8.
  after_path: spec/functional/FR-160-per-class-evidence-floors.md
  after_lines: 56-57
  after_excerpt: |-
    An unknown key anywhere in the file outside a class's `obligations` entries is refused; those entries
    belong to FR-161, which validates them.
- fnd: FND-002
  outcome: fixed
  reason: f2f627dd states criticality on a tie in both the mutation-floor description and AC-5; the new 0.7/0.7
    test asserts the same source. No ratio or floor behavior was weakened.
  after_path: spec/functional/FR-160-per-class-evidence-floors.md
  after_lines: 114-114
  after_excerpt: |-
    An obligation whose FR-053 criticality column is `high` and whose requirement carries no class gets only criticality floors; one whose requirement carries a class and whose column is empty gets only class floors; with both, the higher mutation ratio is evaluated and the finding names its source, and criticality when the two ratios are equal.
```
