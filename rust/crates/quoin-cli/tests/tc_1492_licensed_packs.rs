// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Licensed-pack detection and placement through the shipped binary
//! (FR-137, FR-138). The pack verdict is the engine's `check_publishable`;
//! these tests drive it end to end over synthetic `example.invalid` modules.

#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]

mod pack_support;

use ix_trace_rs::trace;
use pack_support::{SENTINEL, git_init, quoin, scratch, snapshot, text, write_module};

/// A repository directory with a `.git` entry, holding a licensed module.
fn pack_in_work_tree(scratch: &std::path::Path) -> (std::path::PathBuf, std::path::PathBuf) {
    let repo = scratch.join("repo");
    git_init(&repo);
    let module = write_module(&repo.join("mods"), "widget-pack", "licensed");
    (repo, module)
}

fn source_arg(dir: &std::path::Path) -> String {
    format!("path:{}", dir.display())
}

#[trace("FR-137-AC-1")]
#[test]
fn tc_1492_100_a_pack_outside_every_work_tree_installs_and_lists_as_a_pack() {
    let scratch = scratch();
    let home = scratch.path().join("ix-home");
    let module = write_module(&scratch.path().join("src"), "widget-pack", "licensed");
    let installed = quoin(
        &home,
        scratch.path(),
        &["module", "install", &source_arg(&module)],
        &[],
    );
    assert!(installed.status.success(), "{}", text(&installed));
    let listed = quoin(&home, scratch.path(), &["module", "list"], &[]);
    assert!(listed.status.success(), "{}", text(&listed));
    assert!(
        text(&listed).contains("\"pack\": true"),
        "{}",
        text(&listed)
    );
}

#[trace("FR-137-AC-2", "FR-137-AC-5")]
#[test]
fn tc_1492_101_a_pack_in_a_work_tree_is_refused_naming_source_root_and_set() {
    let scratch = scratch();
    let home = scratch.path().join("ix-home");
    let (repo, module) = pack_in_work_tree(scratch.path());
    let before = snapshot(&home);

    let refused = quoin(
        &home,
        scratch.path(),
        &["module", "install", &source_arg(&module)],
        &[],
    );
    assert!(!refused.status.success(), "{}", text(&refused));
    let output = text(&refused);
    let repo = repo.canonicalize().unwrap();
    assert!(
        output.contains(&module.canonicalize().unwrap().display().to_string()),
        "{output}"
    );
    assert!(output.contains(&repo.display().to_string()), "{output}");
    assert!(output.contains("example.invalid/rules@2"), "{output}");
    assert!(output.contains("outside any git work tree"), "{output}");
    assert!(!output.contains(SENTINEL), "{output}");
    assert_eq!(snapshot(&home), before);
}

#[trace("FR-137-AC-3")]
#[test]
fn tc_1492_102_a_module_without_licensed_text_installs_from_a_work_tree() {
    let scratch = scratch();
    let home = scratch.path().join("ix-home");
    let repo = scratch.path().join("repo");
    git_init(&repo);
    let mut names = Vec::new();
    for (name, rights) in [("a-none", "none"), ("b-original", "original")] {
        let module = write_module(&repo.join("mods"), name, rights);
        let installed = quoin(
            &home,
            scratch.path(),
            &["module", "install", &source_arg(&module)],
            &[],
        );
        assert!(installed.status.success(), "{}", text(&installed));
        names.push(name);
    }
    let listed = text(&quoin(&home, scratch.path(), &["module", "list"], &[]));
    assert_eq!(listed.matches("\"pack\": false").count(), 2, "{listed}");
}

#[trace("FR-137-AC-4")]
#[test]
fn tc_1492_103_a_git_file_or_a_stray_empty_git_dir_above_the_source_refuses() {
    let scratch = scratch();
    let home = scratch.path().join("ix-home");
    let linked = scratch.path().join("linked");
    std::fs::create_dir_all(&linked).unwrap();
    std::fs::write(linked.join(".git"), "gitdir: /elsewhere\n").unwrap();
    let stray = scratch.path().join("stray");
    std::fs::create_dir_all(stray.join(".git")).unwrap();
    for tree in [&linked, &stray] {
        let module = write_module(&tree.join("mods"), "widget-pack", "licensed");
        let refused = quoin(
            &home,
            scratch.path(),
            &["module", "install", &source_arg(&module)],
            &[],
        );
        assert!(!refused.status.success(), "{}", text(&refused));
    }
}

#[trace("FR-137-AC-6")]
#[test]
fn tc_1492_104_the_registry_record_is_the_same_shape_and_the_verdict_is_recomputed() {
    let scratch = scratch();
    let home = scratch.path().join("ix-home");
    let src = scratch.path().join("src");
    let pack = write_module(&src, "widget-pack", "licensed");
    let plain = write_module(&src, "plain-module", "none");
    for module in [&pack, &plain] {
        let installed = quoin(
            &home,
            scratch.path(),
            &["module", "install", &source_arg(module)],
            &[],
        );
        assert!(installed.status.success(), "{}", text(&installed));
    }
    let registry: serde_json::Value =
        serde_json::from_slice(&std::fs::read(home.join("filament/registry.json")).unwrap())
            .unwrap();
    let keys = |entry: &serde_json::Value| -> Vec<String> {
        entry.as_object().unwrap().keys().cloned().collect()
    };
    let entries = registry["plugins"].as_array().unwrap();
    assert_eq!(entries.len(), 2);
    assert_eq!(keys(&entries[0]), keys(&entries[1]));

    let installed_rules = home.join("filament/modules/widget-pack/rules.yaml");
    std::fs::write(
        &installed_rules,
        pack_support::clause_set_yaml("rules", "original"),
    )
    .unwrap();
    let listed = text(&quoin(&home, scratch.path(), &["module", "list"], &[]));
    assert!(!listed.contains("\"pack\": true"), "{listed}");
}

#[trace("FR-137-AC-8")]
#[test]
fn tc_1492_106_a_clause_set_without_rights_fails_install_with_the_load_error() {
    let scratch = scratch();
    let home = scratch.path().join("ix-home");
    let module = write_module(&scratch.path().join("src"), "widget-pack", "licensed");
    std::fs::write(
        module.join("rules.yaml"),
        "schemaVersion: clause-set-v1\nauthority: example.invalid\nid: bare\ntitle: Bare\n\
         version: \"1\"\nclauses: []\n",
    )
    .unwrap();
    let refused = quoin(
        &home,
        scratch.path(),
        &["module", "install", &source_arg(&module)],
        &[],
    );
    assert!(!refused.status.success(), "{}", text(&refused));
    assert!(text(&refused).contains("rules.yaml"), "{}", text(&refused));
    assert!(!home.join("filament/modules/widget-pack").exists());
    let listed = text(&quoin(&home, scratch.path(), &["module", "list"], &[]));
    assert!(!listed.contains("widget-pack"), "{listed}");
}

#[trace("FR-137-AC-9")]
#[test]
fn tc_1492_107_a_symlink_is_judged_by_what_it_points_at() {
    let scratch = scratch();
    let home = scratch.path().join("ix-home");
    let (_repo, inside) = pack_in_work_tree(scratch.path());
    let outside = write_module(&scratch.path().join("clean"), "widget-pack", "licensed");

    let to_inside = scratch.path().join("link-to-inside");
    std::os::unix::fs::symlink(&inside, &to_inside).unwrap();
    let refused = quoin(
        &home,
        scratch.path(),
        &["module", "install", &source_arg(&to_inside)],
        &[],
    );
    assert!(!refused.status.success(), "{}", text(&refused));

    let to_outside = scratch.path().join("repo/link-to-outside");
    std::os::unix::fs::symlink(&outside, &to_outside).unwrap();
    let installed = quoin(
        &home,
        scratch.path(),
        &["module", "install", &source_arg(&to_outside)],
        &[],
    );
    assert!(installed.status.success(), "{}", text(&installed));
}

#[trace("FR-137-AC-10")]
#[test]
fn tc_1492_108_gitignore_and_the_git_environment_do_not_change_the_verdict() {
    let scratch = scratch();
    let home = scratch.path().join("ix-home");
    let (repo, module) = pack_in_work_tree(scratch.path());
    std::fs::write(repo.join(".gitignore"), "mods/\n").unwrap();
    let elsewhere = scratch.path().join("elsewhere");
    std::fs::create_dir_all(&elsewhere).unwrap();
    let refused = quoin(
        &home,
        scratch.path(),
        &["module", "install", &source_arg(&module)],
        &[("GIT_DIR", &elsewhere), ("GIT_WORK_TREE", &elsewhere)],
    );
    assert!(!refused.status.success(), "{}", text(&refused));
}

#[cfg(unix)]
#[trace("FR-137-AC-11")]
#[test]
fn tc_1492_109_an_unreadable_clause_set_costs_only_its_own_row() {
    use std::os::unix::fs::PermissionsExt;

    let scratch = scratch();
    let home = scratch.path().join("ix-home");
    let src = scratch.path().join("src");
    for (name, rights) in [("widget-pack", "licensed"), ("plain-module", "none")] {
        let module = write_module(&src, name, rights);
        let installed = quoin(
            &home,
            scratch.path(),
            &["module", "install", &source_arg(&module)],
            &[],
        );
        assert!(installed.status.success(), "{}", text(&installed));
    }
    let rules = home.join("filament/modules/widget-pack/rules.yaml");
    std::fs::set_permissions(&rules, std::fs::Permissions::from_mode(0o000)).unwrap();
    if std::fs::read(&rules).is_ok() {
        // Running with privileges that ignore file modes: no way to make it unreadable.
        return;
    }
    let listed = quoin(&home, scratch.path(), &["module", "list"], &[]);
    std::fs::set_permissions(&rules, std::fs::Permissions::from_mode(0o644)).unwrap();
    assert!(listed.status.success(), "{}", text(&listed));
    let output = text(&listed);
    assert!(output.contains("\"pack\": null"), "{output}");
    assert!(output.contains("loadError"), "{output}");
    assert!(output.contains("rules.yaml"), "{output}");
    assert!(output.contains("plain-module"), "{output}");
    assert!(output.contains("\"pack\": false"), "{output}");
}
