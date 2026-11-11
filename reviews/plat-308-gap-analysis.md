---
id: SR-20261111192910
title: "Gap analysis of quoin PR 810 (FR-133 coverage)"
type: SpecReview
analysis: base
scope: "agent-ix/quoin@40f2c3c; FR-133 criteria, spec/matrix.md, rust/crates/quoin-assurance/src/discharge, rust/crates/quoin-core, rust/crates/quoin-cli"
review_set: subset
---

# Gap analysis of quoin PR 810

## Summary

Ticket: PLAT-308. Plan completion: not assessed (planless). `quoin matrix --json`, run with the
PR-built binary, binds a test to every FR-133 criterion, AC-1 to AC-7. Each binding carries the
row's TC id, and nothing is reported as `criterion-tagged-outside-row`. The "undischarged" status
only means no run evidence has been recorded, as for every other criterion. `quoin validate`
reports "repository QA gates: no findings".

The ids FR-133 and TC-2600..2606 do not exist on origin/main. They do not collide with open PRs
#806 (TC-2300..2302), #807 (FR-131, TC-2400..2407), #808 (FR-132, TC-2500..2506) or #809
(TC-2700..2705). Those PRs will conflict as text in `spec/matrix.md` and `spec/functional/index.md`,
because they all append after FR-130. No production code in the diff lacks an owning requirement.

## Verdict

**PASS WITH FINDINGS.** The main path is backed. The adverse cases that AC-5, AC-3, AC-4 and
AC-6 name are only partly tested.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-133-AC-5's adverse populations are untested. The `unused_facts` count is never asserted, and every fixture leaves it at 0. No completeness test covers an expired or not-yet-valid stored fact (expiry is asserted only on the partition). No test covers a duplicated clause in the binding report or an inconsistent supplied report. Both of those break the identity (code-review FND-001/002). The identity is asserted only on the two happy fixtures | rust/crates/quoin-assurance/src/discharge/completeness.rs:213-251, rust/crates/quoin-core/tests/tc_458_evidence_boundary.rs:680-730 |
| FND-002 | low | FR-133-AC-3 names three refusals: a duplicate clause, a malformed fact and a bad instant. Both the unit test and `tc_2602` exercise only the duplicate. The malformed-fact and bad-`recordedAt` cases, each writing nothing, are untested | rust/crates/quoin-assurance/src/discharge/store.rs:344-367, rust/crates/quoin-core/tests/tc_458_evidence_boundary.rs:733-748 |
| FND-003 | low | FR-133-AC-4 has two halves: a stored file whose clause or whose clause set differs from its path. Only the clause half (a file moved to SYN-002) and corrupt JSON are tested. A record whose `clauseSet` differs from its directory is untested | rust/crates/quoin-assurance/src/discharge/store.rs:369-383 |
| FND-004 | low | FR-133-AC-6 says each of the three ops refuses a request over its ceiling. `tc_412` adds the two `evidence.*` ops, but no test drives `assurance.inward_completeness` over `MAX_BUILD_CASE_BYTES`. The matrix row still marks TC-2605 Complete | rust/crates/quoin-core/src/ops/evidence/tests.rs:384-393, spec/matrix.md:1395 |

## Examined scope

```yaml
clean: false
scope:
- {id: FR-133-AC-1, path: spec/functional/FR-133-discharge-facts-in-the-evidence-store.md, role: examined, excerpt: "A recorded fact is stored with its clause set key, recording instant and recording tool and version, and the stored file holds no other clause of the binding report and no force or expected outputs."}
- {id: FR-133-AC-2, path: spec/functional/FR-133-discharge-facts-in-the-evidence-store.md, role: examined, excerpt: "The discharge partition for a live binding report uses the stored facts of its clause set; the same stored fact discharges a clause before its attestation expires and leaves it open after."}
- {id: FR-133-AC-3, path: spec/functional/FR-133-discharge-facts-in-the-evidence-store.md, role: examined, excerpt: "Recording a clause again replaces its stored fact, and a fact list the FR-046 predicates refuse (a duplicate clause, a malformed fact, a bad instant) writes nothing and exits as a bad request."}
- {id: FR-133-AC-4, path: spec/functional/FR-133-discharge-facts-in-the-evidence-store.md, role: examined, excerpt: "A stored file that is not valid JSON, or whose clause set or clause differs from its path, is an error naming the file, never an open clause."}
- {id: FR-133-AC-5, path: spec/functional/FR-133-discharge-facts-in-the-evidence-store.md, role: examined, excerpt: "The inward-completeness report states binding, discharged, dispositions by decision and the open remainder by clause with its reason, such that discharged + Σ dispositions + open == binding; unresolved applicability and unused facts are counted outside binding; the report carries no score."}
- {id: FR-133-AC-6, path: spec/functional/FR-133-discharge-facts-in-the-evidence-store.md, role: examined, excerpt: "evidence.record_discharge, evidence.discharge_report and assurance.inward_completeness answer through the production runtime with absolute store paths, and each refuses a request over its ceiling before reading it."}
- {id: FR-133-AC-7, path: spec/functional/FR-133-discharge-facts-in-the-evidence-store.md, role: examined, excerpt: "quoin evidence record-discharge followed by quoin discharge --repo ... --inward reports the recorded fact as discharged and the remaining clause as open."}
bindings:
- {test_id: a_recorded_fact_stores_the_fact_and_no_obligation, ac_id: FR-133-AC-1, trace: correct}
- {test_id: the_report_partitions_the_live_binding_with_the_stored_facts, ac_id: FR-133-AC-2, trace: correct}
- {test_id: recording_again_replaces_the_fact_and_a_refused_list_writes_nothing, ac_id: FR-133-AC-3, trace: correct}
- {test_id: tc_2602_a_refused_fact_list_exits_bad_request_and_writes_nothing, ac_id: FR-133-AC-3, trace: correct}
- {test_id: a_stored_file_at_the_wrong_path_is_an_error_not_an_open_clause, ac_id: FR-133-AC-4, trace: correct}
- {test_id: the_populations_add_up_to_the_binding_clauses_and_the_remainder_is_named, ac_id: FR-133-AC-5, trace: correct}
- {test_id: nothing_discharged_is_all_open_and_renders_the_remainder, ac_id: FR-133-AC-5, trace: correct}
- {test_id: tc_2605_recorded_discharge_facts_feed_a_live_partition_and_the_completeness_report, ac_id: FR-133-AC-6, trace: correct}
- {test_id: tc_2606_recorded_facts_are_read_back_by_the_discharge_command, ac_id: FR-133-AC-7, trace: correct}
```
