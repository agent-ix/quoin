---
id: NFR-036
title: "The release snapshot is deterministic for the same evidence and never decides a release"
type: NFR
quality_attribute: reliability
relationships:
  - target: "ix://agent-ix/quoin/FR-166"
    type: "constrains"
  - target: "ix://agent-ix/quoin/FR-167"
    type: "constrains"
  - target: "ix://agent-ix/quoin/FR-168"
    type: "constrains"
---

# NFR-036: The release snapshot is deterministic for the same evidence and never decides a release

## Statement

`quoin release snapshot` SHALL write the same bytes for the same inputs and `--as-of`.

The file `quoin release snapshot` writes SHALL state only values that other Quoin views and the supplied history file
state, with no verdict that a release is acceptable.

## Scope

- Applies to: `quoin release snapshot` and the file it writes.
- Out of scope: the release decision, which is a named person's signed tag
  (`ix://agent-ix/engineering-assurance/NFR-007`), and the placement of the file in the release commit.

## Rationale

The tag binds the snapshot through git's commit hash, so a second run over the same evidence that wrote different
bytes would make the committed file depend on when it was written. A snapshot that worded the evidence as
releasable would put a judgement in front of the person who is to make it.

## Measurement and Evaluation

| Metric | Target | Threshold | Method |
|--------|--------|-----------|--------|
| Byte differences between two runs over identical inputs and `--as-of` | 0 | 0 | Test |
| Clock, environment, locale, absolute-path or host-dependent values in the output (the repository-relative `path` members FR-167 names are not counted) | 0 | 0 | Test |
| Verdict, score, rank, digest or hash members in the output | 0 | 0 | Test |
| Files changed or created other than `--out` | 0 | 0 | Test |

## Verification

Two runs over a fixture, in different working directories, with different environment variables and a changed
clock, are compared byte for byte; the output keys and values are scanned for the members named above.
