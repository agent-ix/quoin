---
id: SR-2026112016223475846355002
title: PLAT-1790 PR 1012 code-review
type: SpecReview
analysis: code-review
scope: agent-ix/quoin; 14 files in PR 1012
review_set: subset
---

## Summary

Changes requested. The policy gate checks each verified adjacent pair, emits the version-2 reason and preserves ordinary claim rules. The review found a signed-pair consistency defect and a mismatch in the three-way target join.

## Verdict

FAIL at 002201ce523323d2e6e17cd3107a8660ac082358.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | The host retains parsed run and projection values and later reopens both files to authenticate their content. A concurrent replacement makes the signature check and the qualified result describe different bytes. Authenticate the same byte buffers used to parse the retained pair, or derive the evaluated pair from the verified immutable objects. Preserve the unavailable/refusal cause when that correspondence cannot be established. | rust/crates/quoin-cli/src/analysis_evidence.rs:557-565 |
| FND-002 | medium | With design_coverage present, identity_count checks the selected coverage locus using same_native_target, which drops the line hint. It then discards that candidate. Binder and result matching compares only the displayed design row and the evidence item. An explicit disagreement between coverage and design lines can therefore still display a qualified item. Keep the unique coverage locus and require that all supplied line hints from the three participating loci agree. The CLI now derives its own rows from the same walk, but matrix.build also accepts these projections separately. | rust/crates/quoin-assurance/src/matrix.rs:600-612 |

## Coverage

Plan completion: not assessed

The full Quire matrix has 2,236 rows, including 1,402 tagged, 642 untagged, 150 method-without-symbol, 36 retired and 6 tagged only by ignored tests. These totals describe the repository and do not make unrelated backlog part of this PR. FR-065-AC-25 has both new policy tests, FR-115-AC-16 the join test and FR-153-AC-8 the CLI refusal. No todo!/unimplemented! introduced by the changed production paths. Reviewed policy, status and selection behavior maps to FR-065, FR-115, FR-116 and FR-153. No whole-repository completeness or live-evidence claim.

| Swap or distinction | Evidence on this candidate |
| --- | --- |
| Actor | #1004 covers signed pair, unsigned/untrusted evidence, different commit and target authorization; independent change of actor name against the same verified principal is missing. |
| Report bytes | Input parsing/size, verifier result and commit differences have separate tests. Neither the CLI case nor #1004 changes only the accepted retained projection bytes; the loader/authentication read split matters. |
| Criterion | Synthetic admitted report credits one criterion only. New matrix tests reject a same-ID foreign document, a conflicting item line and ambiguous coverage rows. The actual CLI checks the sibling stays absent. See code-review FND-002 for the design-vs-coverage conflict. |
| Snapshot | The Quire owner has independent selected-corpus and root-change checks producing selection-mismatch. The candidate Quoin CLI case uses one inventory; it does not substitute a second snapshot independently. |
| Revision | A synthetic bridge case isolates a wrong performed revision. The shipped CLI reports wrong-revision and confirms locator and exact diagnostic for differing supplied revisions. |
| Selection | The CLI refuses a nonexistent record selector. Quire tests independently cover foreign selection and the Quoin bridge tests formal rule selections. A different existing owner selection is not handed through the shipped CLI in these seven checks. |
| Claimed or qualified | The recorded pass is bound in the audit yet stays unevaluated in the CLI; the sibling stays absent. Synthetic bridge fixtures cover a qualified record, including trace lookup. A live positive remains blocked by VER-17 and STD-717. |

The coder recorded the focused policy regression on base ea86b0e2 as actual Valid versus expected Invalid. I inspected the final-head logs for seven passing focused tests (2 receipt, 1 target join, 3 matrix compatibility, 1 CLI) and successful targeted Clippy across four packages. I separately ran the four-package cargo fmt check through locked-build.sh and git diff --check. No live CI was run. The test inputs do not supply a live FR-093 decision or TC-006 corpus.

## Review scope

- rust/crates/quoin-assurance/src/case/leaf.rs
- rust/crates/quoin-assurance/src/matrix.rs
- rust/crates/quoin-assurance/tests/tc_1080_matrix.rs
- rust/crates/quoin-change-assurance/schemas/verification-receipt-v2.schema.json
- rust/crates/quoin-change-assurance/src/model/outcome.rs
- rust/crates/quoin-change-assurance/src/records.rs
- rust/crates/quoin-change-assurance/src/verify/approval.rs
- rust/crates/quoin-change-assurance/src/verify/mod.rs
- rust/crates/quoin-change-assurance/tests/tc_1511_claimed_approval.rs
- rust/crates/quoin-cli/src/analysis_evidence.rs
- rust/crates/quoin-cli/src/matrix.rs
- rust/crates/quoin-cli/tests/tc_ver80_differential_report_command.rs
- rust/crates/quoin-quire/src/design_matrix.rs
- rust/crates/quoin-quire/src/evidence.rs

FR-065-AC-25 (examined)

> | FR-065-AC-25 | Given a verified same-record predecessor chain, each adjacent `required: true` to `false` transition, or each widening of `signature_kinds` or of a non-empty `approvers` set to empty or a wider set while both policies require approval, makes review invalid with `policy_weakened` and null `claimed_approval`, including when the latest approved event carries no claim; an empty `approvers` set followed by an empty set is unchanged. Moving from optional to required is permitted strengthening, but with no `verified_approval` the result remains incomplete with `approval_missing`. A t

FR-115-AC-16 (examined)

> | FR-115-AC-16 | With FR-153 analysis evidence supplied, each Analysis criterion receives only result items whose exact FR-078 target matches its identity; qualified claims appear as `qualified-record`, and every non-qualified cause retains its exact category, locator and unmet-premise detail. JSON and text show the same complete item set; no item is joined by suite, symbol or digest alone, and the legacy auditor-derived `evidence_status` is unchanged. | Test |

FR-153-AC-8 (examined)

> | FR-153-AC-8 | A bound `pass` entry with a healthy `quoin evidence audit` and no FR-116 decision is not counted: no `qualified-record` status and no `record_binders` claim. | Test |

FR-153-AC-12 (examined)

> | FR-153-AC-12 | `quoin matrix` and derived `quoin assurance` accept repeatable `--run-observation FILE` inputs using one `quoin.analysis-run-observation/v1` object per canonical run-record path; the inputs supply CI association, performed checkout SHA and actor-authority evidence without adding those fields to FR-030 run records. Both views expose the selected analysis result, FR-078 category, record locators and each unmet premise in their JSON and text output using the additive quoin-assurance output members defined by FR-115 and FR-040; the quire-rs FR-078 wire contract remains unchanged.

FR-153-AC-13 (examined)

> | FR-153-AC-13 | An observation for another record cannot qualify the selected analysis result; the selected entry, obligation and declared method come from the canonical record and catalog; absent observations remain `unevaluated`; and no observation is reused by suite, symbol or request digest. | Test |

FR-153-AC-14 (examined)

> | FR-153-AC-14 | Missing or malformed `selected_head` or `checkout_sha` remains `unevaluated` for performed revision; differing valid values yield `wrong-revision`; conflicting supplied `selected_head` values yield `record-conflict`; none produces `qualified-record`. | Test |

FR-153-AC-16 (examined)

> | FR-153-AC-16 | Only an FR-116 `ActorAdmission` with string `actor` and `admission_ref` and string-array `authorized`, whose actor equals the complete FR-036 signature result's non-empty `signer.principal` for exact commit C with `kind` `commit`, `status` `signed_valid`, and `signer.mapping` `mapped`, and whose authorization includes the selected obligation, satisfies actor authority; the host resolves C's full Git object id and requires `signature_result.verified_commit_id` to equal it. C's first-parent tree (or empty tree for a root commit) has neither selected canonical path and C adds bot

FR-153-AC-17 (examined)

> | FR-153-AC-17 | Invalid observation-file JSON/protocol, empty required `protocol` or `record`, wrongly typed or incomplete members, unknown members, empty/duplicate obligation ids, duplicate record selectors or selectors outside the selected canonical store are refused without a report that claims positive qualification; a missing or invalid retained projection/run pair also cannot qualify; omitted observation files leave each affected result `unevaluated`. | Test |

FR-116-AC-18 (examined)

> | FR-116-AC-18 | The result exposes the host-admission and bounded authority basis for every positive decision; an unadmitted self-asserted context cannot obtain a positive decision merely by copying the expected field values. | Test |

## Rust lane

Reviewed the Rust-specific boundary and serde behavior, error reason and receipt schema, ownership, and resource considerations. Eight source/test boundary groups across the four crates retain the existing wire shapes except for the explicitly required receipt reason. The scope is governed by repository Rust idioms, not scoring or Jev AssuranceProfiles.
