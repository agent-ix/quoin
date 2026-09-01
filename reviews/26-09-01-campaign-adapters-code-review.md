---
id: SR-116
title: "Code review — campaign-native result adapters"
type: SpecReview
analysis: code-review
scope: "src/evidence/adapters/{contract-conformance,differential-report,types,registry}.ts, src/commands/evidence/record.ts, tests/campaign-adapters.test.ts, tests/fixtures/evidence/, docs/campaign-native-result-inventory.md, spec/functional/FR-069, spec/matrix.md"
review_set: subset
relationships:
  - target: "ix://agent-ix/quoin/issues/323"
    type: "reviews"
  - target: "ix://agent-ix/quoin/FR-069"
    type: "references"
---

# SR-116: Code review — campaign-native result adapters

## Summary

Reviews the FR-069 inventory and the two adapters it justifies: contract
conformance JSONL from `quire-contract-ir`, and domain differential summaries
from `tl-mltl`. Eleven of thirteen scope items get no adapter, each for a
stated reason.

## Verdict

**CONDITIONAL** — no high findings. One medium finding records a gate claim I
made wrongly on the previous PR and have corrected; one low finding records a
vocabulary limit this change deliberately did not resolve.

## Gates

- `tsc --noEmit` — clean.
- `eslint src tests` — clean.
- `prettier --check .` — clean, after FND-001.
- `vitest run` — 926 of 927 pass. The single failure is `quire-contract`
  TC-118, filed as #326 and unrelated.
- `quire coverage --json` — FR-069 has no unbacked row, and
  `tests/campaign-adapters.test.ts` contributes no unmatched tag.

## Findings

| ID      | Severity | Summary                                                                           | Refs                                                   |
| ------- | -------- | --------------------------------------------------------------------------------- | ------------------------------------------------------ |
| FND-001 | medium   | PR #325 reported `prettier --check .` clean; the check ran before the last commit | reviews/26-09-01-change-assurance-cli-code-review.md:1 |
| FND-002 | low      | `unsupported` has no stored outcome, so it is reported rather than recorded       | src/evidence/adapters/types.ts:33                      |

## Finding detail

### FND-001 — a gate claim that outran the gate

PR #325 and SR-114 both reported `prettier --check .` as clean. It was, when I
ran it — and then I added two review artifacts and committed without re-running
it. Both files landed unformatted, so `prettier --check .` fails on `main`.

Failure scenario: a reader takes "gates green" as a statement about the merged
commit, when it was a statement about an earlier one. Nothing about the FR-068
surface is affected — `tsc`, `eslint`, `vitest`, and the built-CLI end-to-end
run were all genuinely clean against the final tree.

Fixed here: both files are formatted, `prettier --check .` passes, and the
correction is posted on #325 rather than left in a review nobody re-reads. The
lesson is narrow and worth keeping: a check run before the last commit is not a
check of the commit.

### FND-002 — a producer state the store cannot hold

`RunEntry.outcome` is `pass | fail | skip | error`. A differential report's
`unsupported` is none of them: nothing chose to omit the case, and nothing
failed.

Failure scenario: an adapter that mapped it to `skip` would delete the
distinction at the point of intake, permanently and silently, for every future
run of that producer.

Deliberately not resolved here. The adapter refuses to transcribe the state and
names it in `AdapterResult.unrepresented`, which `quoin evidence record` prints
in both output modes. That is an adapter-layer field: no record family, no
stored schema change, nothing duplicated from Engineering Assurance. Extending
the stored outcome vocabulary is a bigger decision than an adapter ticket, and
it should be made against a count of how many real producers need it — not
against this one.

## Notes

- Both adapters fail closed on an unknown protocol or schema version rather than
  reading what they recognise. A JSONL stream under a different protocol would
  parse just as cleanly and mean something else, and a conformance run is
  exactly where a silently-misread row becomes a fixture nobody checked.
- Both refuse an empty result set. A conformance run that emitted no line did
  not report that every fixture matched, and a differential that compared no
  case did not report agreement — that is the vacuity rule FR-034 already
  applies to rule-less scans.
- Conformance identity is `<corpus>::<operation>::<fixture>`. The runner replays
  the same fixture id under four operations, so a bare fixture id would let one
  result overwrite another.
- The differential schema is matched as `<domain>.differential-summary/v1`, so
  another domain can adopt the shape without claiming `tl-mltl`'s identity, and
  a `v2` is refused rather than read by a `v1` reader that recognises the fields
  it knows.
- Samples are real. The conformance fixture is seven unedited lines of a real
  99-row run — JSONL is line-delimited, so a selection of lines is still real
  bytes per record — and the full run's digest is recorded in the inventory so
  the selection is pinned to a specific run. The differential fixture is the
  committed report at `tl-mltl` `fe1c620`, unedited.
- TC-1332's expectation of the exact adapter-name list was updated, not
  widened. That list is deliberately pinned as the command's surface; adding two
  names is a surface change and the test is where it gets acknowledged.
- Three scope items get no adapter because no producer emits them. Writing a
  reader for a format nobody produces means inventing the format, and the first
  real sample would then be shaped to fit the reader.
