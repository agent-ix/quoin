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
  loader-invocation metric, the population is the complete canonical path set of
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
| Quire `Spec` loader invocations over the selected module-document set per command invocation (NFR-011-M-2) | 1 | 1 | Test |

## Verification

A generated bundle of 250 documents is walked by each bundle-scale command, and
the elapsed time asserted against the threshold. For NFR-011-M-2, each command
invocation loads one Quire `Spec` for its selected module-document set and reuses
that returned document set across its consumers. The metric counts Quoin's
loader invocations and checks the canonical selected path set exposed by the
`Spec`; it does not count filesystem syscalls, bytes, or reads performed inside
the `Spec::from_path` constructor. Quoin makes no claim about that constructor's
per-file filesystem behavior. The same selected-document population applies to
all four commands. Evidence audit includes its coverage obligations before
joining them to the evidence store, so its selected set remains part of this
metric; a zero-loader audit is not a corpus-independent exception.

The selected population is command-specific only through its repository and
module roots: `advise` uses its `--repo` and `--module` roots; evidence audit
uses the roots captured in `AuditOptions`; matrix uses the same roots as its
shared audit assembly; completeness uses its `--repo` and `--module` roots.
Each command's exact path set is the canonical set Quire selects for coverage
under those inputs. Evidence-store records and matrix run observations are not
members of that set.

Quoin's boundary is the public Quire `Spec` API: `Spec::from_path` constructs a
corpus and `Spec::documents()` exposes the selected loaded-document set.
NFR-011-M-2 guarantees one Quoin loader invocation over the exact canonical
path set selected by Quire's coverage rules, followed by reuse of that returned
`Spec` or snapshot by command consumers. It does not specify or test how many
filesystem syscalls the Quire constructor performs. Completeness returns its
loaded snapshot to the CLI, which also reuses it for the overlap with its
`--bundle` corpus. Matrix passes one Quire `Spec` to coverage and design-matrix
computation. Advise and evidence audit still need to adopt and test an
invocation-scoped snapshot.

Tests compare the selected path set with the existing `quire.coverage` rules.
Snapshot-backed routes assert that the snapshot contains the selected canonical
paths and that command consumers reuse it rather than reopening selected paths.
Matrix asserts one `Spec` loader invocation, the exact canonical selected path
set returned by that `Spec`, and reuse for coverage and design-matrix
computation. These tests observe Quoin's loader boundary and consumer behavior;
they do not observe filesystem syscalls inside Quire's constructor. Tests fail
if a selected path is omitted, a command invokes its loader more than once, or
a consumer reopens a selected path.

**Implementation status:** completeness reuses the selected snapshot, including
documents that overlap its `--bundle` corpus. Matrix loads one Quire `Spec` and
reuses it for coverage and design-matrix computation; a traced test asserts one
loader invocation and the exact canonical fixture path set. Advise and evidence
audit remain open and NFR-011-M-2 is not met until those routes use and test the
shared snapshot boundary. The 250-document timing measurements also remain
open. The gap is `agent-ix/quoin#133`.

## Dependencies

- **Upstream**: [FR-037](../functional/FR-037-declared-vocabulary-completeness.md),
  [FR-032](../functional/FR-032-evidence-auditor.md) — the bundle-scale commands.
- **Downstream**: [NFR-012](./NFR-012-ecosystem-compatibility.md), whose corpus-wide
  sweeps multiply this cost by the repository count.
