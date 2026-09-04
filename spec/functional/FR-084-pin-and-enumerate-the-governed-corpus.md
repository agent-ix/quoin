---
id: FR-084
title: "Pin and enumerate the governed corpus"
type: FR
relationships:
  - target: "ix://agent-ix/quoin/US-022"
    type: "traces_to"
  - target: "ix://agent-ix/quoin/FR-092"
    type: "depends_on"
---

# FR-084: Pin and enumerate the governed corpus

## Description

The corpus measurement SHALL enumerate the governed corpus from a declared workspace root under a
declared exclusion vocabulary, recording for every enumerated repository its origin URL, its resolved
commit, its working-tree cleanliness and its document count.

## Rationale

A measurement whose population is not stated is not a measurement. This programme has withdrawn two
published figures, both because nobody could afterwards say what had been counted. The same workspace
yields 331,702, 24,643 or 7,587 Markdown documents depending on which directories are excluded, so
the exclusion vocabulary is an input somebody authors, not a detail the walker decides.

## Inputs

- A declared workspace root containing candidate repository directories.
- A declared exclusion vocabulary of directory names that are never corpus content.
- A declared corpus identifier naming this population and distinguishing it from the `quire-rs#385`
  fixture corpus, which is a different population measured separately.

## Outputs

- A corpus record listing each repository with `origin`, `commit`, `clean`, `stable` and its document
  count, plus the declared exclusion vocabulary and corpus identifier verbatim.
- An `excluded` record for every candidate directory or path the enumeration rejected, naming the
  rule that rejected it.

## Behavior

- The measurement SHALL treat a directory as a corpus repository when it contains a `.git` directory
  and a `spec` directory.
- When a candidate directory's `.git` is a file rather than a directory, the measurement SHALL record
  that directory `excluded` under the rule `git-link-file`, so that a worktree or a submodule is not
  counted as a second copy of the repository it belongs to.
- When a directory below an enumerated repository's root contains a `.git` entry of either kind, the
  measurement SHALL record that subtree `excluded` from the enclosing repository under the rule
  `nested-repository`.
- When a subtree is excluded under `nested-repository`, the measurement SHALL evaluate that directory
  separately against the repository rule.
- The measurement SHALL NOT traverse a symbolic link.
- The measurement SHALL record every symbolic link it declines under the rule `symlink`.
- The measurement SHALL exclude any path segment beginning with `.` and any segment named in the
  declared exclusion vocabulary.
- If a repository has no `origin` remote, then the measurement SHALL record the repository with
  `origin: null` and SHALL retain it in the population.
- While a repository's working tree is dirty, the measurement SHALL record `clean: false` against that
  repository, because its measured documents are not the documents at the recorded commit.
- The measurement SHALL re-read each repository's `HEAD` and working-tree cleanliness after
  enumerating its documents, and SHALL record `stable: false` when either differs from the value read
  before.
- The measurement SHALL enumerate every `*.md` file under each retained repository as one corpus
  document.

## Constraints

| ID | Constraint | Type | Validation |
| --- | --- | --- | --- |
| FR-084-CON-1 | The working tree and refs of every enumerated repository SHALL be unchanged by a measurement run; incidental Git index or lock activity from a read-only plumbing call is not a change. | Safety | Test |
| FR-084-CON-2 | Every excluded candidate SHALL carry the rule that excluded it, never an empty reason. | Interface | Test |
| FR-084-CON-3 | The exclusion vocabulary SHALL be a declared input, never a list compiled into the measurement. | Interface | Test |

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-084-AC-1 | A directory containing both `.git/` and `spec/` is enumerated as one repository carrying `origin`, `commit`, `clean` and `stable`. | Test (TC-1500) |
| FR-084-AC-2 | A directory whose `.git` is a file is excluded under `git-link-file`, and a nested repository below an enumerated root is excluded from it under `nested-repository` and evaluated separately. | Test (TC-1501) |
| FR-084-AC-3 | A repository with no `origin` remote is retained with `origin: null` and its documents are still counted. | Test (TC-1502) |
| FR-084-AC-4 | A repository whose working tree carries an uncommitted change is recorded `clean: false`, and a repository whose `HEAD` moves during the run is recorded `stable: false`. | Test (TC-1503) |
| FR-084-AC-5 | Files under a dot-prefixed directory and under each declared excluded directory name are absent from the document count, and the exclusion vocabulary and corpus identifier appear verbatim in the corpus record. | Test (TC-1504) |
| FR-084-AC-6 | The sum of the per-repository document counts equals the total document count published for the corpus, and no document is counted under two repositories. | Test (TC-1505) |
| FR-084-AC-7 | A symbolic link to a directory is not traversed, is recorded under the rule `symlink`, and contributes no document; a link cycle terminates the walk. | Test (TC-1566) |
| FR-084-AC-8 | Changing only the declared exclusion vocabulary changes the enumerated document count, demonstrating the vocabulary is read rather than compiled in. | Test (TC-1567) |

## Dependencies

- **Upstream**: [FR-092](./FR-092-stay-advisory-and-read-only.md) owns the read-only envelope this enumeration runs inside.
- **Downstream**: [FR-086](./FR-086-assign-one-measurement-state-per-document.md) consumes the enumerated documents; [FR-090](./FR-090-publish-rates-with-unit-population-and-method.md) consumes the corpus record as a population identifier.
