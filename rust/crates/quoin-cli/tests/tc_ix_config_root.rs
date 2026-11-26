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

fn invoke(config_root: &Path, legacy_home: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_quoin"))
        .args(args)
        .env("IX_CONFIG_ROOT", config_root)
        .env("IX_HOME", legacy_home)
        .env_remove("QUOIN_SEMANTIC_ROOT")
        .output()
        .expect("the quoin binary starts")
}

#[trace("FR-004-AC-2", "FR-004-AC-4")]
#[test]
fn module_list_and_ensure_defaults_use_ix_config_root_before_ix_home() {
    let scratch = tempfile::tempdir().expect("scratch directory is created");
    let config_root = scratch.path().join("config-root");
    let legacy_home = scratch.path().join("legacy-home");
    install_defaults(&config_root);
    fs::create_dir_all(legacy_home.join("filament")).expect("legacy home exists");

    let listed = invoke(&config_root, &legacy_home, &["module", "list"]);
    assert!(
        listed.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&listed.stderr)
    );
    let modules: serde_json::Value =
        serde_json::from_slice(&listed.stdout).expect("module list emits JSON");
    assert_eq!(
        modules["plugins"].as_array().map(Vec::len),
        Some(10),
        "module list reads the config root selected by IX_CONFIG_ROOT"
    );

    let ensured = invoke(&config_root, &legacy_home, &["module", "ensure-defaults"]);
    assert!(
        ensured.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&ensured.stderr)
    );
}
