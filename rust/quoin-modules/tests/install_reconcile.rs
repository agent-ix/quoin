// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Install, rollback and reconcile behaviours (quoin#381, FR-019).
//!
//! The git resolver is a seam, so these run with no network. The *decorator*
//! pattern is used rather than a stub: [`CountingResolver`] wraps a real
//! directory-backed resolution and counts calls, so "lazy reconcile performs no
//! network I/O" is measured rather than assumed.

use std::cell::RefCell;
use std::fs;
use std::path::{Path, PathBuf};

use quoin_modules::git::{GitResolver, ResolvedGitSource, Revision};
use quoin_modules::install::ModuleInstaller;
use quoin_modules::manifest::{MarketplaceEntry, MarketplaceManifest};
use quoin_modules::reconcile::{ensure_defaults, reconcile, ReconcileMode};
use quoin_modules::registry::ModuleRegistry;
use quoin_modules::semantic::{Diagnostic, SemanticGate, SemanticVerdict, Severity};
use quoin_modules::{
    parse_source_arg, CommitSha, InstallPaths, IxHome, ModuleName, ModulesError, ModulesErrorCode,
    PermissiveGate, RollbackOutcome, Source,
};

/// A resolver that serves a fixed directory per `(url, revision)` and counts
/// how many times it was asked — the "network" for these tests.
struct CountingResolver {
    trees: Vec<(String, String, PathBuf)>,
    calls: RefCell<usize>,
}

impl CountingResolver {
    fn new() -> Self {
        Self {
            trees: Vec::new(),
            calls: RefCell::new(0),
        }
    }

    fn serving(mut self, url: &str, revision: &str, dir: impl Into<PathBuf>) -> Self {
        self.trees
            .push((url.to_owned(), revision.to_owned(), dir.into()));
        self
    }

    fn calls(&self) -> usize {
        *self.calls.borrow()
    }
}

impl GitResolver for CountingResolver {
    fn resolve(
        &self,
        url: &str,
        revision: Revision<'_>,
        _subdir: Option<&str>,
    ) -> Result<ResolvedGitSource, ModulesError> {
        *self.calls.borrow_mut() += 1;
        let wanted = revision.as_str();
        let dir = self
            .trees
            .iter()
            .find(|(u, r, _)| u == url && r == wanted)
            .map(|(_, _, d)| d.clone())
            .ok_or_else(|| ModulesError::GitRevisionNotFound {
                url: url.to_owned(),
                revision: wanted.to_owned(),
            })?;
        Ok(ResolvedGitSource {
            dir,
            sha: CommitSha::new(sha_for(wanted)).expect("fixture sha is 40 hex"),
            requested_ref: match revision {
                Revision::Ref(r) => Some(r.to_owned()),
                Revision::Sha(_) | Revision::Head => None,
            },
        })
    }
}

/// A deterministic 40-hex fixture sha derived from a revision name.
fn sha_for(revision: &str) -> String {
    let mut out = String::new();
    for byte in revision.bytes().cycle().take(20) {
        out.push_str(&format!("{byte:02x}"));
    }
    out
}

/// A gate that refuses named modules, so the rollback paths are reachable.
struct RefusingGate {
    refuse: Vec<String>,
}

impl SemanticGate for RefusingGate {
    fn inspect(&self, name: &ModuleName, _root: &Path) -> SemanticVerdict {
        if self.refuse.iter().any(|n| n == name.as_str()) {
            return SemanticVerdict {
                diagnostics: vec![Diagnostic {
                    severity: Severity::Error,
                    rule: "SEM-001".to_owned(),
                    message: "data_schema reference is outside the contract".to_owned(),
                }],
                pin: None,
            };
        }
        SemanticVerdict::default()
    }
}

/// Write a module fixture directory with a `manifest.yaml` and a marker file.
fn write_module(root: &Path, name: &str, marker: &str) -> PathBuf {
    let dir = root.join(name);
    fs::create_dir_all(dir.join("artifacts")).expect("fixture dirs");
    fs::write(dir.join("manifest.yaml"), format!("name: {name}\n")).expect("manifest");
    fs::write(dir.join("artifacts").join("marker.txt"), marker).expect("marker");
    dir
}

struct Home {
    _root: tempfile::TempDir,
    paths: InstallPaths,
    fixtures: PathBuf,
}

fn home() -> Home {
    let root = tempfile::tempdir().expect("temp dir");
    let fixtures = root.path().join("fixtures");
    fs::create_dir_all(&fixtures).expect("fixtures dir");
    Home {
        paths: InstallPaths::for_home(&IxHome::new(root.path().join("ixhome"))),
        fixtures,
        _root: root,
    }
}

/// Trace: FR-019-AC-2
#[test]
fn tc_381_280_install_lists_and_removes_a_path_module() {
    let h = home();
    let src = write_module(&h.fixtures, "spec-objects-business", "v1");
    let resolver = CountingResolver::new();
    let gate = PermissiveGate;
    let installer = ModuleInstaller::new(h.paths.clone(), &resolver, &gate);

    let source = parse_source_arg(&format!("path:{}", src.display())).expect("path source");
    let outcome = installer.install_ad_hoc(&source).expect("install succeeds");
    assert_eq!(outcome.module.name.as_str(), "spec-objects-business");
    assert!(!outcome.replaced_previous);

    let target = h.paths.target_root.join("spec-objects-business");
    assert_eq!(
        fs::read_to_string(target.join("artifacts").join("marker.txt")).expect("copied"),
        "v1"
    );
    assert!(h.paths.registry_path.is_file());
    assert_eq!(
        installer
            .list()
            .expect("list")
            .iter()
            .map(|m| m.name.to_string())
            .collect::<Vec<_>>(),
        vec!["spec-objects-business".to_owned()]
    );

    let name = ModuleName::new("spec-objects-business").expect("legal name");
    installer.remove(&name).expect("remove succeeds");
    assert!(installer.list().expect("list").is_empty());
    assert!(!target.exists());
    assert_eq!(
        installer.remove(&name).expect_err("already removed").code(),
        ModulesErrorCode::ModuleNotInstalled
    );
}

/// Trace: FR-019-AC-2
#[test]
fn tc_381_281_a_rejected_reinstall_restores_the_previous_version() {
    let h = home();
    let good = write_module(&h.fixtures, "mod", "good");
    let resolver = CountingResolver::new();
    let permissive = PermissiveGate;

    let source = parse_source_arg(&format!("path:{}", good.display())).expect("path source");
    ModuleInstaller::new(h.paths.clone(), &resolver, &permissive)
        .install_ad_hoc(&source)
        .expect("first install");

    // Now offer a different tree under the same module name, and refuse it.
    let bad_root = h.fixtures.join("bad");
    let bad = write_module(&bad_root, "mod", "bad");
    let refusing = RefusingGate {
        refuse: vec!["mod".to_owned()],
    };
    let installer = ModuleInstaller::new(h.paths.clone(), &resolver, &refusing);
    let bad_source = parse_source_arg(&format!("path:{}", bad.display())).expect("path source");
    let err = installer
        .install_ad_hoc(&bad_source)
        .expect_err("the gate refuses this module");

    match err {
        ModulesError::SemanticContractViolation { rollback, .. } => {
            assert_eq!(rollback, RollbackOutcome::Restored);
        }
        other => panic!("expected a contract violation, got {other}"),
    }

    // The previous content, not the rejected content, is what is on disk.
    let target = h.paths.target_root.join("mod");
    assert_eq!(
        fs::read_to_string(target.join("artifacts").join("marker.txt")).expect("restored"),
        "good",
        "a rejected reinstall must leave the previous version in place"
    );
    let registry = ModuleRegistry::read(&h.paths.registry_path).expect("registry");
    let entry = registry
        .find(&ModuleName::new("mod").expect("legal"))
        .expect("still registered");
    assert!(
        entry
            .resolved_path
            .contains(&good.to_string_lossy().to_string()),
        "the registry must point at the restored source, got {}",
        entry.resolved_path
    );
}

/// Trace: FR-019-AC-2
#[test]
fn tc_381_282_a_rejected_first_install_leaves_nothing_behind() {
    let h = home();
    let src = write_module(&h.fixtures, "mod", "bad");
    let resolver = CountingResolver::new();
    let refusing = RefusingGate {
        refuse: vec!["mod".to_owned()],
    };
    let installer = ModuleInstaller::new(h.paths.clone(), &resolver, &refusing);
    let source = parse_source_arg(&format!("path:{}", src.display())).expect("path source");
    let err = installer
        .install_ad_hoc(&source)
        .expect_err("the gate refuses this module");
    match err {
        ModulesError::SemanticContractViolation { rollback, .. } => {
            assert_eq!(rollback, RollbackOutcome::RemovedFreshInstall);
        }
        other => panic!("expected a contract violation, got {other}"),
    }
    assert!(!h.paths.target_root.join("mod").exists());
    assert!(ModuleRegistry::read(&h.paths.registry_path)
        .expect("registry")
        .plugins
        .is_empty());
}

fn git_manifest(name: &str, r#ref: &str, sha: Option<&str>) -> MarketplaceManifest {
    MarketplaceManifest {
        schema_version: 1,
        name: Some("test-set".to_owned()),
        entries: vec![MarketplaceEntry {
            name: ModuleName::new(name).expect("legal name"),
            source: Source::Github {
                repo: "agent-ix/fixture".to_owned(),
                r#ref: Some(r#ref.to_owned()),
                sha: sha.map(ToOwned::to_owned),
            },
            version: None,
            default_enabled: None,
            path: None,
        }],
    }
}

/// Trace: FR-019-AC-2
#[test]
fn tc_381_283_a_settled_lazy_reconcile_performs_no_network_io() {
    let h = home();
    let tree = write_module(&h.fixtures, "mod", "v1");
    let resolver =
        CountingResolver::new().serving("https://github.com/agent-ix/fixture.git", "v1.0.0", tree);
    let gate = PermissiveGate;
    let installer = ModuleInstaller::new(h.paths.clone(), &resolver, &gate);
    let manifest = git_manifest("mod", "v1.0.0", None);

    let first = ensure_defaults(&manifest, &installer).expect("first reconcile");
    assert_eq!(
        first.installed,
        vec![ModuleName::new("mod").expect("legal")]
    );
    assert_eq!(resolver.calls(), 1, "the first reconcile must fetch");

    let second = ensure_defaults(&manifest, &installer).expect("second reconcile");
    assert_eq!(
        second.unchanged,
        vec![ModuleName::new("mod").expect("legal")]
    );
    assert!(!second.touched_network());
    assert_eq!(
        resolver.calls(),
        1,
        "a settled lazy reconcile must not resolve anything — it runs before \
         every catalog read"
    );
}

/// Trace: FR-019-AC-2
#[test]
fn tc_381_284_sync_mode_re_resolves_and_a_repin_updates() {
    let h = home();
    let v1 = write_module(&h.fixtures, "mod", "v1");
    let v2root = h.fixtures.join("v2");
    let v2 = write_module(&v2root, "mod", "v2");
    let resolver = CountingResolver::new()
        .serving("https://github.com/agent-ix/fixture.git", "v1.0.0", v1)
        .serving("https://github.com/agent-ix/fixture.git", "v2.0.0", v2);
    let gate = PermissiveGate;
    let installer = ModuleInstaller::new(h.paths.clone(), &resolver, &gate);

    ensure_defaults(&git_manifest("mod", "v1.0.0", None), &installer).expect("install v1");
    assert_eq!(resolver.calls(), 1);

    // Sync re-resolves even a matching pin.
    let synced = reconcile(
        &git_manifest("mod", "v1.0.0", None),
        &installer,
        ReconcileMode::Sync,
    )
    .expect("sync reconcile");
    assert_eq!(resolver.calls(), 2);
    assert_eq!(
        synced.unchanged,
        vec![ModuleName::new("mod").expect("legal")],
        "the same commit re-resolved is unchanged, not updated"
    );

    // A re-pinned entry updates, and the new content lands.
    let updated =
        ensure_defaults(&git_manifest("mod", "v2.0.0", None), &installer).expect("repin reconcile");
    assert_eq!(
        updated.updated,
        vec![ModuleName::new("mod").expect("legal")]
    );
    assert_eq!(
        fs::read_to_string(
            h.paths
                .target_root
                .join("mod")
                .join("artifacts")
                .join("marker.txt")
        )
        .expect("new content"),
        "v2"
    );
}

/// Trace: FR-019-AC-2
#[test]
fn tc_381_285_a_disabled_entry_is_skipped_and_never_resolved() {
    let h = home();
    let resolver = CountingResolver::new();
    let gate = PermissiveGate;
    let installer = ModuleInstaller::new(h.paths.clone(), &resolver, &gate);
    let mut manifest = git_manifest("mod", "v1.0.0", None);
    manifest.entries[0].default_enabled = Some(false);

    let report = ensure_defaults(&manifest, &installer).expect("reconcile");
    assert_eq!(report.skipped, vec![ModuleName::new("mod").expect("legal")]);
    assert_eq!(resolver.calls(), 0);
}

/// Trace: FR-019-AC-2
#[test]
fn tc_381_286_an_orphaned_pin_is_reported_not_silently_followed_to_head() {
    // The first-integration pin 2a66359 was orphaned when upstream history was
    // rebuilt; upload-pack refused it and clean-room installs failed while
    // already-installed machines stayed green (agent-ix/quoin#308). A resolver
    // that quietly fell back to HEAD would have hidden that.
    let h = home();
    let resolver = CountingResolver::new();
    let gate = PermissiveGate;
    let installer = ModuleInstaller::new(h.paths.clone(), &resolver, &gate);
    let manifest = git_manifest("mod", "v1.0.0", Some(&sha_for("orphan")));

    let err = ensure_defaults(&manifest, &installer).expect_err("the pin is unreachable");
    assert_eq!(err.code(), ModulesErrorCode::GitRevisionNotFound);
    assert!(
        err.to_string().contains("rebuilt history"),
        "the message must name the cause an author can act on, got: {err}"
    );
}
