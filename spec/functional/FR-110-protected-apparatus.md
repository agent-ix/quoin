---
id: FR-110
title: "Protected measurement apparatus: resolved at intake, compared across collections"
type: FR
relationships:
  - target: "ix://agent-ix/quoin/FR-044"
    type: "extends"
  - target: "ix://agent-ix/quoin/FR-107"
    type: "extends"
  - target: "ix://agent-ix/quoin/FR-108"
    type: "extends"
---

# FR-110: Protected measurement apparatus: resolved at intake, compared across collections

## Description

When a `MeasurementPlan` declares `protected_apparatus`
(engineering-assurance FR-024), Quoin SHALL resolve every entry to files and
digest each one when it writes a collection the plan governs, SHALL record
the resolved (path, digest) set in the stored collection, and SHALL refuse a
delta, a ratchet floor, or a checker baseline across two collections whose
recorded sets differ.

## Rationale

A collection's `verificationStack.artifacts` records a digest for each file
the producer names, and nothing compared those digests. An answer key edited
between a baseline and a new run still produced a delta, a ratchet `held`,
and a checker `accept`. The artifacts map is also the producer's own list, so
a producer could leave the edited file out of it. Quoin therefore resolves
the protected set itself, and compares what each collection recorded when it
was written, never the repository as it reads today.

## Inputs

- A `MeasurementPlan`'s optional `protected_apparatus` (repository-relative
  file paths and `<directory>/**` entries) and `negative_controls`, as
  engineering-assurance FR-024 defines them.
- A candidate collection and the repository it is written into.
- Two stored collections to compare, or every stored run of a plan.

## Outputs

- A stored collection whose `verificationStack.protectedApparatus` maps each
  governing plan's id to its resolved files and their digests, or a typed
  refusal.
- The comparison reasons `apparatus_changed` (blocking) and
  `artifact_changed` (not blocking).
- The ratchet inconclusive reasons `apparatus_changed` and
  `apparatus_unrecorded`.
- The checker reasons `apparatus_unrecorded` (inconclusive) and
  `apparatus_edit` (reject).

## Behavior

### Plan intake

- Plan intake SHALL read `protected_apparatus` and `negative_controls` into
  engineering-assurance's `ProtectedApparatus` and `NegativeControls`. A list
  engineering-assurance refuses, a `stage: gate` plan that states no
  `protected_apparatus` or no `negative_controls`, and an `apparatus-edit`
  control on a plan with no `protected_apparatus`, SHALL refuse the plan load
  as `QM-PLAN-INVALID` naming the member. The gate-stage requirement has no
  exception for a plan written before it: a gate that names nothing it
  protects gives credit a changed answer key can earn.

### Resolution at intake

- For each plan that governs an observation of the candidate and declares
  `protected_apparatus`, intake SHALL resolve every entry against the
  repository:
  - a file entry names one regular file;
  - a `<directory>/**` entry names every regular file under the directory,
    recursively, dotfiles included, and SHALL name at least one;
  - every path segment is matched exactly against the directory listing, so
    resolution is case-sensitive on every filesystem;
  - a symlink named by an entry, passed through on the way to one, or found
    under a directory entry is refused and never followed.
- Intake SHALL refuse the write, and write nothing, with:
  - `QM-APPARATUS-UNRESOLVED` when an entry names no file: nothing exists
    there, a file entry names a directory, or a directory entry's directory
    is absent or holds no file;
  - `QM-APPARATUS-SYMLINK` for a symlink;
  - `QM-APPARATUS-UNREADABLE` when a file cannot be digested or listed, or
    something under a directory entry is not a regular file, a directory or
    a symlink;
  - `QM-APPARATUS-TOO-LARGE` when one plan's set exceeds 50,000 files;
  - `QM-APPARATUS-UNDECLARED`, naming every file, when the candidate's
    `verificationStack.artifacts` omits a resolved file;
  - `QM-COLLECTION-INVALID` when it states a resolved file at a digest the
    file does not have.
- Intake SHALL refuse, as `QM-COLLECTION-INVALID`, a candidate that states
  `verificationStack.protectedApparatus` or `verificationStack.unverifiedArtifacts`:
  intake computes both, and a caller that states either is mistaken or is
  writing its own answer.
- Intake SHALL write `verificationStack.protectedApparatus` as
  `{ <plan id>: { <path>: "sha256:…" } }`, and SHALL leave the member absent
  when no governing plan protects apparatus. A protected file is always a digested artifact, so a protected
  path is never listed in `unverifiedArtifacts`.
- Reading a stored collection SHALL refuse a `protectedApparatus` that is not
  an object of non-empty objects of sha256 digests as
  `QM-COLLECTION-INVALID`. An absent member reads as no record.

### Comparison (FR-044-AC-3)

- For a slice both collections measured, comparison SHALL add the blocking
  reason `apparatus_changed` — the delta `null`, the status `incomparable` —
  when the plan's recorded sets differ: a path whose digest changed, a path
  present on one side only (a file added under or removed from a directory
  entry), a set recorded on one side only, or a protected path either side
  lists in `unverifiedArtifacts`. The message names the paths.
- When either side recorded a set for the slice's plan, comparison SHALL add
  the non-blocking reason `artifact_changed`, naming each
  `verificationStack.artifacts` entry outside the protected set whose digest
  differs or that one side alone states.
- Comparison reads no plan. Two collections that both recorded no set are
  not compared on apparatus, and comparison output for them is unchanged.

### Ratchet verdict (FR-107)

- A series is **protected** when the plan declares `protected_apparatus`, or
  when any collection in it recorded a set for the plan. Each collection's
  recorded set is read whatever the plan declares today, so removing the
  list from the plan does not switch protection off.
- In a protected series, a `ratchet` SHALL be `inconclusive` with
  `apparatus_unrecorded` when the newest collection, or a collection behind
  an earlier usable value of the slice, recorded no set, and with
  `apparatus_changed` when such an earlier collection recorded a set that
  differs from the newest one's.

### Verdict checker (FR-108)

- The checker SHALL read each run's recorded set whatever the plan declares
  today, and a series is protected as above.
- In a protected series, a run SHALL be a baseline only for a run that
  recorded the same set.
- An earlier run that recorded a set different from the candidate's SHALL be
  the finding `apparatus_edit` (reject). An earlier run, or a candidate, that
  recorded no set SHALL be `apparatus_unrecorded` (inconclusive).
- `rerun_until_pass` SHALL ignore the protected set: a regressed run followed
  by a pass of the same source revision, configuration, tool, corpus and
  verification stack is a rerun even when the apparatus changed between them,
  because editing the answer key between a failed run and a pass is the rerun
  the rule exists to catch.

### Why a changed set rejects

Every run the checker considers shares the plan's `definition_version`, and
engineering-assurance FR-024 makes a new `definition_version` unconditional
for any change to the resolved set. A recorded set that differs inside one
series is therefore a change the data proves was not versioned: a
contradiction, which rejects (FR-108-AC-5), whether or not the plan declares
the `apparatus-edit` negative control. A missing record is missing evidence
and never rejects.

### Plans that protect nothing

- A plan with no `protected_apparatus`, whose collections recorded no set,
  SHALL be written, compared, ratcheted and checked exactly as before this
  requirement.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-110-AC-1 | `protected_apparatus` and `negative_controls` load as engineering-assurance's types; an unsafe or `**` entry, an empty or repeated list, an unknown control kind, an `apparatus-edit` control with no `protected_apparatus`, and a `gate` plan missing either list refuse the plan load as `QM-PLAN-INVALID` naming the member; a `gate` plan stating both loads. | Test (TC-1810) |
| FR-110-AC-2 | A written collection records, under the plan's id, every file its entries resolve to — every file under a directory entry, dotfiles and nested files included — with each file's digest; no protected path is in `unverifiedArtifacts`; with no protecting plan the member is absent. A candidate stating `protectedApparatus` or `unverifiedArtifacts` is `QM-COLLECTION-INVALID` and writes nothing. | Test (TC-1811, TC-1819, TC-1827) |
| FR-110-AC-3 | A missing protected file, a missing or empty directory entry, a file entry naming a directory, and a case-only name difference are `QM-APPARATUS-UNRESOLVED`; a symlinked entry, ancestor, or file under a directory entry is `QM-APPARATUS-SYMLINK`; a socket under a directory entry or an unlistable directory is `QM-APPARATUS-UNREADABLE`; an artifacts map omitting a resolved file is `QM-APPARATUS-UNDECLARED` naming it, and one stating another digest is `QM-COLLECTION-INVALID`; none writes a collection. | Test (TC-1812..TC-1815) |
| FR-110-AC-4 | Comparing two stored collections, an edited protected file, a file added under or removed from a directory entry, a record on one side only, and a protected path listed as unverified each add blocking `apparatus_changed` with a `null` delta and `incomparable` status, naming the path; a moved unprotected artifact adds only non-blocking `artifact_changed` with the delta kept; under a plan that protects nothing, neither reason appears. | Test (TC-1816..TC-1819, TC-1821) |
| FR-110-AC-5 | A baseline stored before the apparatus was edited cannot be rewritten under its id, and a new run is `apparatus_changed` against it even when a regenerated baseline was stored beside the run. | Test (TC-1820) |
| FR-110-AC-6 | A ratchet whose earlier value was measured with a different recorded set is `inconclusive` (`apparatus_changed`), and one whose newest collection recorded none is `inconclusive` (`apparatus_unrecorded`), including after the list is removed from the plan; over one set it is `held`. | Test (TC-1825, TC-1828, TC-1830) |
| FR-110-AC-7 | The checker uses no baseline across a changed set: the candidate is `no_prior` and `apparatus_edit` (reject) with or without an `apparatus-edit` control, and `apparatus_unrecorded` (inconclusive) when it recorded no set; removing the list from the plan still rejects the changed series; an uncommitted answer-key edit between a regressed run and a pass is `rerun_until_pass`; the same runs over one set are accepted. | Test (TC-1822..TC-1824, TC-1828, TC-1829) |
| FR-110-AC-8 | A stored `protectedApparatus` that is not an object, holds an empty plan entry, or holds a value that is not a sha256 digest is refused on read as `QM-COLLECTION-INVALID` naming the member. An empty plan-id key, and a path key that is not a valid single-file `ApparatusPath` (a `<directory>/**` entry, an absolute path, or a `..` segment among them), are refused the same way. | Test (TC-1826, TC-1833) |
| FR-110-AC-9 | One plan's resolved set past its limit (50,000 files) is `QM-APPARATUS-TOO-LARGE`; each path segment is matched exactly against the directory listing, so a case-only difference names no file on any filesystem. | Test (TC-1831, TC-1832) |
| FR-110-AC-10 | A protected file is opened exactly once, with `O_NOFOLLOW` and `O_NONBLOCK`, and its type is read from the open handle (`fstat`), never from a second `stat` of the path: nothing can swap what the path names between the resolver's `descend` and the digest, and a FIFO fails fast rather than blocking the resolver forever waiting for a writer. A plan's directory walk is refused as `QM-APPARATUS-TOO-LARGE` past 100,000 directories visited, counting empty ones, before the file-count limit is reached; the files every plan one write governs resolve to, summed, are refused the same way past 200,000. | Test (TC-1834, TC-1835, TC-1836) |

## Constraints

- **FR-110-CON-1**: Comparison stays verdict-free (FR-107-CON-1): it withholds
  a delta and passes no judgement.
- **FR-110-CON-2**: Every comparison reads the sets the collections recorded,
  never the repository at comparison time.
- **FR-110-CON-3**: The entry grammar, list rules and control kinds are
  engineering-assurance's; Quoin states none of them a second time.

## Known limits

- **Comparison cannot see a record that was never written.** Two stored
  collections that both lack a record are compared as before, because
  comparison reads no plan. The ratchet and the checker, which hold the plan,
  refuse that case. Quoin's own intake always writes the record for a
  protecting plan, so only a collection written by other means can lack it.

## Dependencies

- engineering-assurance FR-024 defines `protected_apparatus`,
  `negative_controls`, and the resolver rules this requirement implements.
- FR-044 defines the store, intake and comparison; FR-107 the ratchet;
  FR-108 the checker.
