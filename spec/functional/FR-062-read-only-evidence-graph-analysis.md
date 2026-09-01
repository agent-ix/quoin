---
id: FR-062
title: "Read-only evidence-graph analysis views"
type: FR
relationships:
  - target: "ix://agent-ix/quoin/US-018"
    type: "implements"
  - target: "ix://agent-ix/quoin/FR-030"
    type: "requires"
  - target: "ix://agent-ix/quoin/FR-032"
    type: "requires"
  - target: "ix://agent-ix/quire-rs/FR-067"
    type: "requires"
  - target: "ix://agent-ix/quire-rs/FR-068"
    type: "requires"
---
# FR-062: Read-only evidence-graph analysis views

## Description

When a caller selects `fan-out`, `change-impact`, or `churn`, `quoin` SHALL
derive one deterministic `GraphAnalysisReport` from a validated Quire assurance
export, the retained evidence store, and the existing auditor verdicts. The
three views SHALL be available through `quoin graph fan-out`,
`quoin graph change-impact`, and `quoin graph churn`, with `--json` exposing the
same report model as the human rendering.

Every graph subcommand SHALL require `--export <json>` naming the existing
Quire assurance-export artifact, `--premises <json>` naming the caller's
accepted format, module-version, and module-schema premises, and `--audit
<json>` naming an existing FR-032 verdict artifact. `--repo` SHALL locate
retained Quoin state only; it SHALL NOT cause Quoin to invoke Quire, recompute
an audit, reconstruct the export, or discover acceptance premises from ambient
modules.

These are structural views, not measurements or assurance verdicts. Counts and
paths stay attached to their exact source revision, contract premises, and
completeness state.

## Inputs

- A Quire assurance export supplied through required `--export <json>` and
  accepted against the required `--premises <json>` input under its format,
  module-version, and module-schema premises, carrying the pinned source
  revision, live obligations, artifacts, and corpus relationships.
- [FR-030](./FR-030-evidence-store.md) bindings and their affirmation history.
- [FR-032](./FR-032-evidence-auditor.md) verdicts supplied through required
  `--audit <json>`, used without reinterpretation. The accepted audit envelope
  SHALL carry `format: "quoin-audit-envelope"`, `format_version: 1`, the source
  repository and revision, and the export format, version, and exact module
  premises, all of which SHALL match the accepted export before any verdict is
  joined. Its nested FR-032 `findings`, `healthy`, and `unevaluated` payload
  SHALL remain unchanged.
- For `change-impact`, one or more live requirement ids supplied through the
  required, repeatable `--requirement` flag.
- For `change-impact`, an optional repeated `--relation` selection. When absent,
  the exact default is `depends_on`, `derives_from`, `implements`, `mitigates`,
  `refines`, `requires`, `satisfies`, and `traces_to`; an explicit selection
  replaces rather than extends that default.

## Outputs

Every report SHALL carry:

- `view`, the Quire source repository and full revision, the accepted export
  format and module premises, and a deterministic `complete`, `incomplete`, or
  `not_computed` state;
- `gaps`, including every absent or unreadable store input, unknown relationship
  availability, unresolved binding id, and requested requirement that cannot be
  resolved; and
- one view-specific row collection plus stable sort keys.

`fan-out` rows SHALL carry the suite, its sorted distinct live obligation ids,
their owning requirement ids, the obligation count, and any unresolved binding
ids associated with the suite.

`change-impact` rows SHALL carry the reached requirement, minimum impact depth,
the exact relationship path from a requested seed, its live obligations, their
bound suites, and the auditor verdict for each binding. The report SHALL also
carry the exact relationship-kind selection used for the closure.

`churn` rows SHALL carry every live obligation, its owning requirement, sorted
bound suites, the sorted unique reaffirmation events retained for that
obligation, and the event count.

## Behavior

### Fan-out

The engine SHALL group the current binding graph by suite and count distinct
live obligation ids. Multiple symbols or repeated store entries for the same
`(suite, obligation)` SHALL NOT increase fan-out. A binding whose obligation is
absent from the accepted export SHALL appear under `unresolved_bindings` and in
`gaps`; it SHALL NOT disappear or increase the live-obligation count.

### Change-impact closure

The engine SHALL select accepted corpus relationships whose source and target
are requirement artifacts of type `StR`, `US`, `FR`, or `NFR` and whose kind is
in the effective relationship-kind selection. Plan, Task, SpecReview, and other
non-requirement artifact types SHALL never enter the requirement closure. Each
selected edge is an authored dependency from source to target.
Starting with each requested requirement at depth zero, the engine SHALL walk
the reverse dependency direction: if `source` depends on `target`, a change to
`target` exposes `source`. It SHALL preserve the relationship kind and the
lexicographically first shortest path for each reached requirement. A selected
kind absent from the accepted module vocabulary SHALL make that seed
`not_computed` rather than silently narrowing the closure.

The closure SHALL terminate on cycles, retain shared dependents, and join every
reached requirement to its live obligations and current suite bindings. The
view SHALL label those joins as change exposure. It SHALL copy each FR-032
auditor verdict separately and SHALL NOT turn a supported binding into suspect
merely because it is reachable.

### Reaffirmation churn

The engine SHALL include every live obligation, including those with zero
reaffirmations. One explicit affirmation applied to several suite bindings is
one event, deduplicated by `(obligation, who, commit, note-or-empty)`. The suites
affected by that event remain visible on the row. Rows sort by descending event
count and then obligation id.

The report SHALL describe this count as retained reaffirmation history, not as
a complete Git history or proof that a statement is unstable. Affirmations for
an obligation absent from the accepted export SHALL be reported as gaps rather
than folded into a live row.

### Availability and rendering

An invalid or unsupported Quire export SHALL be refused before any row is
returned. An absent bindings store, an unreadable required store file, or an
unknown relationship premise SHALL produce `not_computed` or `incomplete` with
an exact reason as applicable; none SHALL be rendered as a zero-result healthy
graph. Human and JSON renderers SHALL consume the same report object.

An absent or unreadable `--export`, `--premises`, or `--audit` path SHALL fail
before row construction and name the failed input. A valid export whose
declared premises do not exactly match the accepted premise input, or an audit
whose source/export identity does not exactly match that export, SHALL likewise
fail before any artifact, relationship, obligation, verdict, or row is
returned.

## Constraints

| ID | Constraint | Type | Validation |
|----|------------|------|------------|
| FR-062-CON-1 | Graph analysis SHALL neither execute a producer, suite, Quire command, Git command, update check, or network request nor write a repository or evidence-store file. | Architecture | Inspection |
| FR-062-CON-2 | The views SHALL NOT derive a trust, quality, instability, or release-readiness score or threshold classification. | Responsibility | Test |
| FR-062-CON-3 | Change exposure SHALL NOT replace, promote, or suppress an FR-032 auditor verdict. | Responsibility | Test |
| FR-062-CON-4 | The implementation SHALL consume the accepted Quire export rather than re-read specification frontmatter or build a second artifact graph. | Architecture | Inspection |

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-062-AC-1 | Fan-out reports each suite once with the exact sorted set and count of distinct live obligations; duplicate symbols and duplicate `(suite, obligation)` entries do not increase the count. | Test (TC-1249) |
| FR-062-AC-2 | A binding to an obligation absent from the accepted export is named under its suite and in report gaps, and is excluded from the live count rather than dropped or treated as live. | Test (TC-1250) |
| FR-062-AC-3 | Change impact records its default or replacement relationship-kind selection, computes the reverse transitive dependency closure from every requested requirement, terminates on cycles, retains shared dependents, and selects the lexicographically first shortest path deterministically. | Test (TC-1251) |
| FR-062-AC-4 | Every reached StR, US, FR, or NFR requirement is joined to all of its live obligations and bound suites with depth and path, while an unknown or non-requirement requested id is reported and returns no partial closure for that seed. | Test (TC-1252) |
| FR-062-AC-5 | Each impacted binding carries its unchanged FR-032 verdict in a field distinct from change exposure; reachability alone never changes the verdict (CON-3). | Test (TC-1253) |
| FR-062-AC-6 | Churn deduplicates one affirmation copied across several suite bindings by `(obligation, who, commit, note-or-empty)` while retaining every affected suite on the row. | Test (TC-1254) |
| FR-062-AC-7 | Churn includes live obligations with zero events and sorts rows by descending event count then obligation id; affirmation history for an absent obligation is reported as a gap. | Test (TC-1255) |
| FR-062-AC-8 | Every view carries the exact accepted source revision and export/module premises; repeated analysis over identical accepted inputs preserves those fields byte-for-byte. | Test (TC-1256) |
| FR-062-AC-9 | Invalid inputs, absent or unreadable required `--export`/`--premises`/`--audit` paths, rejected premises, and mismatched audit identity fail before rows; absent, unreadable, incomplete, and empty retained inputs produce distinct report states and reasons, never an invented healthy zero. | Test (TC-1257) |
| FR-062-AC-10 | Reordered equivalent inputs produce byte-identical JSON, and human and JSON renderings are projections of the same sorted report object. | Test (TC-1258) |
| FR-062-AC-11 | Static boundaries and a non-JSON command-path regression prove the three views execute and write nothing, disable the inherited update check, and consume no frontmatter reader or independently built graph (CON-1, CON-4). | Inspection (TC-1259) |
| FR-062-AC-12 | Existing evidence audit, assurance-case, and measurement reports remain byte-compatible when graph analysis is not invoked, and graph reports contain no derived score or threshold label (CON-2). | Test (TC-1260) |

## Dependencies

- **Upstream**: [FR-030](./FR-030-evidence-store.md),
  [FR-032](./FR-032-evidence-auditor.md), and the stable source-grounded export
  defined by `agent-ix/quire-rs` FR-067 and FR-068.
- **Downstream**: `agent-ix/quoin#281` incorporates these rows into governed
  portfolio reporting without changing their population or completeness
  semantics.
