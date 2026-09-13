// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Parity with the TypeScript oracle (quoin#381, FR-101).
//!
//! Every expectation here is read from `rust/goldens/ts-oracle.json`, captured
//! once from the TypeScript implementation and committed. See
//! `rust/goldens/PROVENANCE.md` for the revision and the capture command. No
//! test in this file shells out to Node: TypeScript is the oracle exactly once,
//! at capture time.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]

use std::fs;
use std::path::{Path, PathBuf};

use quoin_config::paths::FixedEnvironment;
use quoin_config::schema::PluginConfigSchema as _;
use quoin_config::{
    IncidentLog, OrgOptions, OrgSource, QuoinConfig, RuntimeContext, origin_org, resolve_org,
};
use serde_json::Value as Json;

const GOLDENS: &str = include_str!("../../../goldens/ts-oracle.json");

fn goldens() -> Json {
    serde_json::from_str(GOLDENS).expect("committed goldens parse")
}

fn section(name: &str) -> Json {
    goldens()
        .get(name)
        .unwrap_or_else(|| panic!("goldens carry a `{name}` section"))
        .clone()
}

fn as_array(value: &Json, name: &str) -> Vec<Json> {
    value
        .as_array()
        .unwrap_or_else(|| panic!("`{name}` is an array"))
        .clone()
}

fn git_config(url: &str) -> String {
    format!(
        "[core]\n\trepositoryformatversion = 0\n[remote \"origin\"]\n\turl = {url}\n\tfetch = +refs/heads/*:refs/remotes/origin/*\n"
    )
}

/// Trace: FR-025-AC-2, FR-025-AC-3, FR-025-AC-9, FR-025-AC-10
#[test]
fn tc_381_100_origin_org_matches_the_typescript_oracle_for_every_url_form() {
    let cases = as_array(&section("origin_org_urls"), "origin_org_urls");
    assert!(cases.len() >= 20, "golden set shrank: {}", cases.len());
    for case in cases {
        let url = case["url"].as_str().expect("url is a string");
        let expected = case["org"].as_str();
        let actual = origin_org(&git_config(url));
        assert_eq!(
            actual.as_ref().map(quoin_config::OrgName::as_str),
            expected,
            "url {url}"
        );
    }
}

/// Trace: FR-025-AC-11, FR-025-AC-4
#[test]
fn tc_381_101_origin_org_matches_the_oracle_for_every_config_shape() {
    let cases = as_array(&section("origin_org_configs"), "origin_org_configs");
    assert!(cases.len() >= 8, "golden set shrank: {}", cases.len());
    for case in cases {
        let label = case["label"].as_str().expect("label");
        let config = case["config"].as_str().expect("config");
        let expected = case["org"].as_str();
        assert_eq!(
            origin_org(config)
                .as_ref()
                .map(quoin_config::OrgName::as_str),
            expected,
            "case {label}"
        );
    }
}

/// Trace: FR-027-AC-6
#[test]
fn tc_381_102_config_schema_accepts_and_rejects_exactly_what_zod_did() {
    let cases = as_array(&section("config_schema"), "config_schema");
    assert!(cases.len() >= 7, "golden set shrank: {}", cases.len());
    for case in cases {
        let input = &case["input"];
        let expected_valid = case["valid"].as_bool().expect("valid flag");
        let yaml: serde_yaml_ng::Value =
            serde_yaml_ng::to_value(input).expect("json input converts to yaml");
        let actual = QuoinConfig::validate(&yaml);
        assert_eq!(
            actual.is_ok(),
            expected_valid,
            "input {input} — got {actual:?}"
        );
        if expected_valid {
            let expected_org = case["value"].get("org").and_then(Json::as_str);
            let got = actual.expect("valid");
            assert_eq!(
                got.org.as_ref().map(quoin_config::OrgName::as_str),
                expected_org,
                "input {input}"
            );
        }
    }
}

/// A hermetic environment and runtime context built from one golden's `given`.
struct Fixture {
    _root: tempfile::TempDir,
    repo_root: PathBuf,
    env: FixedEnvironment,
    ctx: RuntimeContext,
}

fn build_fixture(given: &Json) -> Fixture {
    let root = tempfile::tempdir().expect("temp dir");
    let base = root.path();

    let config_home = base.join("xdg");
    let mut env =
        FixedEnvironment::new().with_var("XDG_CONFIG_HOME", config_home.to_string_lossy());
    if let Some(user) = given["user"].as_str() {
        let dir = config_home.join("ix").join("config.d");
        fs::create_dir_all(&dir).expect("user config dir");
        fs::write(dir.join("quoin.yaml"), user).expect("user config");
    }
    if let Some(value) = given["env"].as_str() {
        env = env.with_var("QUOIN_ORG", value);
    }

    let mut ctx = RuntimeContext::new();
    if let Some(project) = given["project"].as_str() {
        let project_root = base.join("proj").join(".ix");
        fs::create_dir_all(project_root.join("config.d")).expect("project config dir");
        fs::write(project_root.join("config.d").join("quoin.yaml"), project)
            .expect("project config");
        ctx = ctx.with_project_config_root(project_root);
    }
    if let Some(enabled) = given["projectEnabled"].as_bool() {
        ctx = ctx.with_project_config_enabled(enabled);
    }

    let repo_root = base.join("repo");
    fs::create_dir_all(&repo_root).expect("repo root");
    if let Some(remote) = given["remote"].as_str() {
        let git = repo_root.join(".git");
        fs::create_dir_all(&git).expect("git dir");
        fs::write(git.join("config"), git_config(remote)).expect("git config");
    }

    Fixture {
        _root: root,
        repo_root,
        env,
        ctx,
    }
}

/// Trace: FR-027-AC-1, FR-027-AC-3, FR-027-AC-4, FR-027-AC-5, FR-027-AC-9
#[test]
fn tc_381_103_org_resolution_layering_matches_the_typescript_oracle() {
    let cases = as_array(&section("config_service_org"), "config_service_org");
    assert!(cases.len() >= 10, "golden set shrank: {}", cases.len());
    for case in cases {
        let label = case["label"].as_str().expect("label");
        let fixture = build_fixture(&case["given"]);
        let mut log = IncidentLog::new();
        let resolved = resolve_org(
            &fixture.repo_root,
            &OrgOptions::default(),
            &fixture.env,
            &fixture.ctx,
            &mut log,
        );
        let expected = &case["resolved"];
        assert_eq!(
            resolved.source.as_str(),
            expected["source"].as_str().expect("source"),
            "case {label}: source"
        );
        assert_eq!(
            resolved.org.as_ref().map(quoin_config::OrgName::as_str),
            expected.get("org").and_then(Json::as_str),
            "case {label}: org"
        );
    }
}

/// Trace: FR-023-AC-4, FR-025-AC-1
#[test]
fn tc_381_104_flag_and_env_precedence_matches_the_oracle() {
    let cases = as_array(&section("resolve_org_flag_env"), "resolve_org_flag_env");
    assert!(cases.len() >= 5, "golden set shrank: {}", cases.len());
    let root = tempfile::tempdir().expect("temp dir");
    let repo_root = root.path().join("repo");
    fs::create_dir_all(repo_root.join(".git")).expect("git dir");
    fs::write(
        repo_root.join(".git").join("config"),
        git_config("git@github.com:from-git/repo.git"),
    )
    .expect("git config");

    for case in cases {
        let opts = &case["opts"];
        let flag = opts.get("flag").and_then(Json::as_str);
        // The oracle's `env` option is an explicit environment map; an absent
        // `QUOIN_ORG` in it means the variable is unset for that case.
        let mut env = FixedEnvironment::new().with_var(
            "XDG_CONFIG_HOME",
            root.path().join("empty-xdg").to_string_lossy(),
        );
        if let Some(value) = opts.get("env").and_then(|e| e.get("QUOIN_ORG")) {
            env = env.with_var("QUOIN_ORG", value.as_str().expect("QUOIN_ORG is a string"));
        }
        let mut log = IncidentLog::new();
        let resolved = resolve_org(
            &repo_root,
            &OrgOptions { flag },
            &env,
            &RuntimeContext::new(),
            &mut log,
        );
        let expected = &case["resolved"];
        assert_eq!(
            resolved.source.as_str(),
            expected["source"].as_str().expect("source"),
            "opts {opts}"
        );
        assert_eq!(
            resolved.org.as_ref().map(quoin_config::OrgName::as_str),
            expected.get("org").and_then(Json::as_str),
            "opts {opts}"
        );
    }
}

/// Trace: FR-025-AC-8
#[test]
fn tc_381_105_worktree_gitdir_resolves_the_main_checkouts_org() {
    // Mirror git's layout: the worktree's `.git` is a file naming a gitdir
    // under the main checkout, whose `commondir` points back at the shared
    // `.git` that actually holds the config.
    let root = tempfile::tempdir().expect("temp dir");
    let main = root.path().join("main");
    let common_git = main.join(".git");
    let wt_git_dir = common_git.join("worktrees").join("feature");
    fs::create_dir_all(&wt_git_dir).expect("worktree gitdir");
    fs::write(
        common_git.join("config"),
        git_config("git@github.com:acme/widgets.git"),
    )
    .expect("git config");
    fs::write(wt_git_dir.join("commondir"), "../..\n").expect("commondir");

    let worktree = root.path().join("wt");
    fs::create_dir_all(&worktree).expect("worktree");
    fs::write(
        worktree.join(".git"),
        format!("gitdir: {}\n", wt_git_dir.display()),
    )
    .expect("gitdir pointer");

    let mut log = IncidentLog::new();
    let env = FixedEnvironment::new()
        .with_var("XDG_CONFIG_HOME", root.path().join("xdg").to_string_lossy());
    let resolved = resolve_org(
        &worktree,
        &OrgOptions::default(),
        &env,
        &RuntimeContext::new(),
        &mut log,
    );
    assert_eq!(resolved.source, OrgSource::Git);
    assert_eq!(
        resolved.org.as_ref().map(quoin_config::OrgName::as_str),
        Some("acme")
    );
}

/// Trace: FR-025-AC-8
#[test]
fn tc_381_106_broken_gitdir_pointers_resolve_to_none() {
    let root = tempfile::tempdir().expect("temp dir");
    let env = FixedEnvironment::new()
        .with_var("XDG_CONFIG_HOME", root.path().join("xdg").to_string_lossy());
    for (label, body) in [
        ("points nowhere useful", "gitdir: /nonexistent/path\n"),
        ("no gitdir pointer", "not a gitdir pointer\n"),
        ("empty pointer", "gitdir:   \n"),
    ] {
        let repo = root.path().join(label.replace(' ', "-"));
        fs::create_dir_all(&repo).expect("repo dir");
        fs::write(repo.join(".git"), body).expect("dot git file");
        let mut log = IncidentLog::new();
        let resolved = resolve_org(
            &repo,
            &OrgOptions::default(),
            &env,
            &RuntimeContext::new(),
            &mut log,
        );
        assert_eq!(resolved.source, OrgSource::None, "case {label}");
        assert_eq!(resolved.org, None, "case {label}");
    }
}

/// Trace: FR-025-AC-4
#[test]
fn tc_381_107_missing_repository_metadata_resolves_to_none() {
    let root = tempfile::tempdir().expect("temp dir");
    let repo: &Path = root.path();
    let env = FixedEnvironment::new()
        .with_var("XDG_CONFIG_HOME", root.path().join("xdg").to_string_lossy());
    let mut log = IncidentLog::new();
    let resolved = resolve_org(
        repo,
        &OrgOptions::default(),
        &env,
        &RuntimeContext::new(),
        &mut log,
    );
    assert_eq!(resolved, quoin_config::ResolvedOrg::unresolved());
    assert!(
        quoin_config::UNRESOLVED_ORG_MESSAGE.contains("--org"),
        "the remedy must reach the author"
    );
}
