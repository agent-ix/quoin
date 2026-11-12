// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Where a pack may be installed from (FR-137, FR-138).
//!
//! The rights decision is the engine's and is proved in `quoin-quire`; here a
//! probe stands in for it (a module directory holding a `PACK` file is a pack)
//! so the placement rules are proved over every source kind with no network.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use ix_trace_rs::trace;
use quoin_modules::git::{GitResolver, ResolvedGitSource, Revision};
use quoin_modules::install::ModuleInstaller;
use quoin_modules::manifest::{MarketplaceEntry, MarketplaceManifest};
use quoin_modules::pack::enclosing_work_tree;
use quoin_modules::reconcile::ensure_defaults;
use quoin_modules::{
    CommitSha, InstallPaths, IxHome, ModuleName, ModulesError, ModulesErrorCode, PackProbe,
    PermissiveGate, RestrictedSet, Source,
};

const SENTINEL: &str = "SENTINEL-sprocket-text-7731";

/// A module directory holding a `PACK` file is a pack; one holding `BROKEN`
/// does not load.
struct MarkerProbe;

impl PackProbe for MarkerProbe {
    fn restricted_sets(&self, module_dir: &Path) -> Result<Vec<RestrictedSet>, String> {
        if module_dir.join("BROKEN").exists() {
            return Err("clause set unreadable".to_owned());
        }
        Ok(if module_dir.join("PACK").exists() {
            vec![RestrictedSet {
                authority: "example.invalid".to_owned(),
                id: "widget-rules".to_owned(),
                version: "2".to_owned(),
            }]
        } else {
            Vec::new()
        })
    }
}

/// Serves one fixed directory for every url.
struct FixedResolver(PathBuf);

impl GitResolver for FixedResolver {
    fn resolve(
        &self,
        _url: &str,
        revision: Revision<'_>,
        _subdir: Option<&str>,
    ) -> Result<ResolvedGitSource, ModulesError> {
        Ok(ResolvedGitSource {
            dir: self.0.clone(),
            sha: CommitSha::new("ab".repeat(20)).unwrap(),
            requested_ref: match revision {
                Revision::Ref(r) => Some(r.to_owned()),
                Revision::Head => None,
            },
        })
    }
}

/// A base directory with no `.git` at or above it. The default temp directory
/// of a development machine may sit inside a work tree, which would make every
/// "outside any work tree" case false.
fn clean_base() -> PathBuf {
    [std::env::temp_dir(), "/var/tmp".into(), "/dev/shm".into()]
        .into_iter()
        .filter_map(|dir| dir.canonicalize().ok())
        .find(|dir| enclosing_work_tree(dir).is_none())
        .expect("a temp base outside every git work tree")
}

struct Sandbox {
    root: tempfile::TempDir,
    paths: InstallPaths,
}

fn sandbox() -> Sandbox {
    let root = tempfile::tempdir_in(clean_base()).unwrap();
    let canonical = root.path().canonicalize().unwrap();
    Sandbox {
        paths: InstallPaths::for_home(&IxHome::new(canonical.join("ixhome"))),
        root,
    }
}

impl Sandbox {
    fn module(&self, parent: &str, name: &str, marker: Option<&str>) -> PathBuf {
        let dir = self
            .root
            .path()
            .canonicalize()
            .unwrap()
            .join(parent)
            .join(name);
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("manifest.yaml"), format!("name: {name}\n")).unwrap();
        fs::write(dir.join("clauses.yaml"), SENTINEL).unwrap();
        if let Some(marker) = marker {
            fs::write(dir.join(marker), "").unwrap();
        }
        dir
    }

    /// Every file under the home with its bytes, for before/after comparison.
    fn home_snapshot(&self) -> BTreeMap<PathBuf, Vec<u8>> {
        fn walk(dir: &Path, out: &mut BTreeMap<PathBuf, Vec<u8>>) {
            let Ok(entries) = fs::read_dir(dir) else {
                return;
            };
            for entry in entries {
                let path = entry.unwrap().path();
                if path.is_dir() {
                    out.insert(path.clone(), Vec::new());
                    walk(&path, out);
                } else {
                    out.insert(path.clone(), fs::read(&path).unwrap());
                }
            }
        }
        let mut out = BTreeMap::new();
        walk(self.paths.registry_path.parent().unwrap(), &mut out);
        out
    }
}

fn install(sb: &Sandbox, resolver: &dyn GitResolver, source: &Source) -> Result<(), ModulesError> {
    let gate = PermissiveGate;
    ModuleInstaller::new(sb.paths.clone(), resolver, &gate, &MarkerProbe)
        .install_ad_hoc(source)
        .map(|_| ())
}

fn path_source(dir: &Path) -> Source {
    Source::Path {
        path: dir.to_string_lossy().into_owned(),
    }
}

fn hosted_sources() -> Vec<Source> {
    vec![
        Source::Git {
            url: "https://example.invalid/pack.git".to_owned(),
            r#ref: None,
        },
        Source::GitSubdir {
            url: "https://example.invalid/pack.git".to_owned(),
            path: "sub".to_owned(),
            r#ref: None,
        },
        Source::Github {
            repo: "example/pack".to_owned(),
            r#ref: None,
        },
    ]
}

#[trace("FR-137-AC-1", "FR-138-AC-5")]
#[test]
fn tc_1492_030_a_pack_outside_every_work_tree_installs_from_a_path() {
    let sb = sandbox();
    let src = sb.module("src", "widget-pack", Some("PACK"));
    install(&sb, &FixedResolver(src.clone()), &path_source(&src)).expect("installs");
    assert!(sb.paths.target_root.join("widget-pack").is_dir());
}

#[trace("FR-137-AC-2", "FR-137-AC-5")]
#[test]
fn tc_1492_031_a_pack_under_a_work_tree_is_refused_and_nothing_is_written() {
    let sb = sandbox();
    let tree = sb.root.path().canonicalize().unwrap().join("repo");
    fs::create_dir_all(tree.join(".git")).unwrap();
    let src = sb.module("repo/mods", "widget-pack", Some("PACK"));
    let before = sb.home_snapshot();

    let error = install(&sb, &FixedResolver(src.clone()), &path_source(&src)).unwrap_err();
    assert_eq!(error.code(), ModulesErrorCode::PackInsideWorkTree);
    let text = error.to_string();
    assert!(text.contains(&src.display().to_string()), "{text}");
    assert!(text.contains(&tree.display().to_string()), "{text}");
    assert!(text.contains("example.invalid/widget-rules@2"), "{text}");
    assert!(text.contains("outside any git work tree"), "{text}");
    assert!(!text.contains(SENTINEL), "{text}");
    assert_eq!(sb.home_snapshot(), before);
    assert!(!sb.paths.target_root.exists());
}

#[trace("FR-137-AC-3")]
#[test]
fn tc_1492_032_a_non_pack_installs_from_inside_a_work_tree() {
    let sb = sandbox();
    fs::create_dir_all(sb.root.path().join("repo/.git")).unwrap();
    let src = sb.module("repo/mods", "plain", None);
    install(&sb, &FixedResolver(src.clone()), &path_source(&src)).expect("installs");
}

#[trace("FR-137-AC-4")]
#[test]
fn tc_1492_033_a_git_file_above_the_source_refuses_a_pack() {
    let sb = sandbox();
    let tree = sb.root.path().canonicalize().unwrap().join("linked");
    fs::create_dir_all(&tree).unwrap();
    fs::write(tree.join(".git"), "gitdir: /elsewhere\n").unwrap();
    let src = sb.module("linked/mods", "widget-pack", Some("PACK"));
    let error = install(&sb, &FixedResolver(src.clone()), &path_source(&src)).unwrap_err();
    assert_eq!(error.code(), ModulesErrorCode::PackInsideWorkTree);
}

#[trace("FR-137-AC-7")]
#[test]
fn tc_1492_034_a_modules_directory_inside_a_work_tree_refuses_a_pack() {
    let sb = sandbox();
    let root = sb.root.path().canonicalize().unwrap();
    fs::create_dir_all(root.join("ixhome/.git")).unwrap();
    let src = sb.module("src", "widget-pack", Some("PACK"));
    let error = install(&sb, &FixedResolver(src.clone()), &path_source(&src)).unwrap_err();
    assert_eq!(error.code(), ModulesErrorCode::PackInsideWorkTree);
    assert!(
        error
            .to_string()
            .contains(&root.join("ixhome").display().to_string())
    );
    assert!(!sb.paths.target_root.exists());
    assert!(!sb.paths.registry_path.exists());
}

#[trace("FR-137-AC-9")]
#[test]
fn tc_1492_035_symlinks_are_resolved_before_the_work_tree_is_judged() {
    let sb = sandbox();
    let root = sb.root.path().canonicalize().unwrap();
    fs::create_dir_all(root.join("repo/.git")).unwrap();
    let inside = sb.module("repo/mods", "widget-pack", Some("PACK"));
    let outside = sb.module("clean", "widget-pack", Some("PACK"));

    let link_to_inside = root.join("link-to-inside");
    std::os::unix::fs::symlink(&inside, &link_to_inside).unwrap();
    let error = install(
        &sb,
        &FixedResolver(inside.clone()),
        &path_source(&link_to_inside),
    )
    .unwrap_err();
    assert_eq!(error.code(), ModulesErrorCode::PackInsideWorkTree);

    let link_to_outside = root.join("repo/link-to-outside");
    std::os::unix::fs::symlink(&outside, &link_to_outside).unwrap();
    install(
        &sb,
        &FixedResolver(outside.clone()),
        &path_source(&link_to_outside),
    )
    .expect("installs");
}

#[trace("FR-137-AC-8")]
#[test]
fn tc_1492_037_a_module_whose_clause_sets_do_not_load_fails_install() {
    let sb = sandbox();
    let src = sb.module("src", "broken", Some("BROKEN"));
    let error = install(&sb, &FixedResolver(src.clone()), &path_source(&src)).unwrap_err();
    assert_eq!(error.code(), ModulesErrorCode::PackLoadFailed);
    assert!(error.to_string().contains("clause set unreadable"));
    assert!(!sb.paths.target_root.exists());
}

#[trace("FR-138-AC-1", "FR-138-AC-4")]
#[test]
fn tc_1492_040_a_pack_from_any_hosted_source_is_refused_and_nothing_is_written() {
    let sb = sandbox();
    let tree = sb.module("cache", "widget-pack", Some("PACK"));
    let before = sb.home_snapshot();
    for source in hosted_sources() {
        let error = install(&sb, &FixedResolver(tree.clone()), &source).unwrap_err();
        assert_eq!(
            error.code(),
            ModulesErrorCode::PackSourceRefused,
            "{source:?}"
        );
        let text = error.to_string();
        assert!(text.contains("widget-pack"), "{text}");
        assert!(text.contains(source.type_name()), "{text}");
        assert!(text.contains("example.invalid/widget-rules@2"), "{text}");
        assert!(!text.contains(SENTINEL), "{text}");
        assert_eq!(sb.home_snapshot(), before);
    }
    assert!(!sb.paths.target_root.exists());
}

#[trace("FR-138-AC-2")]
#[test]
fn tc_1492_041_a_non_pack_installs_from_every_hosted_source() {
    let sb = sandbox();
    let tree = sb.module("cache", "plain", None);
    for source in hosted_sources() {
        install(&sb, &FixedResolver(tree.clone()), &source).expect("installs");
    }
}

#[trace("FR-138-AC-3", "FR-138-AC-4")]
#[test]
fn tc_1492_042_ensure_defaults_refuses_a_pack_entry_and_registers_nothing() {
    let sb = sandbox();
    let tree = sb.module("cache", "widget-pack", Some("PACK"));
    let manifest = MarketplaceManifest {
        schema_version: 1,
        name: Some("defaults".to_owned()),
        entries: vec![MarketplaceEntry {
            name: ModuleName::new("widget-pack").unwrap(),
            source: path_source(&tree),
            default_enabled: None,
            path: None,
        }],
    };
    let resolver = FixedResolver(tree);
    let gate = PermissiveGate;
    let installer = ModuleInstaller::new(sb.paths.clone(), &resolver, &gate, &MarkerProbe);
    let error = ensure_defaults(&manifest, &installer).unwrap_err();
    assert_eq!(error.code(), ModulesErrorCode::PackSourceRefused);
    let text = error.to_string();
    assert!(text.contains("widget-pack"), "{text}");
    assert!(!text.contains(SENTINEL), "{text}");
    assert!(installer.list().unwrap().is_empty());
    assert!(!sb.paths.target_root.exists());
}
