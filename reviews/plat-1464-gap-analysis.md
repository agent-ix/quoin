---
id: SR-20261112020054
title: "Gap analysis of quoin PR 824 (PLAT-1464 NFR-003/NFR-007 populations)"
type: SpecReview
analysis: base
scope: "agent-ix/quoin@4faa1137; NFR-003-M-1, NFR-003-M-2, NFR-007-M-1, FR-131-AC-9, FR-131-AC-10"
review_set: subset
---

# Gap analysis of quoin PR 824

## Summary

Ticket: PLAT-1464. Planless. Plan completion was not assessed. This analysis covers the
criteria the PR binds and the 14 ids the PR records as below target.

`quoin matrix --repo .` (quoin 0.30.0) on the PR head reports these as `tagged`:
FR-131-AC-9, FR-131-AC-10, NFR-003-M-1, NFR-003-M-2 and NFR-007-M-1. It reports no
criterion tagged outside its row. The test run read MP-250 as 97/111, MP-251 as 7/111 and
MP-252 as 24/24.

I reproduced all 14 below-target ids by hand against the PR-head binary, in a home seeded
from the retained catalog. Thirteen are genuine quoin defects. One (`report --portfolio`) is
a design question. None is a probe artefact:

- `report --repo /nonexistent`: exit 0. It prints an empty QA report and nothing on stderr.
- `report --portfolio /nonexistent`: exit 0. Stdout says "Status: missing — repository does
  not exist" and stderr is empty. Reporting per-member status may be by design. Decide
  whether an all-missing portfolio exits non-zero.
- `evidence gc --repo /nonexistent`: exit 0, prints "nothing to collect", no diagnostic.
- `change-assurance recover --repo /nonexistent`: exit 0, prints "removed 0 interrupted intake
  staging directories", no diagnostic.
- `completeness --repo` / `--bundle`: exit 0, UNCHECKED. Stderr only says no module declares
  vocabulary coverage; the missing path is never named.
- `completeness --module /nonexistent`: exit 0. The module root is silently ignored, and the
  hint says "pass --module" although --module was passed.
- `validate --repo /nonexistent`: exit 1, but the message reads "repository root  is not a
  readable directory". The path is blank (an empty interpolation), and the message is wrapped
  in a "native runtime validators.run failed: CORE_REFUSED" prefix.
- `measurement intervention --definition`, `measurement operational-release --definition`,
  `measurement record --input`: exit 1, "cannot read measurement definition|input: No such file
  or directory (os error 2)". The path is not named.
- `catalog show NoSuchType`: exit 1, "catalog type not found: X". It does not list the
  available types.
- `change-assurance write-record --input` (file and stdin): exit 2, "native runtime
  change_assurance.write_record failed: CORE_BAD_REQUEST: object key must be a string at byte
  1". It never says the input is JSON or names `--input`, and it leaks the internal op name.
  `evidence record-experiment` gets this right: "input is not JSON: ... at line 1 column 2".

## Verdict

Coverage of the bound criteria is honest. The tests are tagged, run and pass. The
below-target readings are recorded, not hidden, and the ratchet pins them. The product
defects above belong in the separate follow-up fix PR, not this one.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | NFR-003-M-1 reads 97/111 and NFR-003-M-2 reads 7/111, against a 100%/0 target. Thirteen genuine quoin defects across report, evidence gc, change-assurance recover, completeness (x3), validate, measurement (x3), catalog show and change-assurance write-record; follow-up fix PR | rust/crates/quoin-cli/tests/nfr_populations/nfr003.rs:124-136 |
| FND-002 | low | `report --portfolio` with a missing member exits 0 and reports the member as missing on stdout. Whether this is a defect is a design decision not stated in NFR-003 or the portfolio FR | spec/assurance/MP-250-nfr003-actionable-failures.md |
