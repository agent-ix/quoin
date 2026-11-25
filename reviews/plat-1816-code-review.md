---
id: SR-2026112016223475846355044
title: "PLAT-1816 code-review of Quoin dossier producer identity"
type: SpecReview
analysis: code-review
scope: "agent-ix/quoin PR #1065 @85ffa442c4eb17f102d93ea1503c5502d699e26f; Rust dossier producer identity implementation and FR-191/207/215 changes"
review_set: subset
relationships:
  - {target: "ix://agent-ix/quoin/FR-191", type: references}
---
## Summary
Reviewed the Rust implementation, focused test evidence, changed requirements, and applicable repository conventions. Exact producer version comes from the same compile-time value as the running CLI's `--version`; writer serialization and focused tests are sound. One high duplication finding remains: FR-191 repeats the upstream DossierDocument producer identity shape and prohibition already owned by spec-artifacts-process FR-018.

## Verdict
FAIL — the high duplication finding is a required removal under the code-review P0 rule.

## Findings
| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | FR-191 restates the upstream DossierDocument producer identity contract. | spec/functional/FR-191-dossier-command.md:106; spec-artifacts-process FR-018 |

## Coverage
Reviewed all 19 changed paths, including Rust source, tests, and FR-191/207/215. Exact version path: `main.rs::build_version()` serves both `--version` and `dossier.rs` `producer_version`; `writer::compose` serializes it through the existing quoting helper. Focused writer tests, executable producer test, and merged-process-schema validation logs were inspected. No subject revision/hash/digest/generic provenance field was added. Quoin `0.36.8`; quire `0.52.0` (engine `0.62.2`); process module `v0.36.0`. `cargo check`, focused tests, library clippy, and focused CLI-target clippy passed per coder logs. Broad strict clippy surfaced only unchanged baseline lints in unrelated tests. No applicable AssuranceProfile was found. No code edits or PR comments were made.

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | FR-191 now references the authoritative FR-018 frontmatter contract and retains only Quoin's exact running-executable version requirement. |

- fnd: FND-001
  outcome: fixed
  after_excerpt: |-
    - The generated files SHALL comply with the `DossierDocument` frontmatter contract defined by spec-artifacts-process [FR-018](ix://agent-ix/spec-artifacts-process/FR-018). Versioned kinds also carry their document `version`, while `records-of-processing` does not. Source-backed RMM and SOUP documents also carry `as_of`.
    - Every generated file SHALL write the exact version string printed by the running Quoin executable to the producer version field defined by spec-artifacts-process FR-018. That producer version is independent of the dossier document's optional `version`.
