---
id: SR-155
title: "Evidence review of the Rust burn-down spec set"
type: SpecReview
analysis: evidence
scope: "ADR-0003, StR-009, US-024, FR-096..FR-103, NFR-024..NFR-027, IT-003"
review_set: all
---

# Evidence review of the Rust burn-down spec set

## Summary

This review asked, of every criterion in the EPIC #373 spec set, what artefact
proves it and what command produces that artefact. Ninety-eight criteria were
examined — five StR-009 validation criteria, eighty-eight FR and NFR acceptance
criteria, and five IT-003 step criteria — across fifteen artefacts at commit
`30425e6`. No artefact in the set carries a `verification_method` or `evidence`
field, so every evidence claim is inferred from a parenthesised TC identifier;
fourteen of ninety-eight criteria name a command; nine disclose the population
they run over. The sharper result is that the two enforcement artefacts this set
is written against already exist in the tree —
`docs/rust-burndown/executable-path-matrix.md` and `.language-allowances.yaml` —
and neither is named by any criterion, while three criteria as written refuse
them: NFR-024's successor-validity rule rejects the successor convention all 346
retained matrix rows use, NFR-024-AC-6's allowance-number domain cannot express
the categories the manifest declares, and FR-103-AC-6 requires the gate to fail
on dispositions both artefacts already record.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | No artefact in the batch declares `verification_method` or `evidence`; all 93 requirement-level criteria infer their evidence from a parenthesised TC id, and the three non-test criteria name no artefact path at all. | StR-009; FR-096..FR-103; NFR-024..NFR-027; StR-009-VC-1; FR-097-AC-7; FR-102-AC-7 |
| FND-002 | high | 84 of 98 criteria name no command, against the batch's own rule that a criterion names a command rather than an intent; the 14 that do are FR-096-AC-1, FR-097-AC-1, FR-098-AC-1, FR-099-AC-1/2/3/6, FR-100-AC-1, FR-101-AC-7/8, FR-102-AC-1, NFR-026-AC-2, NFR-027-AC-3 and IT-003-SC-01. | FR-096..FR-103; NFR-024..NFR-027; StR-009; IT-003 |
| FND-003 | high | Absence-shaped criteria are satisfiable over an empty population: each asserts that nothing was found without a planted violation or a population count, so a gate that scanned nothing passes them. Only 9 of 98 criteria disclose a population. | FR-097-AC-6; FR-101-AC-5; FR-102-AC-6; FR-103-AC-4; NFR-024-AC-3; FR-100-AC-8 |
| FND-004 | high | NFR-024's successor-validity rule refuses the only retention artefact that exists: `docs/rust-burndown/executable-path-matrix.md` names `quoin#373 Stage N` as the successor for all 346 retained rows (81,992 lines), which NFR-024 Verification declares invalid and NFR-024-AC-2 requires to fail; no criterion names that file as its evidence artefact. | NFR-024-AC-1; NFR-024-AC-2; FR-101-AC-1; docs/rust-burndown/executable-path-matrix.md |
| FND-005 | high | The allowance manifest the criteria read is not the one in the tree: `.language-allowances.yaml` is untracked at `30425e6` so it is not checked in, NFR-024 Verification calls it `quoin/.language-allowances.yaml` while AC-6 calls it root-level, and its eight named categories carry no allowance number where AC-6 requires one in 1..4 — the matrix already classifies rows under allowances 5 and 6. | NFR-024-AC-6; NFR-024-AC-7; NFR-024-AC-8; NFR-024-AC-9; .language-allowances.yaml |
| FND-006 | high | Two gates return opposite verdicts on the same 23,164 lines: FR-099-AC-9 and FR-101 require every `skills/**/workflow-assets/**` asset classified as executable assertion logic with a recorded disposition, while `.language-allowances.yaml` exempts `skills/**` wholesale under owner ruling quoin#373 F-1, and no criterion says which artefact is the evidence. | FR-099-AC-9 (TC-1683); FR-101; NFR-024-AC-9; .language-allowances.yaml |
| FND-007 | high | FR-103-AC-6 requires a disposition recorded against a path inside `corpus/` to fail the gate, yet both existing artefacts record exactly that (manifest `inert` entries for `corpus/cases/**` and `corpus/**`; matrix Allowed row `corpus/cases/**`), and the manifest records the qa-corpus scope ruling as taken on 2026-09-12 where ADR-0003 and FR-103 still treat it as open. | FR-103-AC-6 (TC-1684); FR-103; ADR-0003; .language-allowances.yaml |
| FND-008 | high | The enforcement population is defined to exclude extension-less executable paths: FR-101-AC-9 fixes the inventory at 105,814 lines across 364 files over `{.ts,.mjs,.js,.py,.sh,.tsp}`, while the manifest's own enforcement requirements name 36 executable paths with no extension to scan — Makefile recipes and `.github/workflows/**` `run:` blocks — that no criterion in the set covers. | FR-101-AC-9 (TC-1690); FR-101-AC-8; NFR-024-AC-9; .language-allowances.yaml |
| FND-009 | high | No criterion requires provenance on the expected fixtures FR-101 mandates: once the retained implementation is deleted, FR-098's differential compares Rust against committed fixtures with no recorded producing implementation, revision or digest, which is the self-written-fixture failure FR-101-CON-3 forbids, raised one level. | FR-101; FR-101-CON-3; FR-098-AC-1 (TC-1618); FR-101-AC-5 (TC-1645) |
| FND-010 | medium | FR-101-AC-6 is the set's own self-written-fixture gate and names no artefact or command — "refused by the review gate", where no gate exists and none is required to exist — and "a fixture it wrote in the same test body" narrows FR-101's own Behavior clause so that a setup helper or a separate generation step escapes it. | FR-101-AC-6 (TC-1646); FR-101-CON-3 |
| FND-011 | medium | Seven of the eight Property-verified criteria name no input domain or generator, so the evidence artefact is unspecified and a one-element strategy satisfies each; FR-101-AC-2 and FR-100-AC-3 have no generatable domain at all and are gate rules rather than properties. FR-098-AC-4 is the only one that states its domain. | FR-096-AC-2/AC-3; FR-098-AC-2/AC-5; FR-100-AC-3; FR-101-AC-2; FR-102-AC-2 |
| FND-012 | medium | NFR-026-AC-2's evidence is unconstructible as written: `rust/rust-toolchain.toml` pins the channel so an older toolchain cannot be active without an override the criterion does not name, and neither `cargo fmt --check` nor `cargo clippy` reports a required-versus-observed version, which needs a wrapper gate no requirement creates. `rust/` does not exist at this commit. | NFR-026-AC-2 (TC-1671); NFR-026-AC-1 (TC-1670) |
| FND-013 | medium | IT-003 supplies no evidence and binds to no test row: its five step criteria carry no id from the allocated TC-1601..TC-1690 block where IT-002 cites TC-EV ids, and its own Notes record that `filament-core-data` publishes nothing, so the published-package evidence for StR-009-VC-4 and FR-097 is zero until Phase B closes and nothing records that as an evidence gap. | IT-003-SC-01..05; FR-097-AC-2 (TC-1613); StR-009-VC-4 |
| FND-014 | medium | StR-009 allocates TC-1601..TC-1604 for assertions already carried at FR level (VC-2 against TC-1641, VC-3 against TC-1618, VC-4 against TC-1613, VC-5 against TC-1648), so stakeholder coverage can be satisfied by four thin re-assertions, and VC-1 is Demonstration with no demonstration artefact named. | StR-009-VC-1..VC-5; FR-101-AC-1; FR-098-AC-1; FR-097-AC-2; FR-101-AC-8 |
| FND-015 | medium | Six criteria require a gate to read a record whose location no requirement names — the three-part retention answer, the executable-path row, the final inventory, the dated oclif disposition, the per-module corpus disposition and the retention matrix row — while two of those records already exist in the tree uncited. | FR-100-AC-6 (TC-1638); FR-101-AC-1 (TC-1641); FR-101-AC-8 (TC-1648); FR-102-AC-3 (TC-1651); FR-103-AC-1 (TC-1655); NFR-024-AC-1 (TC-1660) |
| FND-016 | medium | Nine criteria carry a non-test verification method with no evidence artefact: two Inspection acceptance criteria and seven Inspection constraint rows name neither a checklist, a file, nor a TC, so nothing records that the inspection was performed. | FR-097-AC-7; FR-102-AC-7; FR-096-CON-4; FR-097-CON-3; FR-099-CON-2; FR-100-CON-2; FR-101-CON-5; FR-102-CON-2; FR-103-CON-3 |
| FND-017 | medium | FR-098 names its parity population as "the pinned external spec corpus cited in `src/quire/exec.ts`", and that file cites no corpus pin — its only mention of a corpus is a `maxBuffer` comment — so the differential's declared population has no identity reference, and FR-102 deletes that file at the shell replacement. | FR-098; FR-098-AC-1 (TC-1618); FR-102-AC-6 (TC-1654) |

## Evidence strategy

The set's intended strategy is sound and is stated in prose rather than in
evidence fields. Four methods are in use: differential and digest-replay tests
against the retained implementation as oracle (FR-098, FR-100, NFR-025);
classification gates reading a checked-in manifest and a matrix (FR-101,
FR-103, NFR-024); source and CI checks over the Rust workspace (NFR-026,
NFR-027); and one live-registry integration test (IT-003). Each of the four is
paired with a planted-violation clause somewhere in its requirement, which is
the right instinct and is what keeps most of the set out of FND-003.

What is missing is the binding layer. Two evidence artefacts —
`docs/rust-burndown/executable-path-matrix.md` and `.language-allowances.yaml` —
already carry the classifications the gates are supposed to assert over, and no
criterion names either, so FND-004 through FND-008 are all the same defect seen
from different requirements: the criteria were written against an imagined
artefact rather than the one in the tree. Naming the two files, and reconciling
the successor convention, the allowance vocabulary, the `skills/**` ruling and
the `corpus/` ruling against them, closes five of the nine high findings without
weakening any criterion.

Population disclosure is the second gap and is cheaper to close. FR-098-AC-1,
FR-098-AC-7, NFR-024-AC-5, NFR-025-AC-1, NFR-025-AC-5 and NFR-027-AC-9 already
require a population count or an inconclusive verdict; the same clause applied
to the absence-shaped criteria in FND-003 removes the empty-population escape
the owner named in US-024-EX-3.

No test exists for TC-1601 through TC-1690 and the Test Matrix has not been
rebuilt, so nothing here is a coverage measurement. These findings are about
whether the criteria, once tested, would produce evidence — not about whether
the evidence exists yet.
