---
id: FR-164
title: "quoin anomaly report --fail-on: a gate for CI and release decisions"
type: FR
relationships:
  - target: "ix://agent-ix/spec-artifacts-process/US-003"
    type: "implements"
  - target: "ix://agent-ix/quoin/FR-162"
    type: "depends_on"
  - target: "ix://agent-ix/quoin/FR-163"
    type: "depends_on"
---

# FR-164: quoin anomaly report --fail-on: a gate for CI and release decisions

## Description

Tracked by Linear PLAT-1528 and PLAT-1586. `quoin anomaly report --fail-on <mode>` turns the report of FR-162 into a
check that CI or a release step can run. It tests a structural condition of the records at the checked-out commit:
whether an anomaly is still unassessed, and whether a carried safety-impacting anomaly has a release rationale
(FR-163). A carried anomaly is one FR-162 lists because it is not `closed`, or is `closed` as `wont-fix`: a defect
deliberately shipped must not pass the gate by being closed. It does not decide whether a person should release with an anomaly; that is the rationale's author's
judgement, and the gate does not read the rationale for sense.

## Behavior

### Modes

- The option `--fail-on` SHALL take the value `unassessed` or `unjustified-impacting` and MAY be given more than once.
- If `--fail-on` is not given, then the command SHALL evaluate no gate and the report SHALL carry no `gate` section.
- Under `--fail-on unassessed`, the command SHALL fail for each listed anomaly whose `safety_impact` is `unassessed`.
- Under `--fail-on unjustified-impacting`, the command SHALL fail for each anomaly whose `status` is not `closed`, or is `closed` with `resolution` `wont-fix`, whose `safety_impact` is `impacting` and which has no release rationale.
- An anomaly has a release rationale when its `releaseRationale` holds at least one entry whose `author` and `rationale` each satisfy the value rule of `agent-ix/spec-artifacts-process` FR-016: non-empty after trimming and not a placeholder (`n/a`, `none`, `unknown`, `tbd`, `-`, compared case-insensitively). A document that passes `quire validate` and the gate therefore agree.
- The command SHALL NOT read whether a rationale is adequate.
- An anomaly whose `safety_impact` is `unassessed` SHALL NOT fail `unjustified-impacting`.
- A `closed` anomaly whose `resolution` is not `wont-fix` SHALL NOT fail `unjustified-impacting`, whatever it carries.
- If more than one mode is given, then the command SHALL evaluate each, and an anomaly MAY fail more than one.

### Result

- The command SHALL still render the full report of FR-162 in the chosen format.
- The command SHALL state a `gate` section: the `modes` evaluated and one `failures` element `{ mode, anomaly, path }` for each failure, ordered by anomaly id and then mode.
- The command SHALL name every failing anomaly; it SHALL NOT truncate or summarise the list.
- If any failure exists, then the command SHALL exit non-zero.
- If an anomaly document in scope is unreadable (FR-162), then the command SHALL exit non-zero under any `--fail-on`, because the gate cannot say it has seen every anomaly.
- If no failure exists and no document is unreadable, then the command SHALL exit zero and `failures` SHALL be empty.
- The Markdown for an empty `failures` SHALL state that no anomaly fails the evaluated modes, naming the modes, and SHALL NOT state that the release, an anomaly or a risk is acceptable.
- The command SHALL NOT record a gate result in any file.
- If `--fail-on` is given a value other than the two modes, then the command SHALL exit non-zero with a usage message naming the valid modes and SHALL print no report.
- The gate SHALL evaluate the working tree and history at the checked-out commit, and SHALL NOT read a release identifier.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-164-AC-1 | `--fail-on unassessed` over a corpus holding two `unassessed` anomalies, one of them `closed`, exits non-zero and the `gate` section names both by id and path. | Test |
| FR-164-AC-2 | `--fail-on unassessed` over a corpus with no `unassessed` anomaly exits zero with `failures` empty and the full report still rendered. | Test |
| FR-164-AC-3 | `--fail-on unjustified-impacting` exits non-zero for an `assessed` `impacting` anomaly with no release rationale, and for a `resolved` `impacting` one with none, naming each. | Test |
| FR-164-AC-4 | The same corpus with a release rationale on each of those anomalies exits zero; an entry with an empty or placeholder (`TBD`, `n/a`, `-`, in any letter case) `author` or `rationale` does not count as a rationale. | Test |
| FR-164-AC-5 | An `impacting` anomaly `closed` as `fixed`, `duplicate` or `not-a-defect`, a `none` anomaly and an `unassessed` anomaly, none with a rationale, each yield no `unjustified-impacting` failure; an `impacting` anomaly `closed` as `wont-fix` with no rationale yields one, and the same with a rationale yields none. | Test |
| FR-164-AC-6 | Over a corpus holding an `unassessed` anomaly and an unjustified `impacting` one, `--fail-on unassessed --fail-on unjustified-impacting` lists each once under its own mode, in anomaly id then mode order. | Test |
| FR-164-AC-7 | A rationale whose text is `x` counts as a rationale and the gate does not exit non-zero for it; one whose text is `TBD` does not count. | Test |
| FR-164-AC-8 | A corpus with an unreadable anomaly document exits non-zero under `--fail-on unassessed` even when every readable anomaly is assessed, and the document is named in `unreadable`. | Test |
| FR-164-AC-9 | With no `--fail-on`, the report carries no `gate` section and exits zero for a corpus with `unassessed` anomalies. | Test |
| FR-164-AC-10 | `--fail-on` with another value exits non-zero, prints a usage message naming `unassessed` and `unjustified-impacting`, and prints no report. | Test |
| FR-164-AC-11 | Over every passing fixture, the Markdown and JSON contain no sentence or key stating that the release, an anomaly or a risk is accepted or acceptable, and no file is written. | Test |
| FR-164-AC-12 | The JSON of a failing run holds `gate.modes` and `gate.failures` with elements whose keys are exactly `mode`, `anomaly` and `path`. | Test |

## Dependencies

- **Upstream**: [FR-162](./FR-162-anomaly-report.md), [FR-163](./FR-163-anomaly-report-release-rationale.md).
- **Downstream**: a CI or release step that runs the command; the dossier export reads the `gate` section of the JSON.
