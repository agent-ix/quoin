---
id: NFR-011
title: "Bundle-scale commands stay within a stated time budget"
type: NFR
quality_attribute: performance_efficiency
relationships:
  - target: "ix://agent-ix/quoin/StR-004"
    type: "traces_to"
  - target: "ix://agent-ix/quoin/FR-037"
    type: "constrains"
  - target: "ix://agent-ix/quoin/FR-032"
    type: "constrains"
---

# NFR-011: Bundle-scale commands stay within a stated time budget

## Statement

Every `quoin` command that walks a whole spec bundle SHALL complete within **5
seconds** on a bundle of 250 documents on ordinary developer hardware, and the
figure SHALL come from a measurement rather than an estimate.

## Scope

- Applies to: `quoin completeness`, `quoin advise`, `quoin evidence audit`,
  `quoin matrix` — the commands whose cost grows with bundle size. For the
  document-pass metric, the population is the complete canonical path set of
  requirement and evidence documents selected for those roots by the existing
  `quire.coverage` document-selection rules. Policy files, catalog metadata and
  evidence-store records are separate inputs, not members of this population.
- For `quoin completeness`, the `--bundle` Markdown corpus remains a separate
  FR-037 input. Its intersection with the selected module-document population
  reuses the snapshot contents; bundle-only Markdown files are read once by the
  completeness scanner and are outside NFR-011-M-2. This applies to the default
  bundle root and any non-default `--bundle` path.
- Operational context: a CI gate and an interactive terminal run.
- Excluded: `quire-rs` engine time, which the engine's own NFRs govern.

## Rationale

quoin has **no performance measurement of any kind** — no benchmark, no timing
assertion, no threshold anywhere in the repository. That was defensible while it
was a scaffolding tool run by hand. It is not now: `quoin evidence audit` and
`quoin completeness` are merge gates, and a gate people wait on is a gate people
route around.

The number is a **budget, not a prediction**. Its purpose is that a change making
a bundle walk an order of magnitude slower fails a test instead of being absorbed
as "CI got slower". 5s over 250 documents is roughly twice the current cost of the
most expensive command, which leaves headroom without licensing a regression.

Stating it also makes the cost model explicit: these commands are **O(documents)**
with a single pass, and any change that makes one O(documents × obligations) is a
design change, not a tuning question.

## Measurement and Evaluation

| Metric                                                        | Target | Threshold | Method |
| ------------------------------------------------------------- | ------ | --------- | ------ |
| Wall-clock for a bundle walk over 250 documents               | < 2s   | < 5s      | Test   |
| Full passes over the selected module document set per invocation (NFR-011-M-2) | 1 | 1 | Test |

## Verification

A generated bundle of 250 documents is walked by each bundle-scale command, and
the elapsed time asserted against the threshold. For NFR-011-M-2, one invocation
scoped document snapshot owns the selected module document set. Its loader reads
each selected document's complete contents once and shares those contents with
the command's consumers; directory enumeration, metadata checks, and evidence
store reads are not document reads for this metric. All four commands, including
`quoin evidence audit`, use this same snapshot because the audit obtains its
obligations from module coverage before joining them to the evidence store. A
zero-read audit therefore fails the metric rather than being treated as a
corpus-independent exception.

The selected population is command-specific only through its repository and
module roots: `advise` uses its `--repo` and `--module` roots; evidence audit
uses the roots captured in `AuditOptions`; matrix uses the same roots as its
shared audit assembly; completeness uses its `--repo` and `--module` roots.
Each command's exact path set is the canonical set Quire selects for coverage
under those inputs. Evidence-store records and matrix run observations are not
members of that set.

Quoin's host read boundary is Quire's repository loader, exposed through
`quire_rs::Spec::from_path` and its loaded-document view. The invocation-scoped
snapshot retains each selected path's full contents and identity; command
consumers receive that data rather than reopening a selected path. The first
completeness slice returns the loaded snapshot to the CLI, which also reuses it
for the overlap with its `--bundle` corpus. The other three routes must adopt
the same boundary before NFR-011-M-2 is met.

Tests instrument the sole read boundary. For each command, they compare the
observed path set with the path set selected by the existing `quire.coverage`
rules and assert exactly one complete-content read per selected path. Because
all selected-document reads must pass through the instrumented boundary, a
consumer that omits the snapshot or reopens a selected document also increments
the observed count and fails the exact-once assertion. The tests fail if a path
is omitted, read more than once, or read outside the selected set.

**Implementation status:** the first bounded slice implements the snapshot reuse
for `quoin completeness`, including selected documents that overlap its
`--bundle` corpus. Its tests assert Quire's exact selected path set and one read
result per selected path, then assert that completeness does not reopen selected
or selected-unreadable paths and reads a bundle-only path once. Per-route
exact-read tests for `advise`, `evidence audit`, and `matrix` remain open;
NFR-011-M-2 is not met until those routes use the same snapshot and their tests
pass. The 250-document timing measurements also remain open. The gap is
`agent-ix/quoin#133`.

## Dependencies

- **Upstream**: [FR-037](../functional/FR-037-declared-vocabulary-completeness.md),
  [FR-032](../functional/FR-032-evidence-auditor.md) — the bundle-scale commands.
- **Downstream**: [NFR-012](./NFR-012-ecosystem-compatibility.md), whose corpus-wide
  sweeps multiply this cost by the repository count.
