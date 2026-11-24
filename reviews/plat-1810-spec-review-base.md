---
id: SR-20261124001342
title: "base review of the dossier risk content, safety class, run identity and records-of-processing check/export spec change"
type: SpecReview
analysis: base
scope: "agent-ix/quoin; spec/functional/FR-191-dossier-command.md, spec/functional/FR-192-dossier-srs-content.md, spec/functional/FR-193-dossier-architecture-content.md, spec/functional/FR-194-dossier-trace-matrix-content.md, spec/functional/FR-196-dossier-verification-report-content.md, spec/functional/FR-199-dossier-risk-management-content.md, spec/functional/FR-207-dossier-records-of-processing-content.md (PR #1038)"
review_set: subset
---

# base review of PR #1038

## Summary

Tickets: PLAT-1810, PLAT-1813, PLAT-1814. Diff scope: `git diff origin/main...HEAD`
(one commit, seven FR files, 81 insertions, 26 deletions); the branch is current with
`origin/main`. The base checklist ran over every changed line: id formats, duplicate
and reused ids, link integrity, compliance-tool lens, and the hash / pin / ledger
antipattern scan. `quire validate --scope .` over the seven files exits 0
(quire 0.52.0, engine 0.62.2). The defects this review found are recorded in the
integrity and dependency SpecReviews of the same PR; this file records the clean
checklist result and the recommendations on the author's open questions.

## Scope examined

- New ACs FR-192-AC-7..9, FR-193-AC-7..8, FR-194-AC-7..8, FR-196-AC-9..12,
  FR-199-AC-9..14, FR-207-AC-10: well formed and unique. `git log -S` over
  `origin/main` shows none of these ids was ever used before, and no source file
  carries a trace tag for any of them.
- Amended ACs FR-191-AC-13, FR-191-AC-24, FR-196-AC-3, FR-199-AC-1 keep their ids.
  Their existing tagged tests (`records_of_processing.rs:1282`,
  `tc_1614_dossier_command.rs:363`, `verification_report.rs:489/536/638`,
  `risk_management.rs:436/467`) assert the old behaviour and must change in the
  code PRs; that is expected for a spec-first change, not a spec defect.
- Gap-kind list, gap order, subject rules and no-id sort list in FR-191: the three
  new kinds (`safety_class_unassigned`, `run_identity_unrecorded`,
  `no_overall_evaluation`) appear once each, every FR that emits them names them
  identically, and every subject rule they need is present.
- Compliance lens: no score, class or acceptability is assigned by the tool; the
  wording is in the project's own words and quotes no standard text.
- Antipattern scan: no migration, compatibility or fallback language; no pin, ledger,
  digest or machine path. The run "source commit" of FR-196 is a displayed
  provenance value of a real feature, not a pin.

## Findings

| ID      | Severity | Summary                                                                  | Refs |
| ------- | -------- | ------------------------------------------------------------------------ | ---- |
| FND-001 | low      | No findings (placeholder); defects are in the integrity and dependency SpecReviews | -    |

## Verdict

Base checklist clean. The PR is NOT mergeable as a whole because of the three high
findings in the dependency SpecReview (DOS-ROP export refused by `quire export`,
the FR-199 source contract contradicting FR-169's closed keys, and FR-196 run
identity values the run file never records).

## Recommendations on the author's open questions

1. FR-193 stays part-level, "item" meaning the part: agree. The text does not say
   that yet ("each part and each other item"); see the integrity review.
2. The coder reads whatever is already parsed for run identity and leaves absent
   fields `not recorded`: not workable as specified. `quoin-assurance` parses no
   run-file field itself; the retained run record (`quoin-evidence` `RunRecord`)
   holds `timestamp`, one combined `tool` string and per-symbol `outcome`s, and no
   source commit. Under FR-196 as written every run-backed criterion would carry a
   permanent `run_identity_unrecorded` gap. The spec has to be fixed first (see the
   dependency review).
3. No spec/test TC files: agree. Quoin binds tests to criteria by trace tags; the
   other dossier FRs have no TC files, and the TC-617..625 files of FR-207 are not a
   rule the new AC must follow.
