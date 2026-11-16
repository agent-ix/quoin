---
id: FR-189
title: "Formalize-requirement workflow"
type: FR
relationships:
  - target: "ix://agent-ix/quoin/StR-004"
    type: "traces_to"
  - target: "ix://agent-ix/quoin/FR-188"
    type: "depends_on"
  - target: "ix://agent-ix/quoin/FR-021"
    type: "depends_on"
  - target: "ix://agent-ix/quoin/FR-190"
    type: "depends_on"
---

# FR-189: Formalize-requirement workflow

## Description

Quoin SHALL ship an ix-flow workflow that drafts a QSL formalization for a
requirement, rejects drafts that fail QSL check or contradict approved rules
before any person reviews them, and reaches `approved` only through a `hitl`
gate whose invariant requires the [FR-188](./FR-188-formal-rule-approval-state-from-signed-commits.md)
`approved` state. Origin: VER-98.

The skill is `formalize-requirement`, laid out like `skills/spec-review/`
(`SKILL.md` plus `workflows/<name>/def.yaml`), with
`ix-flow-invariant-provider` set to `quoin workflow-invariants`. Invariants are
served by the existing `quoin.workflow-invariants/v1` provider
([FR-021](./FR-021-launch-ix-flow-runs.md)).

The analysis invariant consumes an explicit `filament.contract-package/v1`
input, the package protocol accepted by filament-analyze FR-009.
QSL-1900 produces the native `filament.checked-package/v2` artifact; the bridge
from that artifact into the required `filament.contract-package/v1` package is
tracked by VER-109. Real QSL-source analysis depends on that bridge. This FR
defines neither QSL's native package nor the bridge, and adds no compatibility
adapter to filament-analyze.

## Behavior

- The workflow definition SHALL name its phases exactly `drafting`, `checked`,
  `analysed`, `awaiting_approval`, `approved`, `rejected` and `stopped`.
- The transitions SHALL be:
  - `drafting` → `checked` (auto), guarded by invariant `qsl_check_passes`.
  - `checked` → `analysed` (auto).
  - `analysed` → `awaiting_approval` (auto), guarded by invariant
    `analysis_consistent_with_approved`.
  - `awaiting_approval` → `approved` (`hitl`), guarded by invariant
    `formal_rule_approved`.
  - `awaiting_approval` → `rejected` (`hitl`).
  - `checked` → `drafting` and `analysed` → `drafting`, taken when the guard of
    the phase's outgoing transition fails; each carries the diagnostics.
  - `drafting` → `stopped` (terminal), taken on the fourth consecutive
    failure: the initial draft attempt plus three retries.
- The `formalize-requirement` skill procedure SHALL own the retry counter: it
  counts consecutive failed attempts for the run, resets the count when a draft
  reaches `awaiting_approval`, and takes `drafting` → `stopped` rather than
  re-drafting when the count reaches 4. `N = 3` means three retries after the
  initial attempt.
- The drafted set of a run SHALL be the symbol ids of the formalized
  declarations in the draft file the run last passed to `qsl_check_passes`,
  recorded by the skill procedure when `drafting` → `checked` is taken.
- Invariant `qsl_check_passes` SHALL pass when QSL `check` on the draft file
  exits clean (quire-driver `quire-cli`, `Operation::Check`).
- Invariant `analysis_consistent_with_approved` SHALL pass when a
  `filament-analyze run-package` over a package holding the draft and the approved
  rules, given one `--select` document containing only the drafted set's spans,
  reports no failing claim; it SHALL fail on any failing claim. Approved rules
  remain in the package as unselected peers. Under filament-analyze FR-009,
  selected claims are the draft claims being checked, while unselected claims
  provide the approved context used for contradiction and redundancy analysis.
  The selection spans come from the drafted set in the
  `quoin formal-rules status` report and SHALL NOT include approved-rule spans.
  The document is the single claim-selection document of filament-analyze
  FR-009.
- When Quoin evaluates `analysis_consistent_with_approved` for a
  `formalization_request`, then Quoin SHALL require `analysis_package` to be a
  non-empty, absolute path to a readable `filament.contract-package/v1` JSON
  document prepared from the exact `draft_path` that passed
  `qsl_check_passes` and the formal rules that the FR-188 status report marks
  `approved` for that run. A trusted package producer SHALL prepare the package
  from those inputs; the drafting model or agent SHALL NOT supply or replace the
  package contents. Quoin SHALL use this explicit path as the package input to
  `filament-analyze run-package` and SHALL NOT infer a package path from the
  draft, repository configuration, or ambient environment.
- When Quoin evaluates `analysis_consistent_with_approved`, then Quoin SHALL
  verify that every claim source span in the package maps to either a symbol in
  the run's drafted set at `draft_path` or a symbol marked `approved` by the
  FR-188 status report, and that the package contains no other claim. If any
  claim is missing, extra, or maps to a different draft or rule, Quoin SHALL
  fail with `analysis_package_invalid` without invoking
  `filament-analyze run-package`.
- When Quoin evaluates `analysis_consistent_with_approved`, then Quoin SHALL
  return `analysis_package_invalid` without invoking `filament-analyze` if
  `analysis_package` is missing, empty, relative, contains NUL, or cannot be
  read as a `filament.contract-package/v1` document.
- If the drafted set provides no span for the selection, then invariant
  `analysis_consistent_with_approved` SHALL fail.
- The workflow SHALL NOT invoke `filament-analyze run-package` with an empty
  selection; an empty selection is never a valid analysis request. The absence
  of approved rules SHALL NOT make the selection empty when the drafted set
  contributes spans.
- Invariant `formal_rule_approved` SHALL pass only when the drafted set is
  non-empty and `quoin formal-rules status` reports `approved` for every symbol
  in it; an empty drafted set SHALL fail it.
- The `formalization_request` SHALL carry `engineering_assurance` and
  `allowed_signers` as non-empty absolute paths, respectively naming the
  engineering-assurance executable and the allowed-signers file for
  `quoin formal-rules status`. It MAY carry a `gitsign` object with exactly
  `certificate_identity`, `certificate_oidc_issuer`, and `principal`. Each
  value SHALL meet the corresponding validation rules in Engineering
  Assurance FR-040. If `gitsign` is present, all three members SHALL be
  present, and the request schema SHALL reject unknown `gitsign` members.
- The `formal_rule_approved` invariant SHALL validate the trust fields before
  it invokes engineering-assurance. If a path is missing, empty, relative, or
  contains NUL, or if the `gitsign` object is invalid, the invariant SHALL
  return `formal_rule_trust_inputs_invalid` and SHALL NOT invoke the executable.
  For a valid request, the invariant SHALL pass the paths and optional
  `gitsign` values explicitly to `quoin formal-rules status` using the inputs
  defined in FR-188; it SHALL NOT substitute ambient environment or repository
  configuration for a request value.
- The workflow and the agent SHALL NOT create the approval. The ix-flow gate
  acknowledgement is never an approval. The `hitl` gate pauses
  for the person; the invariant passes only after the person's signed commit
  exists.
- The workflow definition SHALL define no transition into `approved` other than
  the `hitl` gate.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-189-AC-1 | Given a scripted drafting step that yields a rule that passes QSL check, the run reaches `awaiting_approval` | Test |
| FR-189-AC-2 | Given a draft that fails QSL check, the run returns to `drafting` with the diagnostics and an incremented retry counter | Test |
| FR-189-AC-3 | Given four consecutive failed attempts (the initial draft attempt plus three retries), the run ends in `stopped` and its final report names the diagnostics from the fourth attempt | Test |
| FR-189-AC-4 | Given a run at `awaiting_approval` whose gate is acknowledged and no signed approval commit exists, advancing to `approved` fails invariant `formal_rule_approved` | Test |
| FR-189-AC-5 | Given the same run after a person's signed approval commit lands, advancing to `approved` passes | Test |
| FR-189-AC-6 | The `def.yaml` holds no transition into `approved` other than the one with the `hitl` gate | Inspection |
| FR-189-AC-7 | The phase names and transitions in `def.yaml` equal the set fixed above | Inspection |
| FR-189-AC-8 | Given a commit signed by a key not in the allowed-signers file, invariant `formal_rule_approved` fails | Test |
| FR-189-AC-9 | `def.yaml` names `quoin workflow-invariants` as the `ix-flow-invariant-provider` and the provider answers each of the three invariants | Test |
| FR-189-AC-10 | Given a run with an empty drafted set at `awaiting_approval`, invariant `formal_rule_approved` fails | Test |
| FR-189-AC-11 | Given a draft whose analysis reports a failing claim, the run returns from `analysed` to `drafting` with the diagnostics and an incremented retry counter | Test |
| FR-189-AC-12 | Given two consecutive failures followed by a passing draft, the run reaches `awaiting_approval` and the next failure count starts at 1 | Test |
| FR-189-AC-13 | Given a repository with no `approved` rule and a drafted set that holds a span, the analysis runs with a selection of the drafted spans only | Test |
| FR-189-AC-14 | Given a drafted set that provides no selection span, invariant `analysis_consistent_with_approved` fails and `filament-analyze run-package` is not invoked | Test |
| FR-189-AC-15 | Given a missing, empty, relative, or NUL-containing trust path, or a partial or invalid `gitsign` object in `formalization_request`, the approval invariant returns `formal_rule_trust_inputs_invalid` without invoking engineering-assurance | Test |
| FR-189-AC-16 | Given an `analysis_package` prepared from the exact draft that passed `qsl_check_passes` and the FR-188 rules reported approved for the run, the analysis invariant invokes `filament-analyze run-package` with that `filament.contract-package/v1` package and one selection containing only the drafted spans | Test |
| FR-189-AC-17 | Given a missing, empty, relative, or NUL-containing `analysis_package`, or a path that cannot be read as `filament.contract-package/v1`, the analysis invariant returns `analysis_package_invalid` without invoking `filament-analyze` | Test |
| FR-189-AC-18 | Given a package whose claims are missing from, extra to, or source-mismatched with the checked draft and FR-188 approved-rule set, the analysis invariant returns `analysis_package_invalid` without invoking `filament-analyze run-package` | Test |
| FR-189-AC-19 | Given an approved rule and a drafted rule that contradicts it, the package contains both claims, the selection contains only the drafted span, and the analysis reports the contradiction against the unselected approved peer | Test |

## Dependencies

- **Upstream**: [FR-188](./FR-188-formal-rule-approval-state-from-signed-commits.md),
  [FR-021](./FR-021-launch-ix-flow-runs.md), [FR-190](./FR-190-formal-rule-analysis-claim-selection.md).
  Real QSL-source analysis additionally depends on QSL-1900's native checked package and VER-109's bridge from
  `filament.checked-package/v2` into `filament.contract-package/v1`.
