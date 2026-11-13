---
id: SR-20261113000905
title: "Scope and boundary review of SOUP promotion, advisory join and report (FR-141..FR-144, NFR-030)"
type: SpecReview
analysis: scope-boundary
review_set: subset
scope: "agent-ix/quoin; spec/functional/FR-141..FR-144, spec/non-functional/NFR-030, spec/functional/index.md, spec/non-functional/index.md, spec/spec.md (PR #837 diff, git diff origin/main...HEAD)"
---
# SR-20261113000905: Scope and boundary review of SOUP promotion, advisory join and report (FR-141..FR-144, NFR-030)

## Summary

Ticket: PLAT-1513. Checks the split of responsibility between quoin and agent-ix/spec-objects-security, and the reuse the ticket asks for.

Clean: quoin owns promotion and the `--discharges` binding (spec-objects-security FR-009-CON-2 is consistent with this). The schema and SoupSupportStatus stay with the owning module (FR-141-CON-2). The join writes nothing into a record (FR-142-CON-1, which agrees with FR-012's refusal of `severity`, `cve_id` and advisory lists). Release gating stays with the consumer (FR-142-CON-2 / FR-034-CON-3). FR-144 is a read-only view like FR-135. No second SBOM parser and no new advisory parser.

## Verdict

PASS with findings. FND-001 is a ticket AC gap. FND-002 is a cross-module contract gap.

## Findings

| ID | Severity | Summary | Refs |
|----|----------|---------|------|
| FND-001 | medium | A PLAT-1513 AC is not met: 'Reuse is stated: evidence adapter registry, retained evidence, `vulnerability` object in spec-objects-security as the candidate join target'. The diff never mentions the `vulnerability` object or the FR-033 adapter registry. spec-objects-security FR-012 added `soup_record` to `vulnerability.affects` and left 'matching advisories to components' to this later capability, yet FR-142 does not say whether an authored `vulnerability` → `affects` → `soup_record` edge is part of the join. Fix: state the boundary. Either the join also attaches vulnerabilities whose `affects` names the record, or the join is scan-only for V1 and the `vulnerability` edge stays authored context. Name the FR-033 registry as the route to the FR-034 adapters. | spec/functional/FR-142-advisory-join-onto-soup-records.md:22-28 |
| FND-002 | medium | `finding_refs` crosses the module boundary with no resolution rule on either side. spec-objects-security FR-009 says it holds SemanticIds naming 'cargo-deny finding entries of quoin FR-034', but an FR-034 `Finding` has no id. Its fields are ruleId, severity, message, path, line and traceIds, and nothing in quoin mints an id for one. So 'attach the findings those ids name' cannot be implemented. A finding attached both by match and by `finding_refs`, or to two records, also breaks FR-142-AC-3's 'attached plus unmatched equals the findings read'. Fix: define the id form quoin resolves, for example `<tool>:<rule-id>@<name>@<version>`, state that a ref matching no retained finding is reported, and state that each finding is counted once. | spec/functional/FR-142-advisory-join-onto-soup-records.md:43 |
