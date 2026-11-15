---
id: SR-206
title: "Base spec review of PLAT-1436 dossier spec (quoin PR #924)"
type: SpecReview
analysis: base
scope: "agent-ix/quoin; PR #924 @ ab633975: spec/functional/FR-188..FR-192, spec/non-functional/NFR-039, spec/usecase/US-024 and the three indexes"
review_set: subset
relationships:
  - target: "ix://agent-ix/quoin/FR-191"
    type: references
  - target: "ix://agent-ix/quoin/US-024"
    type: references
---

## Summary

Ticket: PLAT-1436. A base checklist review of the dossier spec. This is the
entry point for the spec-review set: EARS SR-207, integrity SR-208, dependency
SR-209, evidence SR-210 and failure-domain SR-211.

Checks that passed:

- The ids are the next free ones: FR-188 follows FR-187, NFR-039 follows
  NFR-038, and US-024 follows US-023. Criterion ids are parented correctly and
  unique.
- The indexes are updated.
- `quire validate` exits 0, and its grammar summary is clean.
- The commit-then-export flow is stated, and quire-cli FR-034 section A is cited
  as the reason. The citation was checked against the upstream text, which says
  "SHALL refuse the export when they differ".
- The rejected alternatives are stated: no uncommitted export and no bypass of
  `quire export`.
- The `quoin-quire` systems-record exposure gap is stated and assigned to
  PLAT-1437.
- Only generic document names are used.

Assurance profiles AP-201 and AP-202 are active with `review_policy.mode:
recommend`. Neither governs this change: it does not touch finding scoring or
measurement reporting, and it adds no Jev lens. The recommended evidence and
integrity analyses ran anyway, as SR-210 and SR-208. Scope-boundary analysis was
not requested and did not run. No human selection or acknowledgement was asked
for, because the policy mode is recommend.

## Verdict

CONDITIONAL. The spec is well structured, but the sub-analyses record five
high-severity defects: SR-208 FND-001 and FND-002, SR-209 FND-001 and FND-002,
and SR-211 FND-001. All of them must be fixed before the implementation tickets
start. The base findings below are medium and low.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-188 says `export` passes `--bundle-title` and `--bundle-id` "taken from this spec" but never states the values, so FR-188-AC-8 ("a bundle title and id") passes for any value and two implementations would title the bundle differently. | spec/functional/FR-188-dossier-command.md:40; spec/functional/FR-188-dossier-command.md:85 |
| FND-002 | low | US-024's examples contradict FR-188. EX-1 runs `quoin dossier generate` with no `--version`, which FR-188-AC-10 says exits 2 and writes nothing. EX-3 runs `export --to pdf` with no `--output`, which FR-188 Inputs require. | spec/usecase/US-024-hand-a-reviewer-dossier-documents-generated-from-records.md:31; spec/usecase/US-024-hand-a-reviewer-dossier-documents-generated-from-records.md:43 |
| FND-003 | low | FR-188-AC-4 expects `check` to exit 0 with no edit, but the exit table gives 0 only when every document is also complete. The AC does not say its fixture is complete, so an incomplete fixture correctly exits 1 and fails the AC. | spec/functional/FR-188-dossier-command.md:68; spec/functional/FR-188-dossier-command.md:81 |

## Dispositions

Disposition round 1, checked against PR #924 head 5bba8af4 (fix commits 196404ac
and 5bba8af4).

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | - |
| FND-002 | fixed | - |
| FND-003 | fixed | - |

- FND-001 after_excerpt (`spec/functional/FR-191-dossier-command.md:45`): `- When `export` selects more than one document, `export` SHALL run `quire export` once, as one bundle, with the files in the fixed order `DOS-SRS`, `DOS-ARCH`, `DOS-TRACE`, `DOS-RMM` and with `--bundle-title "Submission Dossier"` and `--bundle-id DOS`.`
- FND-002 after_excerpt (`spec/usecase/US-024-hand-a-reviewer-dossier-documents-generated-from-records.md:31,43`): `- **When** the team runs `quoin dossier generate --version 1.0 --clause-binding binding.json --as-of <RFC 3339 timestamp>`` and `- **When** the team runs `quoin dossier export --to pdf --output dossier.pdf --version 1.0 --clause-binding binding.json --as-of <RFC 3339 timestamp>` with the same values as the generation`
- FND-003 after_excerpt (`spec/functional/FR-191-dossier-command.md:114`): `| FR-191-AC-4 | On a fixture whose documents are all complete, `check` on a committed file with one hand-edited line reports that document as `differs` and exits 1; with no edit it reports `matches` and exits 0 | Test |`
