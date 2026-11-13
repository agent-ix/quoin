---
id: SR-20261113000903
title: "Dependency review of SOUP promotion, advisory join and report (FR-141..FR-144, NFR-030)"
type: SpecReview
analysis: dependency
review_set: subset
scope: "agent-ix/quoin; spec/functional/FR-141..FR-144, spec/non-functional/NFR-030, spec/functional/index.md, spec/non-functional/index.md, spec/spec.md (PR #837 diff, git diff origin/main...HEAD)"
---
# SR-20261113000903: Dependency review of SOUP promotion, advisory join and report (FR-141..FR-144, NFR-030)

## Summary

Ticket: PLAT-1513. Checks the new `relationships:` edges and Dependencies sections, and separates enablement work from feature work.

Every `ix://` target resolves: quoin StR-009, FR-034, FR-041, FR-046, FR-133, FR-135 and FR-141..FR-144, and spec-objects-security FR-009..FR-012 on origin/main. The edge directions and the FR-141 → FR-142 → FR-144 and FR-143 → FR-144 order have no cycle. The missing clause-binding input of FR-144 is recorded as base FND-001.

## Verdict

PASS with findings. FND-001 is medium: an enablement change to FR-041 that has to be stated before implementation.

## Findings

| ID | Severity | Summary | Refs |
|----|----------|---------|------|
| FND-001 | medium | FR-141 needs an FR-041 change that the PR does not state. The FR-041 adapter (`quoin-evidence` `adapters/sbom.rs`) emits one `RunEntry` per component, whose `symbol` is the purl or else `name@version`. It exposes no `supplier`, no separate name or version for a component with a purl, and no flag saying whether the symbol is a purl. FR-141-CON-1 forbids promote from reading the SBOM itself, so copying `name`, `version` and `supplier` (line 38) needs FR-041 to be extended to expose them. That is an enablement dependency. Fix: add an FR-041 amendment, or a precursor requirement, that exposes purl, name, version and supplier per component, including the supplier mapping (CycloneDX `supplier.name` object, SPDX `supplier` 'Organization: X' string). Or drop `supplier` from V1 promotion. | spec/functional/FR-141-promote-sbom-components-to-soup-records.md:26 |
| FND-002 | low | The Dependencies section and the frontmatter disagree. FR-143 lists FR-046 as Upstream and relies on its `--as-of`, but its `relationships:` has no FR-046 edge. FR-144 relates to FR-135 as `references`, but lists it under Upstream. Fix: add `depends_on` FR-046 to FR-143, and in FR-144 either move FR-135 to context or make it `depends_on`. | spec/functional/FR-143-support-status-and-end-of-support-input.md:1-13 |
