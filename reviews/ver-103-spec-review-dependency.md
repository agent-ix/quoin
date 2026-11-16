---
id: SR-20261116004330
title: "Dependency review of VER-103 formalization coverage spec"
type: SpecReview
analysis: dependency
scope: "agent-ix/quoin; PR #936; FR-202 and FR-203 relationships and Dependencies sections"
review_set: all
---

## Summary

The body describes the intended FR-202 to FR-203 order, but the machine-readable dependency references point to the wrong local requirement and omit FR-202 from FR-203's relationship graph. The declared chain is acyclic once corrected.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-202 cites Quoin FR-093 as the QSL `formalizes` source, but that local FR-093 defines evidence-producer trust; the source is quire-rs FR-094 (with FR-051, FR-062 and FR-077). Correct the link and frontmatter relationship to the actual upstream requirement. | spec/functional/FR-202-formalization-coverage-state.md:8-9,51 |
| FND-002 | medium | FR-203's body depends on FR-202, but its frontmatter relationships omit a `depends_on` edge. Add the machine-readable edge so dependency consumers see the requirement that defines these states. | spec/functional/FR-203-formalization-as-evidence-kind.md:5-11,44 |
