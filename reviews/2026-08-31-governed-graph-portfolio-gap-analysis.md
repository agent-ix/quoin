---
id: SR-111
title: "Gap analysis of PLAN-006 governed graph portfolio"
type: SpecReview
analysis: gap-analysis
scope: "PLAN-006; StR-007; US-019; FR-066..FR-067; TM-001 TC-1293..TC-1316"
review_set: subset
relationships:
  - target: "ix://agent-ix/quoin/PLAN-006"
    type: reviews
  - target: "ix://agent-ix/quoin/TM-001"
    type: references
---

# Gap analysis of PLAN-006 governed graph portfolio

## Summary

PLAN-006 is complete. All six task artifacts are `done`, every requirement and
test-plan checkbox is closed, TM-001 marks TC-1293..TC-1316 covered, and every
case resolves to a real test symbol. Quire reports FR-066 at 12/12 backed
criteria and FR-067 at 11/11, with no #281 unbacked row, status lie, no-symbol
row, or unmatched TC tag.

## Verdict

**PASS.** Two mechanical trace defects were corrected. No incomplete task,
unbacked matrix row, unmatched #281 tracking tag, stub, or underspecified
production surface remains.

## Findings

| ID      | Severity | Summary                                                                                                                                                                                                                         | Refs                     |
| ------- | -------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------ |
| FND-001 | high     | Resolved: TC-1298, TC-1299, and TC-1302 used curried `test.each`, a Vitest form the declared TypeScript symbol grammar cannot bind. Each is now one normally named test with an internal exhaustive case loop.                  | FR-066-AC-6, AC-7, AC-10 |
| FND-002 | high     | Resolved: all TC ids were present, but FR-066 and FR-067 criterion groups were 0/12 and 0/11 because tests lacked direct `Trace:` markers. The 23 criteria plus StR-007-VC-1 now bind directly; the groups are 12/12 and 11/11. | FR-066; FR-067; StR-007  |

## Coverage

- Plan completion: 6/6 tasks done; 3/3 requirement checkboxes and 24/24
  test-plan checkboxes closed.
- Matrix verification: TC-1293..TC-1316 are 24/24 covered. Their symbols live
  in `tests/graph-adapters.test.ts`, `tests/graph-portfolio.test.ts`, and
  `tests/graph-portfolio-command.test.ts`; TC-1311 deliberately has both pure
  and command-boundary symbols.
- Quire coverage: FR-066 is 12/12 backed and FR-067 is 11/11 backed. Filtered
  #281 results contain zero unbacked rows, status lies, no-symbol rows,
  unmatched tags, or missing criterion groups.
- Test execution: 26/26 focused adapter/portfolio/command tests pass; the
  combined #152/#281 seam and regression slice passes 58/58; the full suite
  passes 845/845 with the repository-pinned local Quire 0.30.2 binary.
- Review-finding closure: malformed collection files are locally
  `unreadable`; valid siblings survive; snapshot latest/comparison ordering is
  timestamp/id based; curried tests are bindable; every criterion has a direct
  trace marker.
- Reverse traceability: adapter registry, assurance validation, graph-quality
  transcription, tolerant store reads, portfolio reducer, graph-input loader,
  report command, renderers, and exports trace to FR-066/TASK-033..034 or
  FR-067/TASK-035..037. Review and matrix changes trace to TASK-038/StR-007.
  No changed production surface lacks an owning requirement.
- Boundary check: #281 consumes #152's final `17ed860` interface and creates no
  competing graph contract. It performs no producer, Quire, Git, network, or
  graph-discovery work and makes no change in Quoin #286, agent-skills,
  Filament, or quire-code-rs.
- Optional semantic review: skipped as directed; the user did not authorize the
  semantic intent↔test↔code expansion.

## Validation

- `make lint`: pass.
- `corepack pnpm run build`: pass.
- Full pinned Vitest suite: 845/845 pass.
- SR-110 and SR-111 validate as `SpecReview` artifacts with the pinned local
  Quire CLI; the repository's existing duplicate-module warnings are unchanged.
- Diff inspection finds no conflict marker, whitespace error, skipped test,
  placeholder, or file outside the #281 Quoin scope.
