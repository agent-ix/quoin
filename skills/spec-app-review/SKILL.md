---
name: spec-app-review
description: Review an application-type spec for completeness against UI rendering requirements and golden-path structure.
---

# Review Application Specification

If this plugin is not initialized or an Agent IX command fails, read [the quoin setup guide](https://github.com/agent-ix/quoin/blob/main/setup.md) for its prerequisites and local diagnosis.

Validate a `component_type: application` spec against what the spec-editor-ui **actually renders** and the structural requirements from `specify`.

## When to Use

- After creating or updating an application spec
- Before submitting a spec for formal review
- When debugging why sections appear empty in the spec-editor-ui Application Detail view

## Process

1. **Locate spec**: Read `spec/spec.md` in the target repo. Verify `component_type: application` in frontmatter.
2. **Run checklist**: Evaluate every item in [references/checklist.md](references/checklist.md).
3. **Report findings**: Group issues by severity (FAIL = will break UI, WARN = degraded display, INFO = quality improvement).
4. **Fix or flag**: If instructed to fix, apply corrections. Otherwise, output the report.

## Quick Reference

The spec-editor-ui `ApplicationDetailPage` has 7 tabs. Each tab extracts data from specific sections/artifacts:

| Tab | Data Source |
|-----|------------|
| Overview | Purpose, In Scope, Out of Scope, Operational Boundaries, API Endpoints, Components (catalog), Standards (frontmatter) |
| Architecture | Mermaid diagrams (tagged `logical`/`deployment`), FR diagrams, Aggregated Objects (catalog) |
| Integration | Artifacts with `tags: [integration]` in frontmatter |
| Security | Trust Boundaries table, artifacts with `tags: [security]`, Standards |
| Requirements | StR/US/FR/NFR tables parsed from spec.md |
| Decisions | ADR artifacts (`type: ADR`) or inline ADR sections |
| Spec | Raw markdown rendering |

## Key Parsing Rules (Quire Library)

These are the **exact** parser behaviors that determine whether content renders:

- **Out of Scope**: `parseDelegations()` only recognizes `(Handled by X)` or `(Delegated to X)` patterns. Other parentheticals won't extract delegation targets.
- **In Scope**: `parseBulletList()` default pattern is `bold-description` (`**Title** — Description`). Plain bullets fall back to title-only.
- **Operational Boundaries**: `parseBulletList({ pattern: 'bold-colon' })` requires `**Name**: detail` format.
- **Trust Boundaries**: `parseTable()` on section "Trust Boundaries" — expects 3 columns: Role, Capabilities, Restrictions.
- **Mermaid tags**: `findDiagramByTag()` matches `%% @type: logical` and `%% @type: deployment` comments in mermaid blocks.
- **Integration tab**: Filters artifacts by `tags: [integration]` in YAML frontmatter (case-insensitive).
- **Security tab**: Filters artifacts by `tags: [security]` in YAML frontmatter.

## Golden Reference

Use a known well-formed application spec in your org as a comparison reference when reviewing.
