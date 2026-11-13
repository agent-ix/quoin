---
id: MP-250
title: Commands that fail on a bad argument say why and exit non-zero
type: MeasurementPlan
status: active
owner: quoin-maintainers
stage: gate
metric: nfr003.actionable_failure_rate
definition_version: nfr003.actionable-failure-v1
ground_truth_kind: mechanical
statistical_design:
  population: every command the binary's help describes crossed with each NFR-003 error class that applies to it, namely unknown-flag, unknown-subcommand, missing-argument, missing-path, unknown-type, malformed-input and catalog-conflict
  sampling: complete census of the pairs, each class enumerated from the binary's own help text
  repetitions: 1
  estimator: proportion
  population_set:
    ids:
      - "advise --module:missing-path"
      - "advise --repo:missing-path"
      - "advise:unknown-flag"
      - "anomaly:unknown-flag"
      - "anomaly check --module:missing-path"
      - "anomaly check scope:missing-path"
      - "anomaly check:unknown-flag"
      - "anomaly refs --scope:missing-path"
      - "anomaly refs:missing-argument"
      - "anomaly refs:unknown-flag"
      - "anomaly:unknown-subcommand"
      - "assurance:unknown-flag"
      - "catalog list:unknown-flag"
      - "catalog methods:unknown-flag"
      - "catalog show type:unknown-type"
      - "catalog show:unknown-flag"
      - "catalog validate:duplicate-type"
      - "catalog validate:unknown-flag"
      - "catalog:unknown-flag"
      - "catalog:unknown-subcommand"
      - "change-assurance intake:missing-argument"
      - "change-assurance intake:unknown-flag"
      - "change-assurance prepare-attestation:missing-argument"
      - "change-assurance prepare-attestation:unknown-flag"
      - "change-assurance receipt:missing-argument"
      - "change-assurance receipt:unknown-flag"
      - "change-assurance recover --repo:missing-path"
      - "change-assurance recover:unknown-flag"
      - "change-assurance schema:unknown-flag"
      - "change-assurance verify-receipt:missing-argument"
      - "change-assurance verify-receipt:unknown-flag"
      - "change-assurance write-record --input:malformed-file"
      - "change-assurance write-record --input:malformed-stdin"
      - "change-assurance write-record:missing-argument"
      - "change-assurance write-record:unknown-flag"
      - "change-assurance:unknown-flag"
      - "change-assurance:unknown-subcommand"
      - "completeness --bundle:missing-path"
      - "completeness --module:missing-path"
      - "completeness --repo:missing-path"
      - "completeness:unknown-flag"
      - "config doctor:unknown-flag"
      - "config edit:unknown-flag"
      - "config get:missing-argument"
      - "config get:unknown-flag"
      - "config set:missing-argument"
      - "config set:unknown-flag"
      - "config:unknown-flag"
      - "config:unknown-subcommand"
      - "discharge:missing-argument"
      - "discharge:unknown-flag"
      - "evidence audit --module:missing-path"
      - "evidence audit --repo:missing-path"
      - "evidence audit:unknown-flag"
      - "evidence gc --repo:missing-path"
      - "evidence gc:unknown-flag"
      - "evidence inspect-mocks:missing-argument"
      - "evidence inspect-mocks:unknown-flag"
      - "evidence record-discharge:missing-argument"
      - "evidence record-discharge:unknown-flag"
      - "evidence record-experiment --input:malformed-file"
      - "evidence record-experiment --input:malformed-stdin"
      - "evidence record-experiment --input:missing-path"
      - "evidence record-experiment:missing-argument"
      - "evidence record-experiment:unknown-flag"
      - "evidence record-operational --input:malformed-file"
      - "evidence record-operational --input:malformed-stdin"
      - "evidence record-operational --input:missing-path"
      - "evidence record-operational:missing-argument"
      - "evidence record-operational:unknown-flag"
      - "evidence record:unknown-flag"
      - "evidence trust --decision:malformed-file"
      - "evidence trust --decision:malformed-stdin"
      - "evidence trust:missing-argument"
      - "evidence trust:unknown-flag"
      - "evidence:unknown-flag"
      - "evidence:unknown-subcommand"
      - "graph change-impact:missing-argument"
      - "graph change-impact:unknown-flag"
      - "graph fan-out:missing-argument"
      - "graph fan-out:unknown-flag"
      - "graph:unknown-flag"
      - "graph:unknown-subcommand"
      - "matrix --module:missing-path"
      - "matrix --repo:missing-path"
      - "matrix:unknown-flag"
      - "measurement intervention --definition:missing-path"
      - "measurement intervention:missing-argument"
      - "measurement intervention:unknown-flag"
      - "measurement operational-release --definition:missing-path"
      - "measurement operational-release:missing-argument"
      - "measurement operational-release:unknown-flag"
      - "measurement record --input:malformed-file"
      - "measurement record --input:malformed-stdin"
      - "measurement record --input:missing-path"
      - "measurement record:missing-argument"
      - "measurement record:unknown-flag"
      - "measurement verify:unknown-flag"
      - "measurement:unknown-flag"
      - "measurement:unknown-subcommand"
      - "module ensure-defaults:unknown-flag"
      - "module install:unknown-flag"
      - "module list:unknown-flag"
      - "module remove:unknown-flag"
      - "module:unknown-flag"
      - "module:unknown-subcommand"
      - "quoin:unknown-flag"
      - "quoin:unknown-subcommand"
      - "report --portfolio:missing-path"
      - "report --repo:missing-path"
      - "report:unknown-flag"
      - "review:unknown-flag"
      - "soup promote --module:missing-path"
      - "soup promote --repo:missing-path"
      - "soup promote:missing-argument"
      - "soup promote:unknown-flag"
      - "soup:unknown-flag"
      - "soup:unknown-subcommand"
      - "to-plan:unknown-flag"
      - "update:unknown-flag"
      - "validate --repo:missing-path"
      - "validate:unknown-flag"
      - "write --types:unknown-type"
      - "write repo_dir:missing-path"
      - "write:unknown-flag"
  error_model: none; each id is probed once against the real binary and the outcome is read directly from its exit status and streams
  uncertainty: none; a complete census of the declared set, not a sample
  decision_rule:
    comparator: ge
    threshold: 1.0
negative_controls:
  - kind: suppressed-observation
    description: an id left out of examined_ids is refused as population_partial, so a producer cannot pass by probing a convenient subset
relationships:
  - target: ix://agent-ix/quoin/NFR-003
    type: measures
  - target: ix://agent-ix/quoin/FR-131
    type: references
---

# Commands that fail on a bad argument say why and exit non-zero

Measures NFR-003-M-1: failure paths that emit an actionable message or a
non-zero exit, over seven error classes.

## Decision Use

Decide whether command dispatch, argument validation and type lookup, the
failure paths NFR-003 names, surface a failure to the caller with a message the
caller can act on.

## Population

Each id is `command:class`, or `command target:class` where the class acts on
one flag or positional of the command. The root and every command reachable
through the `COMMANDS` blocks of the binary's `--help` are read by the producer,
and each class module enumerates its members from that help text. The set is
compared with `statistical_design.population_set` by the `tc_3003` test, so a
command, flag or argument added or removed without updating this plan fails that
test.

| Class | Members, found from the help text | Probe |
| --- | --- | --- |
| `unknown-flag` | the root and every command | an unknown flag |
| `unknown-subcommand` | the root and every command that lists subcommands | a word that names no subcommand |
| `missing-argument` | every command without subcommands that marks a flag `(required)` or lists an unbracketed positional | the command run bare |
| `missing-path` | every flag or positional described as a directory, a root or a JSON path, on a command that requires nothing else | a path that does not exist |
| `unknown-type` | every flag or positional named `type` or `types` | a type the catalog does not have |
| `malformed-input` | every flag whose help says it reads stdin, on a command that requires nothing else | `{not json` as a file and on stdin |
| `catalog-conflict` | every command whose summary says it checks for duplicate types | two modules declaring the same type |

A command with a required argument besides the one probed is outside
`missing-path` and `malformed-input`: a placeholder for that argument is read
first and decides the outcome. Every probe runs in a home seeded from the
retained catalog fixture with two modules in scope, and with `HOME`,
`XDG_CONFIG_HOME` and `IX_CONFIG_ROOT` isolated from the developer's own, so no
probe reaches the network or reads a user config.

## Measure Definition

A pair matches when the process exits non-zero and its stderr is actionable for
the class, that is it names the cause and points to a next step:

- `unknown-flag`, `unknown-subcommand`: names the offending token and shows
  usage, the help or the available commands. Tokens and next-step phrases are
  matched as whole words, so `available` is not found in `unavailable`.
- `missing-argument`: names every flag and positional the help marks required
  and shows usage or the help.
- `missing-path`: names the path and says what is wrong with it (not a
  directory, no such file, cannot read).
- `unknown-type`: names the type and lists every available type.
- `malformed-input`: says the input is JSON, or names the flag, and gives the
  position of the fault (line, column or byte).
- `catalog-conflict`: names the type and both modules.

The value is matching pairs over pairs examined, definition
`nfr003.actionable-failure-v1`. The test for the unknown-flag class previously
required only that stderr was non-empty; it now requires the message to name
the flag.

## Collection Procedure

`rust/crates/quoin-cli/tests/tc_3003_nfr_populations.rs` runs every class's
probes against the built `quoin` binary and records `population.examined_ids`
and `population.matched_ids`. Each class lives in its own module under
`rust/crates/quoin-cli/tests/nfr_populations/` and has its own test
(TC-3700 to TC-3706) naming the ids that read below the target today. Set
`NFR_COLLECTION_OUT` to a directory to keep the collection it admits.

## Interpretation

100% means every probed failure names its cause and a next step. A value below
100% lists the ids that do not: the per-class tests name them, and fixing a
command fails its test until the id is removed from that list. `report
--portfolio` reads below the target because a missing member is listed on stdout
rather than refused; whether that is a defect is a design decision NFR-003 does
not state. The population is the failures the help text lets the producer
construct; a failure that needs a state the producer does not stage (a corrupt
evidence store, a module that declares a vocabulary) is not in it.
