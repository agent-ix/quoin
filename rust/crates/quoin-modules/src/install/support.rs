// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Filesystem and registry helpers shared by module installation.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use ix_trace_rs::implements;
use tempfile::Builder;

use crate::error::ModulesError;
use crate::install::MATERIALIZED_MARKER;
use crate::pack::RestrictedSet;
use crate::source::Source;

/// The sets as `authority/id@version`, comma separated.
pub(super) fn render_sets(sets: &[RestrictedSet]) -> String {
    sets.iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join(", ")
}

/// A source resolved to a directory, whatever its type.
pub(super) struct ResolvedSource {
    pub(super) dir: PathBuf,
    pub(super) requested_ref: Option<String>,
    pub(super) source: Source,
}

/// Join `sub` onto `base`, refusing anything that escapes it.
pub(super) fn join_checked(base: &Path, sub: &str) -> Result<PathBuf, ModulesError> {
    if sub.starts_with('/') || sub.split(['/', '\\']).any(|s| s == ".." || s.is_empty()) {
        return Err(ModulesError::UnsafeTreePath {
            entry: sub.to_owned(),
        });
    }
    Ok(base.join(sub))
}

/// Replace `target` with a copy of `content_root`.
#[implements("FR-099-AC-11")]
pub(super) fn materialize(content_root: &Path, target: &Path) -> Result<(), ModulesError> {
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
    fs::write(staged.join(MATERIALIZED_MARKER), b"").map_err(|source| {
        ModulesError::MaterializeFailed {
            target: staged.join(MATERIALIZED_MARKER),
            source,
        }
    })?;

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

pub(super) fn remove_dir_if_present(path: &Path) -> Result<(), ModulesError> {
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
pub(super) fn copy_dir(from: &Path, to: &Path) -> Result<(), ModulesError> {
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
pub(super) fn rfc3339_now() -> String {
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
pub(super) fn civil_from_days(z: i64) -> (i64, u32, u32) {
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
#[allow(
    clippy::expect_used,
    clippy::panic,
    reason = "test assertions report the violated filesystem contract"
)]
mod tests {
    use ix_trace_rs::trace;
    use tempfile::tempdir;

    use super::{MATERIALIZED_MARKER, materialize};

    #[trace("FR-099-AC-11")]
    #[test]
    fn tc_1915_failed_materialization_does_not_publish_completion_marker() {
        let scratch = tempdir().expect("scratch directory is created");
        let missing_source = scratch.path().join("missing-source");
        let target = scratch.path().join("modules/module");

        let error = materialize(&missing_source, &target)
            .expect_err("an unreadable source tree cannot be materialized");

        assert!(matches!(
            error,
            crate::error::ModulesError::MaterializeFailed { .. }
        ));
        assert!(!target.exists(), "a failed copy leaves no target root");
        assert!(
            !target.join(MATERIALIZED_MARKER).exists(),
            "an incomplete target has no completion marker"
        );
    }
}
