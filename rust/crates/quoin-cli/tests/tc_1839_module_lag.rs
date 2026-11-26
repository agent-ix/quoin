// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! `quoin module doctor` and `quoin catalog validate` warn when an installed
//! default module is not at the CLI's pin (FR-212).

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "integration-test bodies: a panic here is a failing test, which is the intended signal"
)]

use ix_trace_rs::trace;
use std::process::{Command, Output};

const DEFAULT_MODULES: &str = include_str!("../../../../default-modules.yaml");
const LAGGING_MODULE: &str = "spec-artifacts-iso";

/// The ref the manifest pins `LAGGING_MODULE` to.
fn pinned_ref() -> String {
    let manifest = quoin_modules::MarketplaceManifest::from_yaml(DEFAULT_MODULES).unwrap();
    manifest
        .entries
        .iter()
        .find(|entry| entry.name.as_str() == LAGGING_MODULE)
        .and_then(|entry| entry.source.requested_ref())
        .unwrap()
        .to_owned()
}

/// An `IX_HOME` whose registry records every default module as installed at
/// its pin, except `LAGGING_MODULE`, installed at `lagging_ref` (or with no
/// ref). Each module directory is materialized so reconcile does no network
/// work while the test checks lag reporting.
fn home_with_lagging_module_at(lagging_ref: Option<&str>) -> tempfile::TempDir {
    let home = tempfile::tempdir().unwrap();
    let manifest = quoin_modules::MarketplaceManifest::from_yaml(DEFAULT_MODULES).unwrap();
    let modules = home.path().join("filament").join("modules");
    let mut plugins = Vec::new();
    for entry in &manifest.entries {
        let dir = modules.join(entry.name.as_str());
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("manifest.yaml"),
            format!(
                "name: {}\nversion: 1.0.0\nartifact_types:\n  - name: FR\n",
                entry.name
            ),
        )
        .unwrap();
        if entry.source.git_url().is_some() {
            std::fs::write(dir.join(".quoin-materialize-complete"), "").unwrap();
        }
        let mut record = serde_json::Map::from_iter([
            ("name".to_owned(), serde_json::json!(entry.name)),
            ("source".to_owned(), serde_json::json!(entry.source)),
            ("resolvedPath".to_owned(), serde_json::json!(dir)),
            ("targetPath".to_owned(), serde_json::json!(dir)),
            (
                "installedAt".to_owned(),
                serde_json::json!("2026-01-01T00:00:00Z"),
            ),
        ]);
        let reference = if entry.name.as_str() == LAGGING_MODULE {
            lagging_ref.map(str::to_owned)
        } else {
            entry.source.requested_ref().map(str::to_owned)
        };
        if let Some(reference) = reference {
            record.insert("ref".to_owned(), reference.into());
        }
        plugins.push(record);
    }
    let registry = serde_json::json!({ "schemaVersion": 1, "plugins": plugins });
    std::fs::write(
        home.path().join("filament").join("registry.json"),
        registry.to_string(),
    )
    .unwrap();
    home
}

fn quoin(home: &tempfile::TempDir, arguments: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_quoin"))
        .args(arguments)
        .env("IX_HOME", home.path())
        .output()
        .expect("the native quoin binary runs")
}

#[test]
#[trace("FR-212-AC-1", "FR-212-AC-5")]
fn doctor_warns_for_an_older_module_summarises_and_exits_zero() {
    let home = home_with_lagging_module_at(Some("v0.0.1"));
    let output = quoin(&home, &["module", "doctor"]);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(output.status.code(), Some(0), "{stderr}");
    assert!(stdout.contains("1 installed module(s) not at"), "{stdout}");
    assert!(stderr.contains(LAGGING_MODULE), "{stderr}");
    assert!(stderr.contains("v0.0.1"), "{stderr}");
    assert!(stderr.contains(&pinned_ref()), "{stderr}");
    assert!(stderr.contains("QM032_MODULE_LAGS_PIN"), "{stderr}");
    assert!(
        !stderr.contains(&format!("quoin module remove {LAGGING_MODULE}")),
        "the repair should not require removing a module first: {stderr}"
    );
    assert!(stderr.contains("quoin module ensure-defaults"), "{stderr}");
}

#[test]
#[trace("FR-212-AC-3")]
fn doctor_warns_when_the_installed_ref_is_a_branch() {
    let home = home_with_lagging_module_at(Some("main"));
    let output = quoin(&home, &["module", "doctor"]);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(output.status.code(), Some(0), "{stderr}");
    assert!(stderr.contains("not at the pinned release"), "{stderr}");
    assert!(stderr.contains(LAGGING_MODULE), "{stderr}");
    assert!(stderr.contains("main"), "{stderr}");
    assert!(stderr.contains(&pinned_ref()), "{stderr}");
}

#[test]
#[trace("FR-212-AC-3")]
fn doctor_warns_when_the_installed_module_records_no_ref() {
    let home = home_with_lagging_module_at(None);
    let output = quoin(&home, &["module", "doctor"]);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(output.status.code(), Some(0), "{stderr}");
    assert!(stderr.contains("no recorded ref"), "{stderr}");
    assert!(stderr.contains(LAGGING_MODULE), "{stderr}");
}

#[test]
#[trace("FR-212-AC-2")]
fn doctor_is_silent_when_every_module_is_at_its_pin() {
    let home = home_with_lagging_module_at(Some(&pinned_ref()));
    let output = quoin(&home, &["module", "doctor"]);
    assert_eq!(output.status.code(), Some(0));
    assert!(
        output.stderr.is_empty(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
#[trace("FR-212-AC-6")]
fn catalog_validate_warns_and_still_exits_zero() {
    let home = home_with_lagging_module_at(Some("v0.0.1"));
    let output = quoin(&home, &["catalog", "validate"]);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(output.status.code(), Some(0), "{stderr}");
    assert!(stderr.contains("QM032_MODULE_LAGS_PIN"), "{stderr}");
    assert!(stderr.contains(LAGGING_MODULE), "{stderr}");
}
