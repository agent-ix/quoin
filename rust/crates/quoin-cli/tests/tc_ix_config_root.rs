// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! `IX_CONFIG_ROOT` selects the module home for the shipped module commands.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    reason = "integration-test assertions deliberately panic with useful context"
)]

use ix_trace_rs::trace;
use quoin_modules::{InstalledModule, MarketplaceManifest, ModuleRegistry};
use std::fs;
use std::path::Path;
use std::process::{Command, Output};

const DEFAULT_MODULES: &str = include_str!("../../../../default-modules.yaml");

fn install_defaults(home: &Path) {
    let manifest = MarketplaceManifest::from_yaml(DEFAULT_MODULES).expect("default manifest");
    let target_root = home.join("filament/modules");
    let mut plugins = Vec::new();
    for entry in manifest.enabled_entries() {
        let target = target_root.join(entry.name.as_str());
        fs::create_dir_all(&target).expect("module target is materialized");
        fs::write(
            target.join("manifest.yaml"),
            format!("name: {}\nversion: 0.0.0\n", entry.name.as_str()),
        )
        .expect("module fixture manifest is written");
        plugins.push(InstalledModule {
            name: entry.name.clone(),
            source: entry.source.clone(),
            r#ref: None,
            resolved_path: target.display().to_string(),
            target_path: target.display().to_string(),
            installed_at: "2026-01-01T00:00:00Z".to_owned(),
        });
    }
    ModuleRegistry {
        schema_version: 1,
        plugins,
    }
    .write(&home.join("filament/registry.json"))
    .expect("module registry is written");
}

fn invoke(
    config_root: Option<&str>,
    legacy_home: Option<&Path>,
    home: &Path,
    args: &[&str],
) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_quoin"));
    command
        .args(args)
        .env("HOME", home)
        .env_remove("IX_CONFIG_ROOT")
        .env_remove("IX_HOME")
        .env_remove("QUOIN_SEMANTIC_ROOT");
    if let Some(config_root) = config_root {
        command.env("IX_CONFIG_ROOT", config_root);
    }
    if let Some(legacy_home) = legacy_home {
        command.env("IX_HOME", legacy_home);
    }
    command.output().expect("the quoin binary starts")
}

fn assert_module_count(output: &Output, expected: usize, source: &str) {
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let modules: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("module list emits JSON");
    assert_eq!(
        modules["plugins"].as_array().map(Vec::len),
        Some(expected),
        "module list resolves modules from {source}"
    );
}

#[trace("FR-004-AC-2", "FR-004-AC-4")]
#[test]
fn module_list_and_ensure_defaults_use_ix_config_root_before_ix_home() {
    let scratch = tempfile::tempdir().expect("scratch directory is created");
    let process_home = scratch.path().join("process-home");
    let config_root = scratch.path().join("config-root");
    let legacy_home = scratch.path().join("legacy-home");
    fs::create_dir_all(&process_home).expect("process home exists");
    install_defaults(&config_root);
    fs::create_dir_all(legacy_home.join("filament")).expect("legacy home exists");

    let listed = invoke(
        Some(config_root.to_str().expect("config root is UTF-8")),
        Some(&legacy_home),
        &process_home,
        &["module", "list"],
    );
    assert_module_count(&listed, 9, "IX_CONFIG_ROOT ahead of IX_HOME");

    let ensured = invoke(
        Some(config_root.to_str().expect("config root is UTF-8")),
        Some(&legacy_home),
        &process_home,
        &["module", "ensure-defaults"],
    );
    assert!(
        ensured.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&ensured.stderr)
    );
}

#[trace("FR-004-AC-2")]
#[test]
fn empty_or_unset_ix_config_root_falls_back_to_ix_home_then_home_ix() {
    let scratch = tempfile::tempdir().expect("scratch directory is created");
    let process_home = scratch.path().join("process-home");
    let legacy_home = scratch.path().join("legacy-home");
    let default_home = process_home.join(".ix");
    fs::create_dir_all(&process_home).expect("process home exists");
    install_defaults(&legacy_home);

    let empty_config_root = invoke(
        Some(""),
        Some(&legacy_home),
        &process_home,
        &["module", "list"],
    );
    assert_module_count(&empty_config_root, 9, "IX_HOME after empty IX_CONFIG_ROOT");

    let unset_config_root = invoke(None, Some(&legacy_home), &process_home, &["module", "list"]);
    assert_module_count(&unset_config_root, 9, "IX_HOME after unset IX_CONFIG_ROOT");

    install_defaults(&default_home);
    let default = invoke(None, None, &process_home, &["module", "list"]);
    assert_module_count(&default, 9, "~/.ix after both overrides are unset");
}
