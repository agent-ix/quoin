---
id: SR-135
title: "Code review — risk decisions PR #1048"
type: SpecReview
analysis: code-review
scope: "agent-ix/quoin; rust/crates/quoin-soup/src/soup/advisory.rs, rust/crates/quoin-soup/src/soup.rs, rust/crates/quoin-soup/src/soup/plan.rs; FR-142, FR-141"
review_set: subset
relationships:
  - { target: "ix://agent-ix/quoin/FR-142", type: references }
  - { target: "ix://agent-ix/quoin/FR-141", type: references }
---

## Summary

Reviewed the Rust changes and their surrounding producer behavior for PR #1048, including the SOUP promotion path. The shared-clause join rule creates a confirmed regression for multi-component promotion with `--discharges`.

## Verdict

**FAIL** — a normal multi-component promote writes the same clause on every new record, and the changed join then leaves every attached advisory unresolved as shared.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | Multi-component `soup promote --discharges` makes all newly created records share one clause; the new join rule makes each advisory on those records unresolved. | rust/crates/quoin-soup/src/soup/advisory.rs:602-608; rust/crates/quoin-soup/src/soup.rs:266; rust/crates/quoin-soup/src/soup.rs:542 |

## Scope Examined

- FR-142-AC-37 and its implementation in `resolve_for`.
- `soup promote` argument handling, the clause passed to every `write_new` call, and `plan::render`'s `disposition_clause` output.
- Rust review of changed advice, dossier, anomaly, and SOUP code and associated test changes; no other defect found.

## Verification

The producer stores the same `clause` for each created record. The new join counts all records with that clause and returns `ClauseShared` when the count exceeds one, before consulting the discharge fact. Thus a valid shared fact cannot resolve any finding after a multi-component promote. This is PLAT-1861. The build and test gates were attempted but blocked by missing generated `node_modules/@agent-ix/semantic-core/generated/json-schema`; `cargo deny check` was blocked by the read-only Cargo advisory database path.


## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-002 | medium | A verified signer mismatch is shown in the attribution row but is not converted to the DOS-RISK decision_signer_mismatch gap. | rust/crates/quoin-dossier/src/risk_management.rs:473-482; FR-214-AC-8; FR-199-AC-19 |

The decision_gaps match handles DecidedByDiffersFromCommitAuthor but omits DecidedByDiffersFromVerifiedSigner. With verifier options configured, FR-214 replaces the author comparison with the verified-principal comparison, so an actual decider/principal mismatch remains in the table and produces no typed dossier gap.

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | The producer now refuses multi-component --discharges before writing any records, so the shared-clause join mismatch cannot be created through this CLI path. |
| FND-002 | still-open | decision_gaps() does not map DecidedByDiffersFromVerifiedSigner to DecisionSignerMismatch, so configured signature verification can reveal a mismatch without the corresponding DOS-RISK gap. |

Fixed excerpt for FND-001 (source: `rust/crates/quoin-soup/src/soup.rs:279`):

```rust
if arg(arguments, "discharges").is_some() && planned.created.len() > 1 {
```
