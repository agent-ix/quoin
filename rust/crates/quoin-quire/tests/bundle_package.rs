// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! The bundle package reaches the engine's document validation (quoin#557).
//!
//! quire-rs FR-076 qualifies `## Relationships` targets under the document's
//! bundle package and never under the module's. A caller that supplies none
//! gets the table reported `unavailable` (`no-bundle-package`).

#![allow(
    clippy::expect_used,
    clippy::indexing_slicing,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]

use ix_trace_rs::trace;
use std::path::PathBuf;

use quoin_quire::{ModuleSelection, ScopeRoot, ids, validate};

fn fixture() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("semantic-bundle")
}

/// Messages of the relationships diagnostics for the fixture document.
fn relationship_messages(bundle_package: Option<&str>) -> Vec<String> {
    let scope = ScopeRoot::open(fixture()).expect("the fixture is checked in");
    let outcome = validate::run(&validate::Request {
        documents: vec![fixture().join("spec").join("FR-006-config-version.md")],
        modules: ModuleSelection::Closed(vec![
            ids::ModuleRoot::open(fixture()).expect("the fixture module is checked in"),
        ]),
        scope,
        bundle_package: bundle_package.map(str::to_string),
    })
    .expect("a validation batch runs");
    assert_eq!(outcome.evaluations.len(), 1);
    let evaluation = &outcome.evaluations[0];
    assert_eq!(
        evaluation.not_run, None,
        "the document must reach the engine: {evaluation:?}"
    );
    evaluation
        .diagnostics
        .iter()
        .map(|d| d.message.clone())
        .filter(|m| m.contains("relationships"))
        .collect()
}

/// With no bundle package the relationships table is unavailable and the engine
/// says so with the `no-bundle-package` advisory.
#[trace("FR-104-AC-12", "FR-099-AC-2")]
#[test]
fn tc_557_no_bundle_package_leaves_relationships_unavailable() {
    let messages = relationship_messages(None);
    assert_eq!(messages.len(), 1, "{messages:?}");
    assert!(
        messages[0].starts_with("semantic.relationships-no-bundle-package"),
        "{messages:?}"
    );
}

/// With the repository's package the row lowers under it: the `no-bundle-package`
/// advisory is gone and the bare `FR-005` target is only unresolved against the
/// absent bundle index.
#[trace("FR-104-AC-12", "FR-099-AC-2")]
#[test]
fn tc_557_a_relationships_row_lowers_under_the_bundle_package() {
    let messages = relationship_messages(Some("agent-ix/config-service"));
    assert!(
        messages
            .iter()
            .all(|m| !m.contains("relationships-no-bundle-package")),
        "{messages:?}"
    );
    assert_eq!(messages.len(), 1, "{messages:?}");
    assert!(
        messages[0].starts_with("semantic.unresolved-target") && messages[0].contains("FR-005"),
        "{messages:?}"
    );
}
