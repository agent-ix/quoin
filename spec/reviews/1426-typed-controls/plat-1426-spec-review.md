---
id: SR-20261113140423
title: "Spec review of typed control and clause-set output (FR-171 to FR-174, NFR-038)"
type: SpecReview
analysis: base
review_set: subset
scope: "agent-ix/quoin; PR #871 (git diff origin/main...plat-1426-typed-controls-spec at 20e50a4a): spec/functional/FR-171, FR-172, FR-173, FR-174, spec/non-functional/NFR-038, both index files"
---

# Spec review of typed control and clause-set output (FR-171 to FR-174, NFR-038)

## Summary

Ticket: PLAT-1426. Every Behavior bullet, Description and Acceptance Criterion of FR-171 to FR-174 and the
NFR-038 Statement, Measurement and Verification were read against the ticket's scope, against quoin FR-046,
FR-133, FR-135, FR-137 and FR-138, against the code that implements them, and against companion PR
spec-objects-security#49. Scoped `quire validate` over the seven changed files passes. It reports only the
unrelated `semantic.unused-import` module warning. The diff quotes no standard text: NIST, OWASP, SOC 2,
ISO/IEC 27001, GDPR, HIPAA and FDA are named only. It contains no roadmap content beyond owning ticket ids.
It reuses no id: main ends at FR-170 and NFR-036, quoin#870 uses FR-180 to FR-182 and NFR-037, and no other
open quoin PR adds FR-171 to FR-174 or NFR-038.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | FR-173 has the person author an attestation with `sourceRevision` and `evidenceDigest` and copies it unchanged into each fact. The FR-046 attestation parser admits exactly `attestedBy`, `authority`, `attestedAt` and `expiresAt` (`exact(...)` in quoin-assurance discharge/parse.rs:74-79), and FR-133-AC-3 refuses an unknown field. Output that follows FR-173 is therefore refused by `record-discharge`, so FR-173-AC-7 cannot pass. Drop the two fields, or add a requirement that extends the FR-046 attestation. | spec/functional/FR-173-facts-from-mappings.md:38 |
| FND-002 | medium | FR-174 says a clause whose controls are not stated `effective` renders `Not compliant - undischarged`. That holds only when no fact was recorded before. FR-173 emits no fact for such a clause, and FR-133 replaces only a fact that is emitted again. A direct fact recorded while the control was `effective` therefore stays, and the clause still renders `Compliant` after the control is reassessed `ineffective`, until the attestation expires. Specify a withdrawal path, or narrow the bullet and say that the stale fact persists. | spec/functional/FR-174-per-clause-compliance-from-mappings.md:25 |
| FND-003 | medium | The FR-173 CLI contract is undecided. The synopsis omits `--approval-ref` and `--json`. `--approval-ref` is called "required", yet its absence is a defined non-error case. Left-open clauses go "on stderr or in `--json` output", but the Description says the command writes only the facts array to stdout, so a `--json` shape would not be `record-discharge --facts` input. The default for `--spec` is unstated. FR-173-AC-2, AC-3 and AC-4 assert "reported" and "listed" on no fixed channel. | spec/functional/FR-173-facts-from-mappings.md:21-45 |
| FND-004 | medium | FR-171 requires the skill change to "correct control numbers that are wrong against the free sources". No AC backs it: FR-171-AC-4 only limits what may change. Nothing defines which numbers are wrong, and SOC 2 and ISO/IEC 27001 numbers cannot be checked against a free source. List the corrections, or state the check and give it an AC. | spec/functional/FR-171-security-analysis-emits-typed-controls.md:38 |
| FND-005 | medium | FR-172 keys NIST at `nist/sp800-53@r5`. The skill currently cites "Rev 5.2.0", Rev 5 has several releases with different control content, and FR-073 diffs and FR-133 stores by exact version. ASVS gets an exact `@5.0.0`. Pick the release the set is authored from, or state why the coarser `r5` version is right. | spec/functional/FR-172-clause-set-sources-and-licence-boundary.md:22 |
| FND-006 | low | The FR-173 disposition takes "the exclusion's rationale and owner as the rationale", but the way the two values are joined is not stated. FR-173-AC-6 byte identity and FR-173-AC-4 need a fixed form. | spec/functional/FR-173-facts-from-mappings.md:43 |
| FND-007 | low | The NFR-038 Statement "Quoin SHALL NOT assign an effectiveness from a mapping alone" has no metric and no verification. The third metric is verified by "a directory listing", which no AC names. | spec/non-functional/NFR-038-mapping-facts-state-and-never-decide.md:21,44 |
| FND-008 | low | FR-172-AC-4 is a `Test` over artifacts that an LLM-run skill writes, and no harness or fixture run is named. It behaves as a Demonstration, like FR-172-AC-3. | spec/functional/FR-172-clause-set-sources-and-licence-boundary.md:50 |

## Verdict

Not mergeable as is. FND-001 breaks the central scope item ("how mappings feed record-discharge"): the facts the
new command emits are rejected by the existing store. FND-002 to FND-005 are real gaps that a reader would trip
on. Each of the six ticket scope items has a decision: FR-171 covers prose to typed controls, the out-of-scope
owner and the no-name-cleanup rule; FR-172 covers the publishable sets and the licensed packs; FR-173 and FR-174
cover the record-discharge feed and the per-clause RMM. The licence boundary is sound. FR-171-AC-1 to AC-5,
FR-172-AC-1 to AC-3 and AC-5, FR-173-AC-1 and AC-5 to AC-7 (apart from FND-001), and FR-174-AC-1 to AC-4 were
examined and are otherwise testable. The RMM state strings FR-174 uses match quoin-assurance discharge/rmm.rs:75-78.
