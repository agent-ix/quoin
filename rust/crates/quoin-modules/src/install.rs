// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Installing one module, with rollback (quoin#381, FR-019).
//!
//! Ports `installPlugin` from `src/plugins.ts`, whose load-bearing property is
//! not the install but the **rejection**: the materialized copy is overwritten
//! before the semantic block can be read, so a module that fails the contract
//! must leave the previous version in place — or, if there was none, leave
//! nothing behind. A rollback failure is reported *alongside* the rejection, as
//! [`RollbackOutcome`], and never masks it.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use tempfile::Builder;

use crate::error::{ModulesError, RollbackOutcome};
use crate::git::{GitResolver, Revision};
use crate::ids::ModuleName;
use crate::manifest::MarketplaceEntry;
use crate::module_name::{locate_manifest, read_module_name};
use crate::npm::{HttpNpmTransport, NpmTransport};
use crate::pack::{PackProbe, RestrictedSet, enclosing_work_tree};
use crate::paths::InstallPaths;
use crate::registry::{InstalledModule, ModuleRegistry};
use crate::semantic::SemanticGate;
use crate::source::Source;

/// What an install did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstallOutcome {
    /// The registry record that was written.
    pub module: InstalledModule,
    /// Whether a previous version of the same module was replaced.
    pub replaced_previous: bool,
}

/// Installs modules into one home.
///
/// Every collaborator is a constructor argument rather than a default: a
/// resolver so the network is a seam the tests can stand in for, a gate
/// because [`PermissiveGate`](crate::semantic::PermissiveGate) accepting
/// everything must be a choice a caller made, not one it inherited, and a pack
/// probe because [`NoPacks`](crate::pack::NoPacks) waving a licensed module
/// through must be a choice as well.
pub struct ModuleInstaller<'a> {
    paths: InstallPaths,
    resolver: &'a dyn GitResolver,
    npm_transport: &'a dyn NpmTransport,
    gate: &'a dyn SemanticGate,
    packs: &'a dyn PackProbe,
}

/// Which door an install came through.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Origin {
    /// An entry of the default module set.
    Default,
    /// A source the caller named.
    AdHoc,
}

static HTTP_NPM_TRANSPORT: HttpNpmTransport = HttpNpmTransport;

impl<'a> ModuleInstaller<'a> {
    /// Build an installer.
    #[must_use]
    pub fn new(
        paths: InstallPaths,
        resolver: &'a dyn GitResolver,
        gate: &'a dyn SemanticGate,
        packs: &'a dyn PackProbe,
    ) -> Self {
        Self::with_npm_transport(paths, resolver, &HTTP_NPM_TRANSPORT, gate, packs)
    }

    fn with_npm_transport(
        paths: InstallPaths,
        resolver: &'a dyn GitResolver,
        npm_transport: &'a dyn NpmTransport,
        gate: &'a dyn SemanticGate,
        packs: &'a dyn PackProbe,
    ) -> Self {
        Self {
            paths,
            resolver,
            npm_transport,
            gate,
            packs,
        }
    }

    /// The paths this installer writes to.
    #[must_use]
    pub fn paths(&self) -> &InstallPaths {
        &self.paths
    }

    /// Every installed module, in registry order.
    ///
    /// # Errors
    /// [`ModulesError::RegistryUnreadable`].
    pub fn list(&self) -> Result<Vec<InstalledModule>, ModulesError> {
        Ok(ModuleRegistry::read(&self.paths.registry_path)?.plugins)
    }

    /// Remove an installed module and its materialized directory.
    ///
    /// # Errors
    /// [`ModulesError::ModuleNotInstalled`] when nothing by that name is
    /// installed, plus registry read/write failures.
    pub fn remove(&self, name: &ModuleName) -> Result<(), ModulesError> {
        let mut registry = ModuleRegistry::read(&self.paths.registry_path)?;
        if !registry.remove(name) {
            return Err(ModulesError::ModuleNotInstalled { name: name.clone() });
        }
        let target = name.dir_under(&self.paths.target_root);
        remove_dir_if_present(&target)?;
        registry.write(&self.paths.registry_path)
    }

    /// Install one entry, replacing any previous version of the same module.
    ///
    /// # Errors
    /// [`ModulesError::SemanticContractViolation`] when the gate rejects the
    /// module — carrying what happened to the previous version — plus any
    /// resolution, materialization or registry failure.
    pub fn install(&self, entry: &MarketplaceEntry) -> Result<InstallOutcome, ModulesError> {
        self.install_source(
            &entry.source,
            Some(&entry.name),
            entry.path.as_deref(),
            Origin::Default,
        )
    }

    /// Install an ad-hoc source, deriving the module name from its manifest.
    ///
    /// # Errors
    /// As [`ModuleInstaller::install`].
    pub fn install_ad_hoc(&self, source: &Source) -> Result<InstallOutcome, ModulesError> {
        self.install_source(source, None, None, Origin::AdHoc)
    }

    fn install_source(
        &self,
        source: &Source,
        declared_name: Option<&ModuleName>,
        entry_subpath: Option<&str>,
        origin: Origin,
    ) -> Result<InstallOutcome, ModulesError> {
        source.validate()?;
        let resolved = self.resolve(source)?;
        let content_root = match entry_subpath {
            Some(sub) => join_checked(&resolved.dir, sub)?,
            None => resolved.dir.clone(),
        };
        let name = match declared_name {
            Some(name) => name.clone(),
            None => read_module_name(&content_root)?,
        };
        let unloaded = self.refuse_unplaceable_pack(source, origin, &name, &content_root)?;

        // Snapshot before touching anything: the materialized copy is
        // overwritten before the gate can read it, so this is the only record
        // of what to put back.
        let mut registry = ModuleRegistry::read(&self.paths.registry_path)?;
        let previous = registry.find(&name).cloned();

        let target = name.dir_under(&self.paths.target_root);
        fs::create_dir_all(&self.paths.target_root).map_err(|source| {
            ModulesError::MaterializeFailed {
                target: self.paths.target_root.clone(),
                source,
            }
        })?;
        materialize(&content_root, &target)?;

        let record = InstalledModule {
            name: name.clone(),
            source: resolved.source.clone(),
            r#ref: resolved.requested_ref.clone(),
            resolved_path: content_root.to_string_lossy().into_owned(),
            target_path: target.to_string_lossy().into_owned(),
            installed_at: rfc3339_now(),
        };
        registry.upsert(record.clone());
        registry.write(&self.paths.registry_path)?;

        // The semantic contract: a module whose `semantic` block or
        // `data_schema` references are outside the contract is rejected at
        // install, not loaded as an empty model.
        let verdict = self.gate.inspect(&name, &target);
        if verdict.has_errors() {
            let rollback = self.roll_back(&name, previous.as_ref());
            return Err(ModulesError::SemanticContractViolation {
                name,
                report: verdict.render(),
                rollback,
            });
        }

        // A module that did not load for the pack check is judged by the gate
        // first, which owns the reasons a module is unusable, and is failed
        // here when the gate let it through. It is never taken for a non-pack.
        if let Some(load_error) = unloaded {
            let rollback = self.roll_back(&name, previous.as_ref());
            return Err(ModulesError::PackLoadFailed {
                path: content_root,
                detail: load_error,
                rollback,
            });
        }

        Ok(InstallOutcome {
            module: record,
            replaced_previous: previous.is_some(),
        })
    }

    /// Refuse a pack that may not be installed from here (FR-137, FR-138).
    ///
    /// Runs before anything is written, so a refusal leaves the registry and
    /// the modules directory as they were. A module whose clause sets do not
    /// load is never taken for a non-pack: the load error is returned for the
    /// caller to fail with once the semantic gate has had its say.
    fn refuse_unplaceable_pack(
        &self,
        source: &Source,
        origin: Origin,
        name: &ModuleName,
        content_root: &Path,
    ) -> Result<Option<String>, ModulesError> {
        let manifest = locate_manifest(content_root)?;
        let module_dir = manifest.parent().unwrap_or(content_root);
        let restricted = match self.packs.restricted_sets(module_dir) {
            Ok(restricted) => restricted,
            Err(load_error) => return Ok(Some(load_error)),
        };
        if restricted.is_empty() {
            return Ok(None);
        }
        let sets = render_sets(&restricted);
        match (origin, source) {
            (Origin::Default, _) => Err(ModulesError::PackSourceRefused {
                name: name.clone(),
                origin: "the default module set",
                sets,
            }),
            (Origin::AdHoc, Source::Path { .. }) => match enclosing_work_tree(content_root) {
                Some(work_tree) => Err(ModulesError::PackInsideWorkTree {
                    name: name.clone(),
                    path: content_root.to_path_buf(),
                    work_tree,
                    sets,
                }),
                None => Ok(None),
            },
            (Origin::AdHoc, hosted) => Err(ModulesError::PackSourceRefused {
                name: name.clone(),
                origin: hosted.type_name(),
                sets,
            }),
        }
    }

    /// Resolve a source to a content directory.
    fn resolve(&self, source: &Source) -> Result<ResolvedSource, ModulesError> {
        match source {
            Source::Path { path } => {
                let dir = PathBuf::from(path);
                let dir = dir
                    .canonicalize()
                    .map_err(|_| ModulesError::PathSourceNotFound { path: dir.clone() })?;
                Ok(ResolvedSource {
                    dir,
                    requested_ref: None,
                    source: source.clone(),
                })
            }
            Source::Npm { package, .. } => Ok(ResolvedSource {
                dir: crate::npm::resolve_package(
                    &self.paths.cache_root,
                    source,
                    self.npm_transport,
                )?,
                requested_ref: None,
                source: Source::Npm {
                    package: package.clone(),
                    version: None,
                    registry: None,
                },
            }),
            Source::Url { .. } => Err(ModulesError::UnsupportedSource { source_type: "url" }),
            Source::Github { .. } | Source::GitSubdir { .. } | Source::Git { .. } => {
                let url = source.git_url().ok_or(ModulesError::UnsupportedSource {
                    source_type: source.type_name(),
                })?;
                let revision = Revision::of(source.requested_ref());
                let resolved = self.resolver.resolve(&url, revision, source.subdir())?;
                Ok(ResolvedSource {
                    dir: resolved.dir,
                    requested_ref: resolved.requested_ref,
                    source: source.clone(),
                })
            }
        }
    }

    /// Put back what was there before a rejected install.
    fn roll_back(&self, name: &ModuleName, previous: Option<&InstalledModule>) -> RollbackOutcome {
        let attempt = || -> Result<RollbackOutcome, ModulesError> {
            let Some(previous) = previous else {
                let mut registry = ModuleRegistry::read(&self.paths.registry_path)?;
                registry.remove(name);
                remove_dir_if_present(&name.dir_under(&self.paths.target_root))?;
                registry.write(&self.paths.registry_path)?;
                return Ok(RollbackOutcome::RemovedFreshInstall);
            };

            // Re-resolving and re-materializing is what makes the restore real,
            // rather than a registry edit pointing at a directory that still
            // holds the rejected content.
            let resolved = match previous.source {
                Source::Npm { .. } => ResolvedSource {
                    dir: self.cached_npm_source(previous)?,
                    requested_ref: None,
                    source: previous.source.clone(),
                },
                _ => self.resolve(&previous.source)?,
            };
            let target = name.dir_under(&self.paths.target_root);
            materialize(&resolved.dir, &target)?;
            let mut registry = ModuleRegistry::read(&self.paths.registry_path)?;
            registry.upsert(previous.clone());
            registry.write(&self.paths.registry_path)?;
            Ok(RollbackOutcome::Restored)
        };
        attempt().unwrap_or_else(|err| RollbackOutcome::Failed {
            detail: err.to_string(),
        })
    }

    fn cached_npm_source(&self, previous: &InstalledModule) -> Result<PathBuf, ModulesError> {
        let cache_root = self.paths.cache_root.join("npm");
        let cache_root = cache_root
            .canonicalize()
            .map_err(|error| ModulesError::NpmMetadata {
                detail: format!("previous package cache is unavailable: {error}"),
            })?;
        let resolved = PathBuf::from(&previous.resolved_path)
            .canonicalize()
            .map_err(|error| ModulesError::NpmMetadata {
                detail: format!("previous package cache is unavailable: {error}"),
            })?;
        if !resolved.starts_with(&cache_root) {
            return Err(ModulesError::NpmMetadata {
                detail: "previous package cache path is outside the npm cache".to_owned(),
            });
        }
        if read_module_name(&resolved)? != previous.name {
            return Err(ModulesError::NpmMetadata {
                detail: "previous package cache no longer names the installed module".to_owned(),
            });
        }
        Ok(resolved)
    }
}

/// The sets as `authority/id@version`, comma separated.
fn render_sets(sets: &[RestrictedSet]) -> String {
    sets.iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join(", ")
}

/// A source resolved to a directory, whatever its type.
struct ResolvedSource {
    dir: PathBuf,
    requested_ref: Option<String>,
    source: Source,
}

/// Join `sub` onto `base`, refusing anything that escapes it.
fn join_checked(base: &Path, sub: &str) -> Result<PathBuf, ModulesError> {
    if sub.starts_with('/') || sub.split(['/', '\\']).any(|s| s == ".." || s.is_empty()) {
        return Err(ModulesError::UnsafeTreePath {
            entry: sub.to_owned(),
        });
    }
    Ok(base.join(sub))
}

/// Replace `target` with a copy of `content_root`.
fn materialize(content_root: &Path, target: &Path) -> Result<(), ModulesError> {
    let parent = target
        .parent()
        .ok_or_else(|| ModulesError::MaterializeFailed {
            target: target.to_path_buf(),
            source: std::io::Error::new(std::io::ErrorKind::InvalidInput, "target has no parent"),
        })?;
    fs::create_dir_all(parent).map_err(|source| ModulesError::MaterializeFailed {
        target: parent.to_path_buf(),
        source,
    })?;
    let staging = Builder::new()
        .prefix("module-install-")
        .tempdir_in(parent)
        .map_err(|source| ModulesError::MaterializeFailed {
            target: parent.to_path_buf(),
            source,
        })?;
    let staged = staging.path().join("new");
    copy_dir(content_root, &staged)?;

    let previous = staging.path().join("previous");
    let had_previous = match fs::rename(target, &previous) {
        Ok(()) => true,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => false,
        Err(source) => {
            return Err(ModulesError::MaterializeFailed {
                target: target.to_path_buf(),
                source,
            });
        }
    };
    if let Err(source) = fs::rename(&staged, target) {
        if had_previous && let Err(restore_error) = fs::rename(&previous, target) {
            let retained_backup = staging.keep();
            return Err(ModulesError::MaterializeFailed {
                target: target.to_path_buf(),
                source: std::io::Error::new(
                    restore_error.kind(),
                    format!(
                        "install failed ({source}); restoring prior module failed ({restore_error}); previous module retained at {}",
                        retained_backup.join("previous").display()
                    ),
                ),
            });
        }
        return Err(ModulesError::MaterializeFailed {
            target: target.to_path_buf(),
            source,
        });
    }
    Ok(())
}

fn remove_dir_if_present(path: &Path) -> Result<(), ModulesError> {
    match fs::remove_dir_all(path) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(source) => Err(ModulesError::MaterializeFailed {
            target: path.to_path_buf(),
            source,
        }),
    }
}

/// Recursively copy a directory, skipping symlinks.
///
/// A symlink in a source tree can point outside the module, so it is not
/// followed and not recreated — the same rule the git tree extraction applies.
fn copy_dir(from: &Path, to: &Path) -> Result<(), ModulesError> {
    let fail = |target: &Path, source: std::io::Error| ModulesError::MaterializeFailed {
        target: target.to_path_buf(),
        source,
    };
    fs::create_dir_all(to).map_err(|e| fail(to, e))?;
    let entries = fs::read_dir(from).map_err(|e| fail(from, e))?;
    for entry in entries {
        let entry = entry.map_err(|e| fail(from, e))?;
        let file_type = entry.file_type().map_err(|e| fail(from, e))?;
        let target = to.join(entry.file_name());
        if file_type.is_symlink() {
            continue;
        }
        if file_type.is_dir() {
            copy_dir(&entry.path(), &target)?;
        } else {
            fs::copy(entry.path(), &target).map_err(|e| fail(&target, e))?;
        }
    }
    Ok(())
}

/// An RFC 3339 timestamp for the install record.
///
/// Hand-rolled rather than pulling in a date crate: the registry only needs a
/// sortable instant, and the civil-date arithmetic is the whole of it.
///
/// Millisecond precision is not cosmetic. The TypeScript `writeRegistry` writes
/// `Date.prototype.toISOString()`, which always carries exactly three fractional
/// digits, and both implementations write this same registry file during
/// coexistence. A second-precision stamp here is a different string shape in a
/// field the oracle's goldens pin.
fn rfc3339_now() -> String {
    let millis = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map_or(0, |d| d.as_millis());
    let secs = u64::try_from(millis / 1000).unwrap_or(0);
    let milli = u64::try_from(millis % 1000).unwrap_or(0);
    let (days, rem) = (secs / 86_400, secs % 86_400);
    let (hour, minute, second) = (rem / 3600, (rem % 3600) / 60, rem % 60);
    let (year, month, day) = civil_from_days(i64::try_from(days).unwrap_or(0));
    format!("{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}.{milli:03}Z")
}

/// Howard Hinnant's `civil_from_days`, for days since 1970-01-01.
fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    (
        if m <= 2 { y + 1 } else { y },
        u32::try_from(m).unwrap_or(1),
        u32::try_from(d).unwrap_or(1),
    )
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::panic,
        clippy::indexing_slicing,
        reason = "in a test, a panic IS the failure report; the production lints stand"
    )]
    use std::collections::VecDeque;
    use std::io::Cursor;
    use std::sync::Mutex;

    use flate2::Compression;
    use flate2::write::GzEncoder;
    use ix_trace_rs::trace;

    use super::*;
    use crate::npm::{NpmResponse, NpmTransport};
    use crate::semantic::{Diagnostic, SemanticVerdict, Severity};

    struct FixtureNpmTransport(Mutex<VecDeque<Result<NpmResponse, ModulesError>>>);

    impl NpmTransport for FixtureNpmTransport {
        fn get(&self, _url: &url::Url, _limit: u64) -> Result<NpmResponse, ModulesError> {
            self.0
                .lock()
                .expect("response lock")
                .pop_front()
                .ok_or_else(|| ModulesError::NpmTransport {
                    detail: "fixture exhausted".to_owned(),
                })?
        }
    }

    struct NoGit;

    impl GitResolver for NoGit {
        fn resolve(
            &self,
            _url: &str,
            _revision: Revision<'_>,
            _subdir: Option<&str>,
        ) -> Result<crate::git::ResolvedGitSource, ModulesError> {
            Err(ModulesError::UnsupportedSource {
                source_type: "git fixture",
            })
        }
    }

    struct RejectBadPackage;

    impl SemanticGate for RejectBadPackage {
        fn inspect(&self, _name: &ModuleName, root: &Path) -> SemanticVerdict {
            match fs::read_to_string(root.join("manifest.yaml")) {
                Ok(manifest) if manifest.contains("marker: bad") => SemanticVerdict {
                    diagnostics: vec![Diagnostic {
                        severity: Severity::Error,
                        rule: "TEST".to_owned(),
                        message: "fixture rejects bad package".to_owned(),
                    }],
                },
                _ => SemanticVerdict::default(),
            }
        }
    }

    fn package_archive(marker: &str) -> Vec<u8> {
        package_archive_for("demo", marker)
    }

    fn package_archive_for(module_name: &str, marker: &str) -> Vec<u8> {
        let content = format!("name: {module_name}\nmarker: {marker}\n");
        let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
        let mut archive = tar::Builder::new(&mut encoder);
        let mut header = tar::Header::new_gnu();
        header.set_size(u64::try_from(content.len()).expect("fixture size"));
        header.set_mode(0o644);
        header.set_cksum();
        archive
            .append_data(
                &mut header,
                "package/manifest.yaml",
                Cursor::new(content.into_bytes()),
            )
            .expect("append manifest");
        archive.finish().expect("finish tar");
        drop(archive);
        encoder.finish().expect("finish gzip")
    }

    fn npm_responses(version: &str, marker: &str) -> [NpmResponse; 2] {
        let tarball = format!("https://registry.npmjs.org/demo/-/demo-{version}.tgz");
        [
            NpmResponse {
                final_url: url::Url::parse(&format!("https://registry.npmjs.org/demo/{version}"))
                    .expect("metadata URL"),
                body: serde_json::to_vec(&serde_json::json!({
                    "version": version,
                    "dist": { "tarball": tarball }
                }))
                .expect("metadata JSON"),
            },
            NpmResponse {
                final_url: url::Url::parse(&tarball).expect("tarball URL"),
                body: package_archive(marker),
            },
        ]
    }

    struct RecordingNpmTransport {
        responses: Mutex<VecDeque<NpmResponse>>,
        requests: Mutex<Vec<url::Url>>,
    }

    impl NpmTransport for RecordingNpmTransport {
        fn get(&self, url: &url::Url, _limit: u64) -> Result<NpmResponse, ModulesError> {
            self.requests
                .lock()
                .expect("request lock")
                .push(url.clone());
            self.responses
                .lock()
                .expect("response lock")
                .pop_front()
                .ok_or_else(|| ModulesError::NpmTransport {
                    detail: "fixture exhausted".to_owned(),
                })
        }
    }

    #[trace("FR-187-AC-1", "FR-187-AC-12", "FR-187-AC-13")]
    #[test]
    fn tc_1755_001_npm_source_unsets_version_and_rejected_update_restores_prior_install() {
        let temp = tempfile::tempdir().expect("home tempdir");
        let paths = InstallPaths::for_home(&crate::paths::IxHome::new(temp.path().join("ixhome")));
        let mut responses = VecDeque::new();
        responses.extend(npm_responses("1.0.0", "good").map(Ok));
        responses.extend(npm_responses("2.0.0", "bad").map(Ok));
        responses.push_back(Err(ModulesError::NpmTimeout));
        let transport = FixtureNpmTransport(Mutex::new(responses));
        let installer = ModuleInstaller::with_npm_transport(
            paths.clone(),
            &NoGit,
            &transport,
            &RejectBadPackage,
            &crate::pack::NoPacks,
        );
        let source = |version: &str| Source::Npm {
            package: "demo".to_owned(),
            version: Some(version.to_owned()),
            registry: None,
        };

        let first = installer
            .install_ad_hoc(&source("1.0.0"))
            .expect("first package install");
        assert_eq!(
            first.module.source,
            Source::Npm {
                package: "demo".to_owned(),
                version: None,
                registry: None,
            }
        );
        assert_eq!(
            fs::read_to_string(paths.target_root.join("demo/manifest.yaml"))
                .expect("installed package"),
            "name: demo\nmarker: good\n"
        );

        let error = installer
            .install_ad_hoc(&source("2.0.0"))
            .expect_err("bad package must fail semantic validation");
        assert!(matches!(
            error,
            ModulesError::SemanticContractViolation {
                rollback: RollbackOutcome::Restored,
                ..
            }
        ));
        let installed_modules = installer.list().expect("registry after rollback");
        assert_eq!(installed_modules.len(), 1);
        assert_eq!(installed_modules[0].source, first.module.source);
        assert_eq!(
            fs::read_to_string(paths.target_root.join("demo/manifest.yaml"))
                .expect("restored package"),
            "name: demo\nmarker: good\n"
        );

        let error = installer
            .install_ad_hoc(&source("3.0.0"))
            .expect_err("timed out metadata request must fail");
        assert!(matches!(error, ModulesError::NpmTimeout));
        assert_eq!(
            installer.list().expect("registry after timeout"),
            installed_modules
        );
        assert_eq!(
            fs::read_to_string(paths.target_root.join("demo/manifest.yaml"))
                .expect("module after timeout"),
            "name: demo\nmarker: good\n"
        );
    }

    #[trace("FR-187-AC-3")]
    #[test]
    fn tc_1755_002_scoped_package_source_installs_with_scoped_name_and_unset_version() {
        let temp = tempfile::tempdir().expect("home tempdir");
        let paths = InstallPaths::for_home(&crate::paths::IxHome::new(temp.path().join("ixhome")));
        let metadata_url = "https://registry.npmjs.org/@scope%2Fname/1.2.3";
        let tarball_url = "https://registry.npmjs.org/@scope/name/-/name-1.2.3.tgz";
        let metadata = serde_json::to_vec(&serde_json::json!({
            "version": "1.2.3",
            "dist": { "tarball": tarball_url }
        }))
        .expect("scoped package metadata");
        let transport = RecordingNpmTransport {
            responses: Mutex::new(VecDeque::from([
                NpmResponse {
                    final_url: url::Url::parse(metadata_url).expect("metadata response URL"),
                    body: metadata,
                },
                NpmResponse {
                    final_url: url::Url::parse(tarball_url).expect("tarball response URL"),
                    body: package_archive_for("spec-objects-security", "scoped"),
                },
            ])),
            requests: Mutex::new(Vec::new()),
        };
        let installer = ModuleInstaller::with_npm_transport(
            paths.clone(),
            &NoGit,
            &transport,
            &crate::semantic::PermissiveGate,
            &crate::pack::NoPacks,
        );
        let source = Source::Npm {
            package: "@scope/name".to_owned(),
            version: Some("1.2.3".to_owned()),
            registry: None,
        };

        let result = installer
            .install_ad_hoc(&source)
            .expect("scoped package install");
        assert_eq!(
            transport
                .requests
                .lock()
                .expect("request lock")
                .as_slice()
                .iter()
                .map(url::Url::as_str)
                .collect::<Vec<_>>(),
            [metadata_url, tarball_url]
        );
        assert_eq!(
            result.module.source,
            Source::Npm {
                package: "@scope/name".to_owned(),
                version: None,
                registry: None,
            }
        );
        assert_eq!(
            installer.list().expect("installed registry")[0].source,
            result.module.source
        );
        assert_eq!(
            fs::read_to_string(
                paths
                    .target_root
                    .join("spec-objects-security/manifest.yaml")
            )
            .expect("installed manifest"),
            "name: spec-objects-security\nmarker: scoped\n"
        );
    }

    #[trace("FR-019-AC-2")]
    #[test]
    fn tc_381_260_join_checked_refuses_escapes() {
        let base = Path::new("/base");
        assert_eq!(
            join_checked(base, "pkg/sub").expect("a relative subpath is fine"),
            PathBuf::from("/base/pkg/sub")
        );
        for bad in ["/etc", "../x", "a/../../x", "a//b"] {
            assert!(join_checked(base, bad).is_err(), "{bad} must be refused");
        }
    }

    #[trace("FR-019-AC-2")]
    #[test]
    fn tc_381_261_rfc3339_now_is_a_sortable_instant() {
        let stamp = rfc3339_now();
        assert!(stamp.ends_with('Z'), "{stamp}");
        // `2026-01-02T03:04:05.678Z` — the exact shape `toISOString()` emits,
        // millisecond digits included.
        assert_eq!(stamp.len(), 24, "{stamp}");
        assert_eq!(stamp.as_bytes().get(19), Some(&b'.'), "{stamp}");
        assert_eq!(civil_from_days(0), (1970, 1, 1));
        assert_eq!(civil_from_days(19_000), (2022, 1, 8));
    }
}
