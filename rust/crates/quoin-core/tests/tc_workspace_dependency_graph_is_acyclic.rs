// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The workspace's crates form a dependency DAG.
//!
//! Cargo refuses to resolve a cycle among normal or build dependencies, so a
//! successful `cargo metadata` is already the proof; this test additionally
//! walks the resolved graph itself, so a cycle is reported with its members
//! instead of as a resolver message.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::process::Command;

use ix_trace_rs::trace;
use serde_json::Value;

/// The resolved graph among workspace members, normal and build edges only
/// (dev-dependency edges may legitimately loop back).
fn workspace_edges() -> BTreeMap<String, BTreeSet<String>> {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../Cargo.toml");
    let output = Command::new(env!("CARGO"))
        .args(["metadata", "--format-version", "1", "--offline", "--locked"])
        .arg("--manifest-path")
        .arg(&manifest)
        .output()
        .expect("cargo runs");
    assert!(
        output.status.success(),
        "cargo metadata failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let metadata: Value = serde_json::from_slice(&output.stdout).expect("metadata is JSON");
    let members: BTreeSet<&str> = metadata["workspace_members"]
        .as_array()
        .unwrap()
        .iter()
        .map(|id| id.as_str().unwrap())
        .collect();
    // Package ids embed the checkout path; name the crates instead.
    let names: BTreeMap<&str, &str> = metadata["packages"]
        .as_array()
        .unwrap()
        .iter()
        .map(|package| {
            (
                package["id"].as_str().unwrap(),
                package["name"].as_str().unwrap(),
            )
        })
        .collect();
    let mut edges = BTreeMap::new();
    for node in metadata["resolve"]["nodes"].as_array().unwrap() {
        let id = node["id"].as_str().unwrap();
        if !members.contains(id) {
            continue;
        }
        let targets = node["deps"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|dep| {
                dep["dep_kinds"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|kind| kind["kind"].is_null() || kind["kind"] == "build")
            })
            .map(|dep| dep["pkg"].as_str().unwrap())
            .filter(|pkg| members.contains(pkg))
            .map(|pkg| names[pkg].to_owned())
            .collect();
        edges.insert(names[id].to_owned(), targets);
    }
    edges
}

/// Depth-first walk that reports the first back edge as a path.
fn find_cycle(edges: &BTreeMap<String, BTreeSet<String>>) -> Option<Vec<String>> {
    fn visit(
        node: &str,
        edges: &BTreeMap<String, BTreeSet<String>>,
        done: &mut BTreeSet<String>,
        path: &mut Vec<String>,
    ) -> Option<Vec<String>> {
        if let Some(at) = path.iter().position(|seen| seen == node) {
            let mut cycle = path[at..].to_vec();
            cycle.push(node.to_owned());
            return Some(cycle);
        }
        if done.contains(node) {
            return None;
        }
        path.push(node.to_owned());
        for next in edges.get(node).into_iter().flatten() {
            if let Some(cycle) = visit(next, edges, done, path) {
                return Some(cycle);
            }
        }
        path.pop();
        done.insert(node.to_owned());
        None
    }
    let mut done = BTreeSet::new();
    for node in edges.keys() {
        if let Some(cycle) = visit(node, edges, &mut done, &mut Vec::new()) {
            return Some(cycle);
        }
    }
    None
}

#[trace("FR-100-AC-1")]
#[test]
fn the_workspace_dependency_graph_has_no_cycle() {
    let edges = workspace_edges();
    assert!(edges.len() > 1, "the walk must reach more than one crate");
    assert_eq!(find_cycle(&edges), None);
}

/// The detector itself must be able to fail: a two-node loop is found.
#[trace("FR-100-AC-1")]
#[test]
fn the_cycle_detector_finds_a_loop_when_there_is_one() {
    let mut edges: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    edges.insert("a".into(), BTreeSet::from(["b".to_owned()]));
    edges.insert("b".into(), BTreeSet::from(["a".to_owned()]));
    assert_eq!(
        find_cycle(&edges),
        Some(vec!["a".into(), "b".into(), "a".into()])
    );
}

/// Crates with a normal or build edge onto `quoin-cli`. The CLI is the
/// delivery surface; the engine crates must load without it.
fn cli_dependents(edges: &BTreeMap<String, BTreeSet<String>>) -> Vec<String> {
    let is_cli = |name: &str| name == "quoin-cli";
    edges
        .iter()
        .filter(|(id, targets)| !is_cli(id) && targets.iter().any(|target| is_cli(target)))
        .map(|(id, _)| id.clone())
        .collect()
}

/// FR-036-AC-7: the CLI is a leaf.
#[trace("FR-036-AC-7")]
#[test]
fn no_workspace_crate_depends_on_the_cli() {
    let edges = workspace_edges();
    assert!(
        edges.contains_key("quoin-cli"),
        "the walk must reach the CLI crate"
    );
    assert_eq!(cli_dependents(&edges), Vec::<String>::new());
}

/// The check must be able to fail: an edge onto the CLI is reported, and a
/// crate whose name merely contains `quoin-cli` is not the CLI.
#[trace("FR-036-AC-7")]
#[test]
fn the_cli_leaf_check_reports_an_injected_edge() {
    let mut edges: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    edges.insert(
        "quoin-core".into(),
        BTreeSet::from(["quoin-cli".to_owned()]),
    );
    edges.insert("quoin-cli".into(), BTreeSet::new());
    assert_eq!(cli_dependents(&edges), vec!["quoin-core".to_owned()]);

    let mut edges: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    edges.insert(
        "quoin-cli-extras".into(),
        BTreeSet::from(["quoin-core".to_owned()]),
    );
    edges.insert("quoin-core".into(), BTreeSet::new());
    assert_eq!(cli_dependents(&edges), Vec::<String>::new());
}
