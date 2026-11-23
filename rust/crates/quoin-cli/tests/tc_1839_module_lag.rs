// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! `quoin module doctor` and `quoin catalog validate` warn when an installed
//! default module is older than the CLI's pin (FR-211).

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "integration-test bodies: a panic here is a failing test, which is the intended signal"
)]

use ix_trace_rs::trace;
use std::process::{Command, Output};

const DEFAULT_MODULES: &str = include_str!("../../../../default-modules.yaml");

/// An `IX_HOME` whose registry records every default module as installed at
/// `iso_ref` for `spec-artifacts-iso` and at its pin for the others, with each
/// module directory present so lazy reconcile does no network work.
fn home_with_iso_at(iso_ref: &str) -> tempfile::TempDir {
    let home = tempfile::tempdir().unwrap();
    let manifest = quoin_modules::MarketplaceManifest::from_yaml(DEFAULT_MODULES).unwrap();
    let modules = home.path().join("filament").join("modules");
    let mut plugins = Vec::new();
    for entry in &manifest.entries {
        let dir = modules.join(entry.name.as_str());
        std::fs::create_dir_all(&dir).unwrap();
        let reference = if entry.name.as_str() == "spec-artifacts-iso" {
            iso_ref.to_owned()
        } else {
            entry.source.requested_ref().unwrap().to_owned()
        };
        plugins.push(serde_json::json!({
            "name": entry.name,
            "source": entry.source,
            "ref": reference,
            "resolvedPath": dir,
            "targetPath": dir,
            "installedAt": "2026-01-01T00:00:00Z",
        }));
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
#[trace("FR-211-AC-1", "FR-211-AC-3")]
fn doctor_warns_naming_module_installed_and_pinned_version_and_exits_zero() {
    let home = home_with_iso_at("v0.1.0");
    let output = quoin(&home, &["module", "doctor"]);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(output.status.code(), Some(0), "{stderr}");
    assert!(stderr.contains("spec-artifacts-iso"), "{stderr}");
    assert!(stderr.contains("0.1.0"), "{stderr}");
    assert!(stderr.contains("0.27.0"), "{stderr}");
    assert!(stderr.contains("QM032_MODULE_LAGS_PIN"), "{stderr}");
}

#[test]
#[trace("FR-211-AC-2")]
fn doctor_is_silent_when_every_module_is_at_its_pin() {
    let home = home_with_iso_at("v0.27.0");
    let output = quoin(&home, &["module", "doctor"]);
    assert_eq!(output.status.code(), Some(0));
    assert!(
        output.stderr.is_empty(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
#[trace("FR-211-AC-3")]
fn catalog_validate_warns_without_failing() {
    let home = home_with_iso_at("v0.1.0");
    let output = quoin(&home, &["catalog", "validate"]);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(output.status.code(), Some(0), "{stderr}");
    assert!(stderr.contains("QM032_MODULE_LAGS_PIN"), "{stderr}");
    assert!(stderr.contains("spec-artifacts-iso"), "{stderr}");
}
