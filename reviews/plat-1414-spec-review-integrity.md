---
id: SR-20261112024903
title: "Integrity review of quoin PR 828 (FR-136 Anomaly record mapping)"
type: SpecReview
analysis: integrity
scope: "agent-ix/quoin@b92fb76; spec/functional/FR-136-anomaly-record-mapping.md"
review_set: subset
---

# Integrity review of quoin PR 828

## Summary

Ticket: PLAT-1414. Checked FR-136 for consistency with the requirements it declares
`depends_on` (quoin FR-121, and FR-104 through FR-121; spec-artifacts-process FR-014 and FR-015
in PR #139). FR-136 names no exception to FR-121, so FR-121's rules apply to it as written. On
four points FR-136 states the opposite, and two of those contradict FR-121's own acceptance
criteria.

## Verdict

**FAIL, two high and three medium findings.** As written, FR-136 cannot be implemented on the
FR-121 mapping it depends on. Either it states and justifies each departure from FR-121, or it
changes to follow FR-121: lifecycle keys in body sections, recordSources, and the FR-104 refusals.
FR-136 agrees with the module on the vocabularies, table headers, and section names.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | FR-136 maps frontmatter `status` and `safety_impact` to record keys. FR-121, which FR-136 depends on, says "Quire SHALL NOT read a record key from frontmatter", and FR-121-AC-5 asserts that a frontmatter `status: accepted` yields no `status`. FR-121's rationale explains why: lifecycle keys need a body locus for attribution. FR-136 neither states an exception nor gives a reason for one. Either declare the departure in FR-121, with a reason, or carry these keys by a mechanism FR-136 defines | spec/functional/FR-136-anomaly-record-mapping.md:39-40; spec/functional/FR-121-record-key-sections-mapping.md:76,124 |
| FND-002 | high | FR-136 says an unresolved `Target` keeps the entry with `target` unresolved and leaves the other rows unaffected (AC-4). FR-121 takes `SemanticId` cells "by the Target rules of FR-104, with the same refusals". Under FR-104, a bare id that is no bundle artifact is the error `target-not-id`, and under FR-121 any row error makes the key `unavailable` with no value. FR-136 also gives no representation for an "unresolved" `SemanticId`. Either define the override and the unresolved representation, or follow FR-121 and rewrite AC-4 to expect the key `unavailable` | spec/functional/FR-136-anomaly-record-mapping.md:44-46,63; spec/functional/FR-121-record-key-sections-mapping.md:100,105; spec/functional/FR-104-relationships-mapping.md:127 |
| FND-003 | medium | FR-136 requires each `history` entry to carry its 1-based line. FR-121 seals the record (`unevaluatedProperties: false`) and says "locus never rides inside it": locus goes in `recordSources`. Either move the line to `recordSources` (per row, which FR-121 does not yet provide), or state why `history` is an exception | spec/functional/FR-136-anomaly-record-mapping.md:38; spec/functional/FR-121-record-key-sections-mapping.md:60 |
| FND-004 | medium | FR-136 maps `## Safety Impact Assessment` to the record key `assessment`. FR-121 matches a heading to a key by normalized name, so this heading reaches `safetyimpactassessment`, not `assessment`. Neither FR-014 nor FR-015 names the `Anomaly` record keys or the data schema that declares them. Under FR-121 the key set comes from the module's schema. Either name the key the module schema declares, or state the heading-to-key rule FR-136 uses in place of FR-121's | spec/functional/FR-136-anomaly-record-mapping.md:41; spec/functional/FR-121-record-key-sections-mapping.md:70 |
| FND-005 | medium | For a kind mismatch FR-136 reports a separate kind-mismatch diagnostic. FR-015's Behavior says a target "of another kind SHALL fail validation as a dangling reference", while FR-015-AC-6 says "as a kind mismatch", so the module contradicts itself and FR-136 takes one side. FR-136 also does not say whether a mismatched entry keeps a resolved `target`, keeps an unresolved one, or is dropped. Align the classification with the module (fix FR-015 in PR #139) and state the entry's state on a mismatch | spec/functional/FR-136-anomaly-record-mapping.md:47-48,64 |

## Examined scope

```yaml
clean: false
scope:
- {id: FR-136, path: spec/functional/FR-136-anomaly-record-mapping.md, role: examined, excerpt: "Behavior bullets lines 36-54 checked against FR-121, FR-104, FR-014, FR-015"}
- {id: FR-136-AC-4, path: spec/functional/FR-136-anomaly-record-mapping.md, role: examined, excerpt: "row targeting FR-999 yields one dangling-reference diagnostic and an entry with unresolved target; the other rows are unaffected"}
- {id: FR-136-AC-5, path: spec/functional/FR-136-anomaly-record-mapping.md, role: examined, excerpt: "A Hazard row targeting FR-001 yields a kind-mismatch diagnostic at the row."}
- {id: FR-136-AC-7, path: spec/functional/FR-136-anomaly-record-mapping.md, role: examined, excerpt: "no assessment section extracts no assessment key and reports it not_applicable (consistent with FR-121 absent-section rule)"}
- {id: FR-121, path: spec/functional/FR-121-record-key-sections-mapping.md, role: context_only, excerpt: "Quire SHALL NOT read a record key from frontmatter."}
- {id: FR-104, path: spec/functional/FR-104-relationships-mapping.md, role: context_only, excerpt: "a bare token that is no artifact id in the bundle ... target-not-id"}
- {id: FR-015, path: "agent-ix/spec-artifacts-process#139 spec/functional/FR-015-anomaly-safety-impact-assessment.md", role: context_only, excerpt: "A Target that resolves to nothing, or to an artifact of another kind, SHALL fail validation as a dangling reference."}
```

## New findings (disposition pass 1)

Checked at f1ad73c against spec-artifacts-process#139 at 0956b64 (FR-011, FR-014, FR-015).

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-006 | medium | FR-136 says "a document that fails module validation still yields its record". spec-artifacts-process FR-011, which FR-136 now depends on, says "The mapping SHALL be strict: any reported error means no record is built for that document". A schema-only failure such as a missing `closed_by` (AC-2) may build a record, but a mapping error such as a malformed assessment table cannot, under FR-011. Narrow the statement to record-validation failures and state what `quoin anomaly check` does when FR-011 builds no record | spec/functional/FR-136-anomaly-record-mapping.md:53-54, :101 |
| FND-007 | medium | FR-015 makes FR-136 the owner of "report one missing-module diagnostic ... and SHALL NOT report each `Hazard` row as dangling". FR-136 specifies only the `anomaly.missing-module` emission, and leaves dangling references to `quire validate` (quire-rs FR-026). With `spec-objects-safety` absent, every hazard id resolves to nothing, so `quire validate` still reports each `Hazard` row as dangling, and FR-136 cannot suppress that. Either state the per-row suppression in the owner of reference resolution (a quire-rs change), or drop the "SHALL NOT report each row" clause from FR-015 | spec/functional/FR-136-anomaly-record-mapping.md:78, :85-86; spec-artifacts-process FR-015 Behavior |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | - |
| FND-002 | fixed | - |
| FND-003 | fixed | - |
| FND-004 | fixed | - |
| FND-005 | fixed | - |
| FND-006 | fixed | - |
| FND-007 | fixed | - |
