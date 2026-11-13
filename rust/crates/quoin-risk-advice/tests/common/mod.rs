// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

#![allow(
    dead_code,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "shared test helpers: each test crate uses a subset, and a panic IS the failure report"
)]

//! Fixture helpers shared by the risk report tests. Fixtures are written into a
//! throwaway directory; nothing is copied from `spec-objects-safety`, whose
//! record types are read in place from `SPEC_OBJECTS_SAFETY_DIR` or the
//! checkout next to this repository.

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use quoin_quire::{ModuleRoot, ModuleSelection};
use serde_json::Value;

pub(crate) fn modules() -> ModuleSelection {
    let checkout = std::env::var_os("SPEC_OBJECTS_SAFETY_DIR").map_or_else(
        || Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../../spec-objects-safety"),
        PathBuf::from,
    );
    let root = checkout.join("spec_objects_safety");
    assert!(
        root.join("manifest.yaml").is_file(),
        "the spec-objects-safety module is not at {}. These tests read its record types in \
         place and keep no copy: check out agent-ix/spec-objects-safety (at or after \
         PLAT-1565) next to this repository, or set SPEC_OBJECTS_SAFETY_DIR to its checkout",
        root.display()
    );
    ModuleSelection::Closed(vec![ModuleRoot::open(&root).unwrap()])
}

pub(crate) struct Corpus {
    pub(crate) dir: tempfile::TempDir,
}

impl Corpus {
    pub(crate) fn put(&self, id: &str, kind: &str, front: &str, body: &str) {
        // The safety types require the `## Properties` identity table every
        // authored record carries.
        let properties = if matches!(
            kind,
            "risk_estimate"
                | "acceptability_policy"
                | "residual_risk"
                | "benefit_risk"
                | "overall_residual_risk_evaluation"
        ) {
            "\n## Properties\n\n| Field | Type | Multiplicity | Constraints |\n|---|---|---|---|\n| record_id | UUID | 1..1 | identity |\n"
        } else {
            ""
        };
        let text = format!(
            "---\nid: {id}\ntitle: \"{id}\"\ntype: {kind}\nobject: {kind}\n{front}---\n# [{id}] {id}\n\n{body}{properties}"
        );
        std::fs::write(self.dir.path().join(format!("{id}.md")), text).unwrap();
    }
}

pub(crate) fn rels(edges: &[(&str, &str)]) -> String {
    let mut out = String::from("relationships:\n");
    for (target, verb) in edges {
        let _ = writeln!(out, "  - target: {target}\n    type: {verb}");
    }
    out
}

/// A two-by-two policy. `high` is the flag of the region `Serious`/`Likely`
/// lands in; `regions` is the order the region list is written in.
pub(crate) fn policy_with(c: &Corpus, id: &str, high: bool, regions: &[(&str, bool)]) {
    let mut table = String::new();
    for (name, flag) in regions {
        let flag = if *name == "High" { high } else { *flag };
        let _ = writeln!(table, "| {name} | {flag} |");
    }
    c.put(
        id,
        "acceptability_policy",
        "",
        &format!(
            "## Severity levels\n\n| Severity level |\n| --- |\n| Minor |\n| Serious |\n\n\
             ## Probability levels\n\n| Probability level |\n| --- |\n| Rare |\n| Likely |\n\n\
             ## Regions\n\n| Region | Outside acceptable |\n| --- | --- |\n{table}\n\
             ## Cells\n\n| Severity | Probability | Region |\n| --- | --- | --- |\n\
             | Minor | Rare | Low |\n| Minor | Likely | Low |\n| Serious | Rare | Mid |\n| Serious | Likely | High |\n"
        ),
    );
}

pub(crate) const REGIONS: [(&str, bool); 3] = [("Low", false), ("Mid", false), ("High", true)];

pub(crate) fn policy(c: &Corpus, id: &str) {
    policy_with(c, id, true, &REGIONS);
}

pub(crate) fn estimate(
    c: &Corpus,
    id: &str,
    phase: &str,
    levels: (&str, &str),
    edges: &[(&str, &str)],
) {
    c.put(
        id,
        "risk_estimate",
        &format!("phase: {phase}\n{}", rels(edges)),
        &format!(
            "## Judgement\n\n| Severity | Probability | Rationale |\n| --- | --- | --- |\n| {} | {} | why |\n\n\
             ## Provenance\n\n| Asserted by | Asserted at |\n| --- | --- |\n| Pat | 2026-01-01 |\n",
            levels.0, levels.1
        ),
    );
}

pub(crate) fn residual(c: &Corpus, id: &str, est: &str, pol: &str, decision: &str) {
    let by = if decision == "undecided" { "" } else { "Pat" };
    c.put(
        id,
        "residual_risk",
        &rels(&[(est, "estimate"), (pol, "references")]),
        &format!(
            "## Determination\n\n| Decision | Rationale | Decided by |\n| --- | --- | --- |\n| {decision} | r | {by} |\n"
        ),
    );
}

pub(crate) fn snapshot(dir: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
    let mut out = BTreeMap::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(path) = stack.pop() {
        for entry in std::fs::read_dir(&path).unwrap().flatten() {
            if entry.file_type().unwrap().is_dir() {
                stack.push(entry.path());
            } else {
                out.insert(entry.path(), std::fs::read(entry.path()).unwrap());
            }
        }
    }
    out
}

pub(crate) fn keys(value: &Value) -> Vec<&str> {
    let mut keys: Vec<&str> = value
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect();
    keys.sort_unstable();
    keys
}

pub(crate) fn corpus() -> Corpus {
    Corpus {
        dir: tempfile::tempdir().unwrap(),
    }
}
