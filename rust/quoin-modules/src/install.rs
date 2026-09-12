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

use crate::error::{ModulesError, RollbackOutcome};
use crate::git::{GitResolver, Revision};
use crate::ids::ModuleName;
use crate::manifest::MarketplaceEntry;
use crate::module_name::read_module_name;
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
/// Both collaborators are constructor arguments rather than defaults: a
/// resolver so the network is a seam the tests can stand in for, and a gate
/// because [`PermissiveGate`](crate::semantic::PermissiveGate) accepting
/// everything must be a choice a caller made, not one it inherited.
pub struct ModuleInstaller<'a> {
    paths: InstallPaths,
    resolver: &'a dyn GitResolver,
    gate: &'a dyn SemanticGate,
}

impl<'a> ModuleInstaller<'a> {
    /// Build an installer.
    #[must_use]
    pub fn new(
        paths: InstallPaths,
        resolver: &'a dyn GitResolver,
        gate: &'a dyn SemanticGate,
    ) -> Self {
        Self {
            paths,
            resolver,
            gate,
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
        self.install_source(&entry.source, Some(&entry.name), entry.path.as_deref())
    }

    /// Install an ad-hoc source, deriving the module name from its manifest.
    ///
    /// # Errors
    /// As [`ModuleInstaller::install`].
    pub fn install_ad_hoc(&self, source: &Source) -> Result<InstallOutcome, ModulesError> {
        self.install_source(source, None, None)
    }

    fn install_source(
        &self,
        source: &Source,
        declared_name: Option<&ModuleName>,
        entry_subpath: Option<&str>,
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
            source: source.clone(),
            r#ref: resolved.requested_ref.clone(),
            sha: resolved.sha.clone(),
            resolved_path: content_root.to_string_lossy().into_owned(),
            target_path: target.to_string_lossy().into_owned(),
            installed_at: rfc3339_now(),
            semantic: None,
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

        let mut record = record;
        if let Some(pin) = verdict.pin {
            let mut registry = ModuleRegistry::read(&self.paths.registry_path)?;
            registry.pin_semantic(&name, pin.clone());
            registry.write(&self.paths.registry_path)?;
            record.semantic = Some(pin);
        }

        Ok(InstallOutcome {
            module: record,
            replaced_previous: previous.is_some(),
        })
    }

    /// Resolve a source to a content directory and a pin.
    fn resolve(&self, source: &Source) -> Result<ResolvedSource, ModulesError> {
        match source {
            Source::Path { path } => {
                let dir = PathBuf::from(path);
                let dir = dir
                    .canonicalize()
                    .map_err(|_| ModulesError::PathSourceNotFound { path: dir.clone() })?;
                Ok(ResolvedSource {
                    dir,
                    sha: None,
                    requested_ref: None,
                })
            }
            Source::Npm { .. } => Err(ModulesError::UnsupportedSource { source_type: "npm" }),
            Source::Url { .. } => Err(ModulesError::UnsupportedSource { source_type: "url" }),
            Source::Github { .. } | Source::GitSubdir { .. } | Source::Git { .. } => {
                let url = source.git_url().ok_or(ModulesError::UnsupportedSource {
                    source_type: source.type_name(),
                })?;
                let revision = Revision::of(source.pinned_sha(), source.requested_ref());
                let resolved = self.resolver.resolve(&url, revision, source.subdir())?;
                Ok(ResolvedSource {
                    dir: resolved.dir,
                    sha: Some(resolved.sha),
                    requested_ref: resolved.requested_ref,
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
            let resolved = self.resolve(&previous.source)?;
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
}

/// A source resolved to a directory, whatever its type.
struct ResolvedSource {
    dir: PathBuf,
    sha: Option<crate::ids::CommitSha>,
    requested_ref: Option<String>,
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
    remove_dir_if_present(target)?;
    copy_dir(content_root, target)
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
fn rfc3339_now() -> String {
    let secs = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs());
    let (days, rem) = (secs / 86_400, secs % 86_400);
    let (hour, minute, second) = (rem / 3600, (rem % 3600) / 60, rem % 60);
    let (year, month, day) = civil_from_days(i64::try_from(days).unwrap_or(0));
    format!("{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}Z")
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
    use super::*;

    /// Trace: FR-019-AC-2
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

    /// Trace: FR-019-AC-2
    #[test]
    fn tc_381_261_rfc3339_now_is_a_sortable_instant() {
        let stamp = rfc3339_now();
        assert!(stamp.ends_with('Z'), "{stamp}");
        assert_eq!(stamp.len(), 20, "{stamp}");
        assert_eq!(civil_from_days(0), (1970, 1, 1));
        assert_eq!(civil_from_days(19_000), (2022, 1, 8));
    }
}
