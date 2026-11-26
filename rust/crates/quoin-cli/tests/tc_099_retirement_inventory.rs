// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! Trace and gate the CLI retirement inventory and workflow asset dispositions.

#![allow(
    clippy::expect_used,
    reason = "test setup failures are the intended failure report"
)]

use ix_trace_rs::trace;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

fn repository_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(3)
        .expect("quoin-cli lives at rust/crates/quoin-cli")
        .to_path_buf()
}

fn files_under(path: &Path, output: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(path) else {
        return;
    };
    for entry in entries {
        let entry = entry.expect("directory entries are readable");
        let path = entry.path();
        if entry.file_type().expect("entry type is readable").is_dir() {
            files_under(&path, output);
        } else {
            output.push(path);
        }
    }
}

fn workflow_asset_dispositions(root: &Path) -> BTreeMap<String, String> {
    let source = std::fs::read_to_string(
        root.join("rust/crates/quoin-config/WORKFLOW_ASSET_DISPOSITIONS.md"),
    )
    .expect("workflow asset disposition inventory is readable");
    source
        .lines()
        .filter_map(|line| {
            let cells: Vec<_> = line
                .trim()
                .trim_matches('|')
                .split('|')
                .map(str::trim)
                .collect();
            (cells.len() == 2 && cells[0].contains('/'))
                .then(|| (cells[0].to_owned(), cells[1].to_owned()))
        })
        .collect()
}

#[trace("FR-099-AC-8")]
#[test]
fn tc_099_package_and_symbol_inventory_are_the_retirement_gate_inputs() {
    let root = repository_root();
    let package: serde_json::Value = serde_json::from_slice(
        &std::fs::read(root.join("package.json")).expect("package.json is readable"),
    )
    .expect("package.json is valid JSON");
    for section in ["dependencies", "devDependencies", "optionalDependencies"] {
        assert_eq!(
            package[section]["@agent-ix/ix-cli-core"],
            serde_json::Value::Null,
            "package.json must not restore the retired dependency"
        );
    }

    let inventory =
        std::fs::read_to_string(root.join("rust/crates/quoin-config/IX_CLI_CORE_RETIREMENT.md"))
            .expect("baseline symbol inventory is readable");
    let gate_inventory = inventory
        .split_once("## Retirement gate inventory")
        .expect("retirement gate inventory section exists")
        .1;
    let mut entries = 0;
    for line in gate_inventory.lines() {
        let cells: Vec<_> = line
            .trim()
            .trim_matches('|')
            .split('|')
            .map(str::trim)
            .collect();
        if cells.len() != 4 || cells[0] == "Symbol" || cells[0].starts_with("---") {
            continue;
        }
        let symbol = cells[0].trim_matches('`');
        let status = cells[1];
        let implementation = cells[2].trim_matches('`');
        let evidence = cells[3].trim_matches('`');
        assert!(!symbol.is_empty(), "inventory symbol names are required");
        match status {
            "ported" => {
                assert!(
                    root.join(implementation).is_file(),
                    "{symbol} names missing Rust implementation {implementation}"
                );
                assert!(
                    root.join(evidence).is_file(),
                    "{symbol} names missing test evidence {evidence}"
                );
            }
            "deferred" | "excluded" => {
                assert!(
                    !implementation.is_empty(),
                    "{symbol} needs a deferral reason"
                );
                assert!(!evidence.is_empty(), "{symbol} needs a test disposition");
            }
            other => panic!("{symbol} has unknown retirement status {other}"),
        }
        entries += 1;
    }
    assert!(entries > 0, "the gate must read named inventory entries");
}

#[trace("FR-099-AC-9")]
#[test]
fn tc_099_workflow_asset_inventory_covers_every_present_asset() {
    let root = repository_root();
    let mut paths = Vec::new();
    files_under(&root.join("skills"), &mut paths);
    let assets: std::collections::BTreeSet<_> = paths
        .into_iter()
        .filter(|path| {
            path.components()
                .any(|part| part.as_os_str() == "workflow-assets")
        })
        .map(|path| {
            path.strip_prefix(&root)
                .expect("skill asset is under repository root")
                .to_string_lossy()
                .replace('\\', "/")
        })
        .collect();
    let dispositions = workflow_asset_dispositions(&root);
    let recorded: std::collections::BTreeSet<_> = dispositions.keys().cloned().collect();
    assert_eq!(
        assets, recorded,
        "every present workflow asset needs one disposition"
    );
    for (path, disposition) in dispositions {
        assert!(
            matches!(
                disposition.as_str(),
                "first-party invariant shim" | "vendored bundle"
            ),
            "{path} has an unknown workflow asset disposition: {disposition}"
        );
    }
}
