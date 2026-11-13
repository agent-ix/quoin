---
id: SR-20261113080830
title: "Integrity review of per-class evidence floors and class obligations (FR-160, FR-161, NFR-034)"
type: SpecReview
analysis: integrity
review_set: subset
scope: "agent-ix/quoin; PR #855 (git diff origin/main...015a0665): spec/functional/FR-160-per-class-evidence-floors.md, spec/functional/FR-161-per-class-obligations-report.md, spec/non-functional/NFR-034-class-reports-state-and-never-assign.md; context FR-032, FR-039, FR-094, quoin-auditor audit/{input,scores}.rs, quoin-evidence types/independence.rs, quire-rs FR-090/FR-091 (PR 669)"
---

# Integrity review of per-class evidence floors and class obligations (FR-160, FR-161, NFR-034)

## Summary

Ticket: PLAT-1526. This review checks the diff for consistency with FR-032, FR-039 and FR-094 as merged, and with the
sibling engine requirements. The ids FR-160, FR-161 and NFR-034 are new and unique, and both index entries are present.
The cross-repo content is referenced, not copied. The claim that FR-160 reuses the FR-039 mutation rule holds: it uses
the same tokens, the same metric filter and the same worst-symbol reading. The claim that it reuses FR-094 and FR-032
does not hold as written.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-160's Description says quoin "already applies a floor per criticality" through FR-039, FR-032 and FR-094, and that FR-160 "keys the same three floors" on class. Two parts of that are wrong. First, FR-094 is not per criticality. It is keyed on exact obligation ids under a profile projection (`IndependencePolicy { profile, requirements }`), and FR-094 says it is "rather than a second project-wide criticality table". Second, the class policy's three floors are `required_methods`, `mutation_floor` and `independence`. FR-032 multiplicity (`multiplicity_requires`) is not among them, and `required_methods` has no FR-032/039/094 counterpart. Describe the reuse as it is, and decide whether a class needs a multiplicity floor. | FR-160; rust/crates/quoin-evidence/src/types/independence.rs:75-82; rust/crates/quoin-auditor/src/audit/input.rs:69-82; spec/functional/FR-094-profile-selected-evidence-independence.md |
| FND-002 | medium | FR-160 does not say what happens when `--independence-policy` (FR-094) and a class `independence` entry both select dimensions for one obligation. It does not say whether the dimensions are united, or whether there is one `insufficient-independence` finding or two. FR-160-AC-5 settles only the mutation case. The FR-094 auditor also consumes store-made `IndependenceAssessment`s, one per policy requirement, and FR-160 does not say how class dimensions become those inputs. State the combination rule and give it an AC. | FR-160-AC-4; FR-160-AC-5; rust/crates/quoin-auditor/src/audit/input.rs:51-58 |
| FND-003 | medium | FR-160-AC-8 refuses "a policy naming an undeclared class", meaning one absent from "the Quire payload's `software_safety_class` enumeration". No Quire payload carries that enumeration. Quire-rs FR-090 (PR 669) reads the enum only to leave values outside it to schema validation, and `quoin-quire-types` has no vocabulary field. FR-160-CON-2 forbids reading the Markdown. So the AC's first clause has no input. Either add a quire-rs AC that emits the enum, or drop the clause. | FR-160-AC-8; rust/crates/quoin-quire-types/src/lib.rs |
| FND-004 | low | FR-160-AC-6 and FR-160-AC-10 add a class column to `quoin matrix`. FR-160 does not say whether `matrix` takes `--class-policy`. `matrix` already takes `--mutation-floor`, `--multiplicity-requires` and `--independence-policy`, so leaving it out makes the two commands report different floors for one obligation. State it either way. | FR-160-AC-10; quoin matrix --help |
| FND-005 | low | `ix://agent-ix/quire-rs/spec/functional/FR-090` and `FR-091` differ from the convention used by the 46 other quire-rs references in this spec, `ix://agent-ix/quire-rs/FR-NNN`. Use the id form. | spec/functional/FR-160-per-class-evidence-floors.md:14, :32; spec/functional/FR-161-per-class-obligations-report.md:12, :46 |
| FND-006 | low | NFR-034's Verification cites FR-160-AC-6 (unclassified count), which bears on none of its three metrics. No FR-160 AC covers determinism or writing nothing under `--class-policy` for the audit. Cite FR-032's Behavior and FR-032-CON-1, or add an AC. | NFR-034 (Verification); FR-160-AC-6 |

## Verdict

Changes requested. The independence reuse (FND-001, FND-002) and the class-enumeration input (FND-003) need stating
before implementation. The rest are small.
