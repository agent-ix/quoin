---
id: SR-20261123202512
title: "Failure-domain review of release snapshot risk-completeness and anomaly-gate open findings"
type: SpecReview
analysis: failure-domain
scope: "agent-ix/quoin; spec/functional/FR-166-release-snapshot-command.md, spec/functional/FR-167-release-snapshot-format.md"
review_set: subset
---

## Summary

Ticket: PLAT-1808 (PR agent-ix/quoin#1033). Failure-domain review of the new risk and anomaly-gate paths in the
release snapshot: unstated failure modes, silent omission and environment dependence. The exit-2 refusal, gate
failure as a finding (not a refusal), the no-risk-records case and the dedupe identity are stated. One silent
omission path and one unstated input remain.

## Findings

| ID      | Severity | Summary | Refs |
| ------- | -------- | ------- | ---- |
| FND-001 | medium | `quoin risk-summary` renders with exit 0 when the reader cannot load a file below scope, naming it only on stderr (FR-169; `Summary.unreadable`). FR-166 states nothing for this case, so the snapshot exits 0 and the record vanishes unannounced: an unreadable `residual_risk` file turns `accepted-outside-acceptable-region` into `post-control-estimate-without-decision`, an unreadable hazard turns into `situation-without-hazard`. This contradicts the PR's own rule that a finding cannot be omitted unannounced and the anomaly path, which refuses on a mapping error. Refuse with `release_snapshot_input_unreadable` naming the file, as FR-166-AC-10 does for scan files, and add an AC. | FR-166, FR-169, FR-166-AC-10 |
| FND-002 | low | With no `--module`, the risk findings depend on the installed module set (`IX_FILAMENT_MODULES_PATH` or the install root): a different safety module version can change which estimates pass their contract and so which findings appear. NFR-036 names the inputs as repository, `--as-of` and executable version, and its verification varies environment variables. The matrix already reads modules this way, so the dependence is not new in kind, but the module set should be named as an input. | FR-166, NFR-036 |

## Verdict

FND-001 should be fixed before merge; FND-002 is a wording gap.

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | FR-166 now refuses with `release_snapshot_input_unreadable` when a file `quoin risk-summary` reads cannot be read, and FR-166-AC-14 tests it; the remaining gaps in that rule are FND-003 and FND-004 below. |
| FND-002 | fixed | NFR-036 now names the installed module set (the `--module` roots, or the installed modules) as an input in its statement and metric. |

## New findings (disposition pass 1)

| ID      | Severity | Summary | Refs |
| ------- | -------- | ------- | ---- |
| FND-003 | medium | The new refusal is broader than its AC. `quoin risk-summary` reads every non-ignored `.md` file below scope (the quire walker reads all markdown before it knows a type), and a file that is not valid UTF-8 or cannot be opened is unreadable whatever it holds. So the FR-166 behavior refuses a repository with no risk records because of, for example, a Windows-1252 `docs/notes.md`; before this PR that snapshot was written. FR-166-AC-14 tests only "a risk record file", which the reader cannot tell apart. State the rule as any markdown file below `--repo` the reader cannot read, and have AC-14 cover a non-risk file, or narrow the rule. | FR-166, FR-166-AC-14, FR-169 |
| FND-004 | medium | A risk record whose frontmatter does not parse is still dropped silently. `summarize` reads the corpus with `refuse_unreadable` false and ignores `corpus.malformed`; such a file is not "unreadable", so the new refusal does not cover it. A `residual_risk` with broken YAML frontmatter vanishes, and its accepted-outside-acceptable-region finding becomes post-control-estimate-without-decision with exit 0. `quoin risk discrepancies` already refuses both cases (`refuse_unreadable` true, `document_unparseable`). Extend the refusal and AC-14 to a file whose frontmatter does not parse. | FR-166, FR-166-AC-14, FR-169 |
