---
id: FR-084
title: "Pin and enumerate the governed corpus"
type: FR
relationships:
  - target: "ix://agent-ix/quoin/US-022"
    type: "traces_to"
---

# FR-084: Pin and enumerate the governed corpus

## Description

The corpus measurement SHALL enumerate the governed corpus from a declared workspace root and record,
for every enumerated repository, its origin URL, its resolved commit, and whether its working tree
was clean, so that the population of the measurement is stated rather than implied.

## Rationale

A measurement whose population is not stated is not a measurement. The two published figures this
programme had to withdraw were both withdrawn because nobody could say afterwards what had been
counted. Recording the pin at enumeration time makes the census reproducible and makes a later
disagreement about a number resolvable by re-running it.

## Inputs

- A declared workspace root containing candidate repository directories.
- A declared exclusion vocabulary of directory names that are never corpus content.

## Outputs

- A corpus record listing each repository with `origin`, `commit`, `clean`, and its document count.
- A per-repository `excluded` record for every candidate directory the enumeration rejected, with the
  rule that rejected it.

## Behavior

- The measurement SHALL treat a directory as a corpus repository when it contains a `.git` directory
  and a `spec` directory, and SHALL NOT treat a Git worktree link file as a repository, so that a
  repository's worktrees are not counted as additional repositories.
- The measurement SHALL exclude any path segment beginning with `.` and any segment in the declared
  exclusion vocabulary, and SHALL record the exclusion vocabulary in the corpus record.
- If a repository has no `origin` remote, then the measurement SHALL record the repository with
  `origin: null` and SHALL retain it in the population rather than dropping it.
- While a repository's working tree is dirty, the measurement SHALL record `clean: false` against
  that repository, because its measured documents are not the documents at the recorded commit.
- The measurement SHALL enumerate every `*.md` file under each retained repository as one corpus
  document and SHALL record the count per repository.

## Constraints

| ID | Constraint | Type | Validation |
| --- | --- | --- | --- |
| FR-084-CON-1 | Enumeration SHALL NOT write to, or inside, any enumerated repository. | Safety | Test |
| FR-084-CON-2 | A repository excluded from the population SHALL carry the rule that excluded it, never an empty reason. | Interface | Test |

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-084-AC-1 | A directory containing both `.git/` and `spec/` is enumerated as one repository carrying `origin`, `commit` and `clean`. | Test (TC-1500) |
| FR-084-AC-2 | A Git worktree of an already-enumerated repository, whose `.git` is a file, is not enumerated as a second repository and is recorded as excluded with the worktree rule. | Test (TC-1501) |
| FR-084-AC-3 | A repository with no `origin` remote is retained with `origin: null` and its documents are still counted. | Test (TC-1502) |
| FR-084-AC-4 | A repository whose working tree carries an uncommitted change is recorded `clean: false`. | Test (TC-1503) |
| FR-084-AC-5 | Files under a dot-prefixed directory and under each declared excluded directory name are absent from the document count, and the exclusion vocabulary appears verbatim in the corpus record. | Test (TC-1504) |
| FR-084-AC-6 | The sum of the per-repository document counts equals the total document count published for the corpus. | Test (TC-1505) |

## Dependencies

- **Downstream**: [FR-086](./FR-086-assign-one-measurement-state-per-document.md) consumes the enumerated documents.
