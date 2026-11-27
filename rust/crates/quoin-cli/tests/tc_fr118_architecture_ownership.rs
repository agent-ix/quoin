// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Keep the semantic-module ownership record aligned with FR-118.

use ix_trace_rs::trace;

const OWNERSHIP: &str =
    include_str!("../../../../docs/semantic-module-architecture/ownership-and-boundaries.md");
const DECISIONS: &str =
    include_str!("../../../../docs/semantic-module-architecture/decision-ledger.md");

#[trace("FR-118-AC-1")]
#[test]
fn tc_118_quire_owns_typed_markdown_without_renderer_or_policy_ownership() {
    assert!(OWNERSHIP.lines().any(|line| {
        line.contains("| Quire")
            && line.contains("parse, validate, extract, address, and byte-splice typed Markdown")
            && line.contains(
                "template rendering, cross-language generation, schema-package publication",
            )
            && line.contains("application policy")
    }));
}

#[trace("FR-118-AC-2")]
#[test]
fn tc_118_quoin_owns_catalog_and_orchestration_without_parser_or_runtime_ownership() {
    assert!(OWNERSHIP.contains(
        "| Quoin                | catalog discovery, locks, installation, update; authoring-contract discovery, skills, workflows, and evidence-oriented orchestration | parser semantics, compiler implementation, runtime persistence, consumer adapters                                |"
    ));
}

#[trace("FR-118-AC-3")]
#[test]
fn tc_118_kernel_and_module_repositories_keep_distinct_authority() {
    assert!(OWNERSHIP.contains(
        "| `filament-core-data` | semantic kernel, IR, compatibility rules, compiler, and emitters"
    ));
    assert!(OWNERSHIP.contains(
        "| Module repositories  | vocabulary, constraints, archetypes, skeletons, mappings, examples, and semantic versions"
    ));
}

#[trace("FR-118-AC-4")]
#[test]
fn tc_118_consumers_own_adapters_persistence_and_runtime_presentation() {
    assert!(OWNERSHIP.contains(
        "| Consumers            | application adapters, API and IPC projections, persistence mappings and migrations, runtime state, and UI presentation"
    ));
}

#[trace("FR-118-AC-5")]
#[test]
fn tc_118_architecture_record_preserves_adr_0011_levels_roles_and_consumer_execution() {
    for level in ["L0", "L1", "L2"] {
        assert!(
            OWNERSHIP.contains(&format!("| {level}    |")),
            "validation level {level} is recorded"
        );
    }
    for role in ["Validator", "Advisor", "Generator", "Auditor"] {
        assert!(
            OWNERSHIP.contains(&format!("**{role}**")),
            "role {role} is recorded"
        );
    }
    assert!(OWNERSHIP.contains("Consumer CI always executes L2 work"));
    assert!(DECISIONS.lines().any(|line| {
        line.contains("| `ADR-0011`                     | `agent-ix/quire-rs`")
            && line.contains("| Accepted")
    }));
}
