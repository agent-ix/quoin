---
id: SR-20261112191020
title: "Failure-domain analysis of FR-140 pack requirements mapping report in quoin PR 832"
type: SpecReview
analysis: failure-domain
scope: "agent-ix/quoin@f0a0122c; spec/functional/FR-140-pack-requirements-mapping-report.md"
review_set: subset
---

# Failure-domain analysis of FR-140 pack requirements mapping report

## Summary

Ticket: PLAT-1487. Scoped to FR-140 at f0a0122c, focused on the licensing boundary and the
write boundary. Tools: quoin 0.30.1, quire 0.43.0 (engine 0.58.0). The binding report quire-rs
produces (`ClauseBinding`: clause id, force, outcome, reasons, expected outputs) carries no
clause text or title, so the matrices are clean by construction. The AC-5 sentinel search and
the AC-6 snapshot oracle both have control runs and can fail. The remaining exposure is in the
error path and in `--out`.

## Verdict

**CHANGES REQUESTED.** Three edges of the boundary are unspecified or untested.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | `--out` "refused when the path already exists" is a check-then-write: a dangling symlink passes `exists()` and the write follows it; spec must require exclusive create (create_new / O_EXCL) and an AC with a dangling symlink | spec/functional/FR-140-pack-requirements-mapping-report.md:105-107,123 |
| FND-002 | medium | Load errors from the engine can quote field values (serde type errors quote the offending string); AC-8's broken-file fixture need not put a sentinel in the failing field, so AC-5 can pass vacuously on the error path | spec/functional/FR-140-pack-requirements-mapping-report.md:86-89,118,121 |
| FND-003 | medium | "SHALL NOT write into the repository named by --repo" contradicts "the one permitted write is --out" when --out resolves inside that repo; state which wins | spec/functional/FR-140-pack-requirements-mapping-report.md:103-107 |
| FND-004 | low | "any other clause-set field that the engine treats as licensed text" names nothing: the engine marks no fields; enumerate them (clause text, clause title, and decide set title / source title) | spec/functional/FR-140-pack-requirements-mapping-report.md:100-102 |

## Examined scope

```yaml
clean: false
scope:
- {id: FR-140, path: spec/functional/FR-140-pack-requirements-mapping-report.md, role: examined, excerpt: "No writes. ... The one permitted write is the file named by --out, and only when --out is given. --out SHALL be refused when the path already exists, and SHALL be written only after the whole report has been built."}
- {id: FR-140-AC-5, path: spec/functional/FR-140-pack-requirements-mapping-report.md, role: examined, excerpt: "Searching the Markdown, the JSON, stderr and every refusal message of AC-7 to AC-9 for each sentinel finds none; a control run that prints a sentinel is caught by the same search"}
- {id: FR-140-AC-6, path: spec/functional/FR-140-pack-requirements-mapping-report.md, role: examined, excerpt: "a run without --out leaves git status --porcelain unchanged in both, the snapshot identical, and the evidence store and IX_HOME byte-identical. A control run that deliberately writes a file into the repository is detected by the same oracle."}
- {id: FR-140-AC-8, path: spec/functional/FR-140-pack-requirements-mapping-report.md, role: examined, excerpt: "A pack with a clause-set file that fails to load, and a pack whose set fails to evaluate, each exit with status 2"}
- {id: FR-140-AC-10, path: spec/functional/FR-140-pack-requirements-mapping-report.md, role: examined, excerpt: "--out <path> writes exactly the bytes stdout would have carried to that path and leaves stdout empty; a second run with the same path is refused and leaves the file unchanged"}
- {id: FR-139, path: spec/functional/FR-139-pack-writes-nothing-into-a-repository.md, role: context_only, excerpt: "Quoin SHALL NOT write any file, clause text, path, or registry data from a pack into the source tree or into any other directory outside IX_HOME."}
```
