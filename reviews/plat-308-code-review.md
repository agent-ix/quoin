---
id: SR-20261111192909
title: "Code review of quoin PR 810 (FR-133 discharge facts in the evidence store)"
type: SpecReview
analysis: base
scope: "agent-ix/quoin@40f2c3c; rust/crates/quoin-assurance/src/discharge/{store,completeness,mod}.rs, rust/crates/quoin-assurance/src/lib.rs, rust/crates/quoin-core/src/ops/evidence/discharge.rs, rust/crates/quoin-core/src/ops/assurance.rs, rust/crates/quoin-core/src/dispatch.rs, rust/crates/quoin-cli/src/{discharge,evidence}.rs, tests"
review_set: subset
---

# Code review of quoin PR 810

## Summary

Ticket: PLAT-308. Scoped to `git diff origin/main...HEAD` at 40f2c3c (one commit, base is
current origin/main). The Rust lane (`rust-review`, with the repo's
`.claude/skills/rust-style/SKILL.md`) is folded in. `cargo test -p quoin-assurance -p quoin-core
-p quoin-cli` ran through locked-build.sh (debug=0, jobs 4): 515 passed, 0 failed. Probes ran
the built `quoin` binary against a scratch repository.

The design rule holds for what is persisted. A stored file is `{schemaVersion, clauseSet,
recordedAt, tool, fact}`. It has no clause list, force, expected outputs, applicability or count.
Every read re-parses the fact through `parse_fact` and re-partitions it against the caller's live
binding report, and expiry is decided at read time. A stored fact for an older clause-set version
sits under another directory and is never read for the new version. That is correct: the clauses
reopen until someone re-attests them.

Path components go through `record_stem`, which is injective, so `.`, `/` and `\` cannot
escape the directory. Last write wins per clause. That matches `trust/<ETD>.json` and run records
(`EvidenceSource::write`), whereas assurance and scenario records are append-only (`publish`). A
discharge attestation is a current, renewable decision, like a trust decision. Anyone who can
write a bogus attestation through the CLI can also hand-edit any store file. Append-only would not
stop that, so git history is an adequate log. I accept the decision.

No digest, pin, revision, environment or config hash was added. The census edits add
`MAX_DISCHARGE_BYTES`, the new ops source file and the new CLI route. They do not weaken any
check. FR-046 is untouched.

## Verdict

**CHANGES REQUESTED (four medium, two low).**

- The inward-completeness op takes an untrusted, precomputed report, and its identity can fail
  (FND-001).
- A duplicated clause in the binding report is counted twice (FND-002).
- Unbounded path components left a partial write on disk (FND-003, reproduced).
- The help pages for the new flags and command were never updated (FND-004, reproduced).

`discharge_report_from_store` is adequate for PLAT-307. Direct facts carry `evidenceRefs` for
Compliance, and dispositions carry decision and rationale for Tailoring. A disposition recorded
against a not-binding clause comes back only as an unused `{clauseId, kind, reason}`. The RMM's
"N/A with rationale" row therefore needs `read_discharge_facts`, which is public. This is not
listed as a finding.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | `assurance.inward_completeness` deserialises an arbitrary `DischargeReport` from stdin and computes `binding = direct + dispositions.len() + open`. It counts dispositions only when `entry.fact` is a `Disposition`. A report whose dispositions entry has no fact, or has a direct fact, gives `discharged + Σ dispositions + open != binding`, which breaks FR-133-AC-5. Duplicate or state-mismatched entries are also accepted. The op also takes the obligation set from a caller-held snapshot rather than the live binding report, against the ticket's design rule. Fix: take `{repo, binding, asOf}`, or `{binding, facts, asOf}`, and build the report internally. Otherwise, refuse an inconsistent report and count entries | rust/crates/quoin-core/src/ops/assurance.rs:325-341, rust/crates/quoin-assurance/src/discharge/completeness.rs:96-127 |
| FND-002 | medium | A `clause-binding-v1` report that lists the same clause id twice as binding is not rejected. One stored fact then counts as two discharged binding clauses. Reproduced: `quoin discharge --repo --inward` printed "Binding clauses: 2, Discharged by evidence: 2" for a single clause C-1. The gap is in FR-046's `build_discharge_report`, but FR-133's counts publish the wrong number. Refuse duplicate clause ids in the binding report (FR-046-AC-2 says "exactly once") | rust/crates/quoin-assurance/src/discharge/build.rs:63-67, rust/crates/quoin-assurance/src/discharge/completeness.rs:121 |
| FND-003 | medium | The authority, id, version and clause components have no length bound and are case-preserving. `record_stem` triples non-alphanumeric bytes. Reproduced: facts `[C-1, "."*120]` wrote `C-1.json`, then failed with ENAMETOOLONG and exited non-zero. The written fact stayed on disk, against "a refused list writes nothing". A fact for any clause id is accepted, because unused facts are stored. So clause ids that differ only in case (C-1, c-1) pass the case-sensitive duplicate check and overwrite one file on a case-insensitive filesystem. FR-130 bounds scenario ids at 64 bytes and refuses case collisions (TC-2209/2210). Bound each component, refuse case-folded collisions in the list, and pre-check all paths before the first write | rust/crates/quoin-assurance/src/discharge/store.rs:135-146, rust/crates/quoin-assurance/src/discharge/store.rs:204-210 |
| FND-004 | medium | `quoin discharge --help` and `quoin evidence --help` are served from the frozen `retained-command-help.json` capture, which this PR does not touch. Reproduced: discharge help still shows `--facts=<value> (required)` and has no `--repo` or `--inward`. Evidence help does not list `record-discharge`, so the `DESCRIPTION` edit in evidence.rs is dead text. `record-discharge --help` falls through to bare Clap output with no flag help. Earlier command changes updated the capture (#757, #763, #766). Update the discharge and evidence pages and add a `record-discharge` page | rust/crates/quoin-cli/tests/fixtures/retained-command-help.json, rust/crates/quoin-cli/src/evidence.rs:17, rust/crates/quoin-cli/src/discharge.rs:15-43 |
| FND-005 | low | The stored `tool {name, version}` and `recordedAt` are written and never read. `read_discharge_facts` discards everything but `fact`, and no report shows them. That is a provenance record that fails the value test. It also makes a re-record after a quoin upgrade churn the file with no change of fact. Git history already gives who and when. Drop both, or surface them in a report that someone uses | rust/crates/quoin-assurance/src/discharge/store.rs:64-76, rust/crates/quoin-assurance/src/discharge/store.rs:185-193 |
| FND-006 | low | `read_discharge_facts` has no ceiling on the number of files in a clause-set directory or on file size. Each file is read whole, then re-validated and partitioned with the O(facts x clauses) `known.contains` scan. That is consistent with the rest of the store, which bounds no read, but it breaks the "every accumulator has a ceiling" rule for a directory anyone can populate | rust/crates/quoin-assurance/src/discharge/store.rs:225-251 |

## Examined scope

```yaml
clean: false
scope:
- {id: store.rs, path: rust/crates/quoin-assurance/src/discharge/store.rs, role: examined, excerpt: "record_discharge_facts / read_discharge_facts / discharge_report_from_store"}
- {id: completeness.rs, path: rust/crates/quoin-assurance/src/discharge/completeness.rs, role: examined, excerpt: "build_inward_completeness / render_inward_completeness"}
- {id: ops/evidence/discharge.rs, path: rust/crates/quoin-core/src/ops/evidence/discharge.rs, role: examined, excerpt: "record_discharge / discharge_report, MAX_DISCHARGE_BYTES = 8 MiB"}
- {id: ops/assurance.rs, path: rust/crates/quoin-core/src/ops/assurance.rs, role: examined, excerpt: "inward_completeness"}
- {id: dispatch.rs, path: rust/crates/quoin-core/src/dispatch.rs, role: examined, excerpt: "three new OPERATIONS entries and match arms"}
- {id: cli/discharge.rs, path: rust/crates/quoin-cli/src/discharge.rs, role: examined, excerpt: "record_command / record / run --repo --inward / inward"}
- {id: cli/evidence.rs, path: rust/crates/quoin-cli/src/evidence.rs, role: examined, excerpt: "DESCRIPTION + record-discharge route"}
- {id: census edits, path: rust/crates/quoin-core/src/dispatch/tests.rs, role: examined, excerpt: "OPS_SOURCES + census_of_the_evidence_domain MAX_DISCHARGE_BYTES; NATIVE_OWNED_ROUTES record-discharge"}
- {id: tests, path: rust/crates/quoin-core/tests/tc_458_evidence_boundary.rs, role: examined, excerpt: "tc_2605 / tc_2602; tc_2606_discharge_store_command.rs"}
- {id: paths.rs, path: rust/crates/quoin-evidence/src/paths.rs, role: context_only, excerpt: "record_stem: [A-Za-z0-9-] kept, every other byte ~HH, empty ~E"}
- {id: source.rs, path: rust/crates/quoin-evidence/src/source.rs, role: context_only, excerpt: "write = replace mutable named record atomically; publish = append-only"}
```
