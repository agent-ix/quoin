---
id: MP-252
title: ix-flow failures reach the caller as a non-zero exit
type: MeasurementPlan
status: active
owner: quoin-maintainers
stage: gate
metric: nfr007.external_failure_exit_rate
definition_version: nfr007.external-failure-exit-v1
ground_truth_kind: mechanical
statistical_design:
  population: every workflow command crossed with each way ix-flow can fail, namely absent from PATH, an old or unreadable version, a hang in the handshake or the run, a non-zero exit, output that is not JSON, empty or over the bound, an error payload, a missing instance id, and a failing context step
  sampling: complete census of the pairs; the workflow commands are those whose `--help` begins `Start the Quoin-owned specification workflow`
  repetitions: 1
  estimator: proportion
  population_set:
    ids:
      - "review:ix-flow-absent"
      - "review:ix-flow-fails-the-context"
      - "review:ix-flow-fails-the-run"
      - "review:ix-flow-floods-its-output"
      - "review:ix-flow-hangs-the-handshake"
      - "review:ix-flow-hangs-the-run"
      - "review:ix-flow-omits-the-instance-id"
      - "review:ix-flow-prints-garbage"
      - "review:ix-flow-prints-nothing"
      - "review:ix-flow-reports-an-error-payload"
      - "review:ix-flow-reports-an-old-version"
      - "review:ix-flow-reports-no-version"
      - "to-plan:ix-flow-absent"
      - "to-plan:ix-flow-fails-the-context"
      - "to-plan:ix-flow-fails-the-run"
      - "to-plan:ix-flow-floods-its-output"
      - "to-plan:ix-flow-hangs-the-handshake"
      - "to-plan:ix-flow-hangs-the-run"
      - "to-plan:ix-flow-omits-the-instance-id"
      - "to-plan:ix-flow-prints-garbage"
      - "to-plan:ix-flow-prints-nothing"
      - "to-plan:ix-flow-reports-an-error-payload"
      - "to-plan:ix-flow-reports-an-old-version"
      - "to-plan:ix-flow-reports-no-version"
  error_model: none; each id is probed once against the real binary and the outcome is read directly from its exit status and streams
  uncertainty: none; a complete census of the declared set, not a sample
  decision_rule:
    comparator: ge
    threshold: 1.0
negative_controls:
  - kind: suppressed-observation
    description: an id left out of examined_ids is refused as population_partial, so a producer cannot pass by probing a convenient subset
relationships:
  - target: ix://agent-ix/quoin/NFR-007
    type: measures
  - target: ix://agent-ix/quoin/FR-131
    type: references
---

# ix-flow failures reach the caller as a non-zero exit

Measures NFR-007-M-1: external-tool failures surfaced as a non-zero `quoin`
exit. The only external tool `quoin` runs is `ix-flow`; `quire` is a linked library,
and `quoin update` fetches over HTTP.

## Decision Use

Decide whether `quoin review` and `quoin to-plan` hide an `ix-flow` failure.

## Population

Each workflow command paired with each failure mode, as `command:mode`. The
commands are found by the producer from the binary's own help text; the twelve
modes are those the producer stages with its stub, and all are in the set. The
set is compared with this plan's by the `tc_3004` test.

| Mode | The stub | The message carries |
| --- | --- | --- |
| `ix-flow-absent` | no `ix-flow` on `PATH` | `could not start ix-flow` |
| `ix-flow-reports-an-old-version` | `--version` prints 0.1.0 | `ix-flow >=0.3.1 is required` |
| `ix-flow-reports-no-version` | `--version` prints a non-version | `supported semantic version` |
| `ix-flow-hangs-the-handshake` | `--version` never returns | `launch deadline` |
| `ix-flow-fails-the-run` | `run` exits 7 with a marker on stderr | the marker |
| `ix-flow-hangs-the-run` | `run` never returns | `launch deadline` |
| `ix-flow-prints-garbage` | `run` exits 0 printing text | `ix-flow returned invalid JSON` |
| `ix-flow-prints-nothing` | `run` exits 0 printing nothing | `ix-flow returned invalid JSON` |
| `ix-flow-floods-its-output` | `run` prints 2 MB | `1 MiB output bound` |
| `ix-flow-reports-an-error-payload` | `run` exits 0 printing `{"ok":false,"error":{...}}` | the payload's message |
| `ix-flow-omits-the-instance-id` | `run` exits 0 printing `{"ok":true}` | `ix-flow returned no instance_id` |
| `ix-flow-fails-the-context` | `add-item` exits 9 with a marker | the marker |

## Measure Definition

The producer gives each command a skill directory that satisfies it, and a
`PATH` holding either nothing or the stub `ix-flow`, which follows the launch
protocol up to the step the mode breaks and logs each operation it is asked
for. A pair matches when `quoin` exits non-zero, the stub was asked for the
operation the mode belongs to, and stderr carries the fragment in the table:
the stub's marker where the message is the tool's own, `quoin`'s own wording
otherwise. A control launch against a stub that
succeeds must exit zero first, or the probe panics. Value: matching pairs over
pairs examined, definition `nfr007.external-failure-exit-v1`.

## Collection Procedure

`rust/crates/quoin-cli/tests/tc_3003_nfr_populations.rs` stages and runs each
pair against the built binary on Unix, every pair on its own thread so the modes
that run to `quoin`'s 30 second launch deadline cost one deadline, and records
the ids. The stub lives in `rust/crates/quoin-cli/tests/nfr_populations/ix_flow.rs`.

## Interpretation

100% means none of these failures is swallowed: each is a non-zero exit with a
message that says what failed. A message that names the cause but not `ix-flow`
(an error payload passed through) counts. The modes are those the stub can
stage; an `ix-flow` that succeeds but does the wrong work is not a failure
`quoin` can see.
