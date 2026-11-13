---
type: index
title: "Non-Functional Requirements"
description: "Index of non-functional requirements (NFR) for quoin."
---

# Non-Functional Requirements

## Contents

- [NFR-001: Default-module reconciliation is idempotent and offline-safe](./NFR-001-idempotent-offline-reconcile.md)
- [NFR-002: Catalog assembly is deterministic](./NFR-002-deterministic-catalog.md)
- [NFR-003: Failures surface as actionable errors](./NFR-003-actionable-errors.md)
- [NFR-004: quoin runs as a standalone package](./NFR-004-standalone-dependencies.md)
- [NFR-005: Workflows reference catalog-defined types](./NFR-005-catalog-driven-workflows.md)
- [NFR-006: The agent eval set captures efficiency metrics](./NFR-006-eval-metric-capture.md)
- [NFR-007: ix-flow is invoked by name and its failures surface](./NFR-007-external-tool-invocation.md)
- [NFR-008: Corrupt manifests abort assembly rather than drop silently](./NFR-008-strict-manifest-parsing.md)
- [NFR-009: Skills are authored once and consumed by every supported agent](./NFR-009-single-source-skills-across-agents.md)
- [NFR-010: A default module is the one its declaration names](./NFR-010-module-provenance.md)
- [NFR-011: Bundle-scale commands stay within a stated time budget](./NFR-011-bundle-scale-performance.md)
- [NFR-012: Version skew across the ecosystem is reported, never absorbed](./NFR-012-ecosystem-compatibility.md)
- [NFR-013: Semantic architecture decisions are traceable and standalone-readable](./NFR-013-traceable-semantic-architecture.md)
- [NFR-014: The semantic architecture record is non-disruptive](./NFR-014-non-disruptive-architecture-record.md)
- [NFR-015: The semantic audit is complete and reproducible](./NFR-015-complete-reproducible-semantic-audit.md) — ⛔ **Withdrawn** (#654)
- [NFR-016: The semantic audit is read-only and non-disruptive](./NFR-016-read-only-semantic-audit.md) — ⛔ **Withdrawn** (#654)
- [NFR-017: Semantic manifest evolution is non-disruptive](./NFR-017-non-disruptive-manifest-evolution.md)
- [NFR-021: Reproducible corpus measurement](./NFR-021-reproducible-corpus-measurement.md) — ⛔ **Withdrawn** (#388)
- [NFR-022: Bounded, read-only measurement run](./NFR-022-bounded-read-only-measurement-run.md) — ⛔ **Withdrawn** (#388)
- [NFR-023: Published figures carry their provenance](./NFR-023-figures-carry-their-provenance.md)
- [NFR-025: Evidence and accepted-corpus bytes are immutable across the port](./NFR-025-immutable-evidence-and-corpus-bytes.md)
- [NFR-027: Rust implementation idioms and the gates that enforce them](./NFR-027-rust-implementation-idioms-and-gates.md)
- [NFR-029: A result artifact is interpretable without its source tree](./NFR-029-self-contained-result-evidence.md)
- [NFR-030: SOUP promotion and report never invent a fact or a score](./NFR-030-soup-views-state-and-never-derive.md)
- [NFR-031: Risk advice states what people wrote and never decides](./NFR-031-risk-advice-states-and-never-decides.md)
- [NFR-032: Differential report intake is bounded and deterministic](./NFR-032-differential-report-intake-is-bounded-and-deterministic.md)
- [NFR-033: Counting an analysis result never infers a binding and is order-independent](./NFR-033-analysis-counting-never-infers-a-binding.md)
- [NFR-035: The anomaly report states what people recorded and never decides acceptance](./NFR-035-anomaly-report-states-and-never-decides.md)
- [NFR-034: Class floors and class obligations state what people wrote and never assign a class](./NFR-034-class-reports-state-and-never-assign.md)
- [NFR-036: The release snapshot is deterministic for the same evidence and never decides a release](./NFR-036-release-snapshot-is-deterministic-and-never-decides.md)
- [NFR-037: Source profile evaluation is deterministic, offline, and never decides compliance](./NFR-037-source-profile-evaluation-is-deterministic-and-states-only.md)
- [NFR-038: Facts from mappings state what people wrote and never invent an attestation or a verdict](./NFR-038-mapping-facts-state-and-never-decide.md)
