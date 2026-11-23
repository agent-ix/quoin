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
