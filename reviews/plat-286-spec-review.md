---
id: SR-17075
title: "Spec review of quoin PR 783 (new FR-124)"
type: SpecReview
analysis: base
scope: "agent-ix/quoin@971b6885; spec/functional/FR-124-inspect-installed-semantic-modules.md, spec/functional/index.md, spec/spec.md"
review_set: subset
---

# Spec review of quoin PR 783

## Summary

Ticket: PLAT-286. Reviewed the new FR-124 for consistency, completeness, EARS form,
statement-to-AC agreement, relationships and ceremony. Integrity and EARS checks were run
inline. `quire validate --scope .` over FR-124, the functional index and spec.md reports no
error. The relationship targets US-020, FR-070, FR-075 and FR-049 all resolve. Tools: quoin
0.29.1, quire 0.36.2 (engine 0.50.2).

The Rationale states the real gap clearly: a lock is judged once at install, and nothing
re-judges it after the imported module moves. It also rejects provenance fields (source revision,
fingerprint, compiler version, digest) as ceremony. That is consistent with the owner's #716
removal, and no ceremony is introduced. The index adds the FR-123 line it had been missing, which
is correct catch-up.

## Verdict

**PASS WITH FINDINGS (two low).** FR-124 is sound and its ACs are testable and can fail. The
findings are omissions at the edges.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | FR-124 does not say what `inspect` does when an installed module's manifest is missing, unreadable or not YAML. "One entry per installed module" and "still write the complete document" imply an entry with an error diagnostic, but no clause or AC states it, and the implementation aborts instead (code-review FND-001). Add a Behavior clause and an AC | spec/functional/FR-124-inspect-installed-semantic-modules.md:46-58 |
| FND-002 | low | Dependencies lists FR-019 (manage the plugin registry, which `inspect` reads) as upstream, but the frontmatter `relationships` has no FR-019 edge. The traced graph therefore omits a dependency the text names | spec/functional/FR-124-inspect-installed-semantic-modules.md:5-13,72 |

## Examined scope

```yaml
clean: false
scope:
- {id: FR-124, path: spec/functional/FR-124-inspect-installed-semantic-modules.md, role: examined, excerpt: "`quoin module inspect` SHALL report, for every installed module, the semantic contract it declares and whether its import locks still hold against the installed set, so that dynamic-only and generated-package modules are visible in one catalog and a stale lock is found where it can be fixed."}
- {id: FR-124-AC-1, path: spec/functional/FR-124-inspect-installed-semantic-modules.md, role: examined, excerpt: "With a dynamic-only module and a module declaring a `rust` target installed together, `inspect` reports `dynamic` for the first and `compiled` for the second, each with its package and contract version."}
- {id: FR-124-AC-2, path: spec/functional/FR-124-inspect-installed-semantic-modules.md, role: examined, excerpt: "A clean installed set exits zero and every entry's error diagnostics are empty."}
- {id: FR-124-AC-3, path: spec/functional/FR-124-inspect-installed-semantic-modules.md, role: examined, excerpt: "A module with no `semantic` block is listed as `no-semantic-block` and the command exits zero."}
- {id: FR-124-AC-4, path: spec/functional/FR-124-inspect-installed-semantic-modules.md, role: examined, excerpt: "When an imported module is reinstalled at a version other than the locked one, `inspect` exits 1, writes the whole document, and reports on the importer an error naming the import, the locked version and the installed version."}
- {id: FR-124-AC-5, path: spec/functional/FR-124-inspect-installed-semantic-modules.md, role: examined, excerpt: "A block whose `contract_version` is `2.0.0` is reported with an error diagnostic naming `2.0.0` and no `semantic` view, and the command exits 1."}
- {id: spec/functional/index.md, path: spec/functional/index.md, role: examined, excerpt: "- [FR-124: Inspect installed semantic modules and their import locks](./FR-124-inspect-installed-semantic-modules.md)"}
- {id: FR-075, path: spec/functional/FR-075-semantic-package-exports-and-locks.md, role: context_only, excerpt: "semantic.imports maps an imported package identity to an exact version"}
```
