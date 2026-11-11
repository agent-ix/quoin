---
id: SR-20261111192911
title: "Spec review of quoin PR 810 (FR-133 discharge facts in the evidence store)"
type: SpecReview
analysis: base
scope: "agent-ix/quoin@40f2c3c; spec/functional/FR-133-discharge-facts-in-the-evidence-store.md, spec/matrix.md, spec/functional/index.md, spec/spec.md"
review_set: subset
---

# Spec review of quoin PR 810

## Summary

Ticket: PLAT-308. FR-133 was authored spec-first in the same PR, and no separate spec review
exists, so this is that review. It covers integrity, EARS form, failure domains and an overclaim
check, scoped to the FR-133 file and its matrix, index and spec.md rows.

FR-133 states one SHALL in its Description and has seven atomic, testable ACs, each with a TC row.
The design rule "facts persisted, obligations re-derived" is stated as a Constraint. FR-046 is
unchanged, and the spec makes no claim about the retracted #272 report baseline. FR-133
deliberately drops the config digest, source revision and environment that the ticket text asked
for. That is correct under FR-044-AC-13 and the no-provenance rule.

## Verdict

**PASS WITH FINDINGS (two medium, two low).** The failure domains for identifier bounds, partial
writes and duplicate clauses in the binding input are not specified. AC-1 mandates two stored
fields that nothing reads.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-133-AC-5 states `discharged + Σ dispositions + open == binding` without saying where the operands come from. It does not state that binding clause ids must be unique, nor that the report is derived from the live binding report and not from a caller-supplied discharge report. As written, a binding report with a repeated clause id satisfies the AC while overstating `binding` and `discharged`. Bind AC-5 to the live binding report and add a refusal for duplicate clause ids | spec/functional/FR-133-discharge-facts-in-the-evidence-store.md:73 |
| FND-002 | medium | The Behavior bullet "each path component encoded so it cannot escape its directory" covers traversal only. It gives no length bound and no case-collision rule for authority, id, version or clause. Contrast FR-130-AC-10/11, which bound scenario ids at 64 bytes and refuse case collisions. "A refused list writes nothing" covers only predicate refusals, so an I/O failure partway through the list leaves a partial write that no AC addresses. Add bounds, a case-collision refusal and an all-paths-checked-before-first-write rule | spec/functional/FR-133-discharge-facts-in-the-evidence-store.md:41-45 |
| FND-003 | low | FR-133-AC-1 requires storing the recording instant and the recording tool and version, but no behaviour reads either. They appear in no report and no criterion uses them, so they fail the value test as provenance fields. AC-1 also cites FR-030-AC-17, which governs the `--tool` identity of a transcribed suite run, not quoin's own version. It cites FR-044-AC-13, which governs measurement-collection digests, not the evidence store. Drop the two fields from AC-1, or name the report that shows them, and fix the citations | spec/functional/FR-133-discharge-facts-in-the-evidence-store.md:36-39, spec/functional/FR-133-discharge-facts-in-the-evidence-store.md:58-61 |
| FND-004 | low | `spec/matrix.md` marks FR-133 "Covered" and TC-2600..2606 "Complete". TC-2605's ceiling refusal for `assurance.inward_completeness` and TC-2602's malformed-fact and bad-instant cases have no test (gap-analysis FND-002/004). The row overstates what is backed | spec/matrix.md:139, spec/matrix.md:1390-1396 |

## Examined scope

```yaml
clean: false
scope:
- {id: FR-133, path: spec/functional/FR-133-discharge-facts-in-the-evidence-store.md, role: examined, excerpt: "Quoin SHALL persist the attested discharge facts of FR-046 in the evidence store and SHALL derive the discharge partition and an inward-completeness report from a live clause-binding-v1 report plus the stored facts."}
- {id: FR-133-AC-1, path: spec/functional/FR-133-discharge-facts-in-the-evidence-store.md, role: examined, excerpt: "A recorded fact is stored with its clause set key, recording instant and recording tool and version, and the stored file holds no other clause of the binding report and no force or expected outputs."}
- {id: FR-133-AC-2, path: spec/functional/FR-133-discharge-facts-in-the-evidence-store.md, role: examined, excerpt: "The discharge partition for a live binding report uses the stored facts of its clause set; the same stored fact discharges a clause before its attestation expires and leaves it open after."}
- {id: FR-133-AC-3, path: spec/functional/FR-133-discharge-facts-in-the-evidence-store.md, role: examined, excerpt: "Recording a clause again replaces its stored fact, and a fact list the FR-046 predicates refuse (a duplicate clause, a malformed fact, a bad instant) writes nothing and exits as a bad request."}
- {id: FR-133-AC-4, path: spec/functional/FR-133-discharge-facts-in-the-evidence-store.md, role: examined, excerpt: "A stored file that is not valid JSON, or whose clause set or clause differs from its path, is an error naming the file, never an open clause."}
- {id: FR-133-AC-5, path: spec/functional/FR-133-discharge-facts-in-the-evidence-store.md, role: examined, excerpt: "The inward-completeness report states binding, discharged, dispositions by decision and the open remainder by clause with its reason, such that discharged + Σ dispositions + open == binding; unresolved applicability and unused facts are counted outside binding; the report carries no score."}
- {id: FR-133-AC-6, path: spec/functional/FR-133-discharge-facts-in-the-evidence-store.md, role: examined, excerpt: "evidence.record_discharge, evidence.discharge_report and assurance.inward_completeness answer through the production runtime with absolute store paths, and each refuses a request over its ceiling before reading it."}
- {id: FR-133-AC-7, path: spec/functional/FR-133-discharge-facts-in-the-evidence-store.md, role: examined, excerpt: "quoin evidence record-discharge followed by quoin discharge --repo ... --inward reports the recorded fact as discharged and the remaining clause as open."}
- {id: FR-046, path: spec/functional/FR-046-clause-discharge.md, role: context_only, excerpt: "FR-046-AC-2 Every binding clause appears exactly once under direct evidence, approved disposition, or open work, and the result contains no score."}
- {id: FR-130-AC-10, path: spec/functional/FR-130-protocol-scenario-evidence-intake.md, role: context_only, excerpt: "scenario id over 64 bytes ... refused as invalid-identifier"}
```
