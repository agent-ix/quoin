// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! `systems::load` reads the four systems tables of a real repository, and a
//! repository without them yields no records (FR-193, PLAT-1437).

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "integration-test bodies: a panic here is a failing test, which is the intended signal"
)]

use std::path::PathBuf;

use quoin_quire::coverage::Request;
use quoin_quire::ids::{ModuleRoot, ScopeRoot};
use quoin_quire::{ModuleSelection, systems};

fn request(fixture: &PathBuf) -> Request {
    Request {
        scope: ScopeRoot::open(fixture).unwrap(),
        modules: ModuleSelection::Closed(vec![ModuleRoot::open(fixture).unwrap()]),
    }
}

fn fixtures() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
}

#[test]
fn a_repository_that_declares_systems_tables_yields_every_record() {
    let loaded = systems::load(&request(&fixtures().join("systems-repo"))).unwrap();

    assert_eq!(loaded.parts.len(), 1);
    assert_eq!(loaded.parts[0].owner, "PART-A");
    assert_eq!(loaded.parts[0].declared_type, "Engine");
    assert_eq!(loaded.ports.len(), 1);
    assert_eq!(loaded.ports[0].interface, "Telemetry");
    assert_eq!(loaded.ports[0].direction, "out");
    assert_eq!(loaded.connections.len(), 1);
    assert_eq!(loaded.connections[0].source, "PART-A.out");
    assert_eq!(loaded.connections[0].target, "PART-B.in");
    assert_eq!(loaded.connections[0].target_multiplicity, "0..*");
    assert_eq!(loaded.connections[0].direction, "forward");
    assert_eq!(loaded.allocations.len(), 1);
    assert_eq!(loaded.allocations[0].source, "FR-001");
    assert_eq!(loaded.allocations[0].target, "PART-A");
    assert_eq!(loaded.allocations[0].document, "FR-001");
}

#[test]
fn a_repository_without_systems_tables_yields_no_records() {
    let loaded = systems::load(&request(&fixtures().join("repo"))).unwrap();
    assert_eq!(loaded, systems::Systems::default());
}
