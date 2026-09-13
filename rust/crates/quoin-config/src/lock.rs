// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The advisory config write lock (quoin#381, FR-027).
//!
//! `@agent-ix/ix-cli-core` guards its config read-merge-write with an advisory
//! sentinel at `<config file>.lock`, and during coexistence both
//! implementations write the same `~/.config/ix/config.d/<id>.yaml`. A writer
//! that skips the sentinel is not merely unsynchronised with itself — it races
//! the TypeScript writer, and a read-merge-write pair that interleaves loses
//! whichever key the loser merged.
//!
//! The protocol is therefore copied exactly, because only an identical protocol
//! interoperates:
//!
//! * claim by creating `<path>.lock` exclusively, mode `0o600`, containing
//!   `<pid>\n`;
//! * on `EEXIST`, retry every [`RETRY`] until [`TIMEOUT`] elapses;
//! * reclaim a sentinel older than [`STALE_AFTER`] whose recorded pid is gone
//!   or unreadable, so a killed writer does not wedge the file forever;
//! * release by unlinking, ignoring failure — a lost unlink degrades to the
//!   stale-reclaim path rather than to a hang.

use std::fs;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use crate::error::ConfigError;

/// How long to wait for the sentinel before giving up.
const TIMEOUT: Duration = Duration::from_secs(5);
/// How long to sleep between claim attempts.
const RETRY: Duration = Duration::from_millis(50);
/// How old a sentinel must be before its owner is checked for liveness.
const STALE_AFTER: Duration = Duration::from_secs(30);

/// Run `body` while holding the advisory lock for `path`.
///
/// The lock is released whether `body` returns or fails.
///
/// # Errors
/// [`ConfigError::ConfigLockTimeout`] when the sentinel cannot be claimed
/// within [`TIMEOUT`], plus whatever `body` returns.
pub(crate) fn with_config_lock<T>(
    path: &Path,
    body: impl FnOnce() -> Result<T, ConfigError>,
) -> Result<T, ConfigError> {
    let lock_path = lock_path_for(path);
    acquire(&lock_path)?;
    let outcome = body();
    // A failed release is deliberately not an error: the sentinel is advisory,
    // and the stale-reclaim path already covers a sentinel nobody removed.
    drop(fs::remove_file(&lock_path));
    outcome
}

/// `<path>.lock`, the sentinel ix-cli-core uses for the same file.
fn lock_path_for(path: &Path) -> PathBuf {
    let mut name = path.as_os_str().to_os_string();
    name.push(".lock");
    PathBuf::from(name)
}

/// Claim the sentinel, waiting and reclaiming as the protocol allows.
fn acquire(lock_path: &Path) -> Result<(), ConfigError> {
    if try_claim(lock_path) {
        return Ok(());
    }
    let deadline = Instant::now() + TIMEOUT;
    loop {
        if reclaim_if_stale(lock_path) && try_claim(lock_path) {
            return Ok(());
        }
        if Instant::now() >= deadline {
            return Err(ConfigError::ConfigLockTimeout {
                lock_path: lock_path.to_path_buf(),
                timeout_ms: u64::try_from(TIMEOUT.as_millis()).unwrap_or(u64::MAX),
            });
        }
        std::thread::sleep(RETRY);
        if try_claim(lock_path) {
            return Ok(());
        }
    }
}

/// One exclusive-create attempt. `false` means someone else holds it.
fn try_claim(lock_path: &Path) -> bool {
    if let Some(parent) = lock_path.parent() {
        drop(fs::create_dir_all(parent));
    }
    let mut options = fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt as _;
        options.mode(0o600);
    }
    match options.open(lock_path) {
        Ok(mut file) => {
            drop(writeln!(file, "{}", std::process::id()));
            true
        }
        Err(_) => false,
    }
}

/// Remove a sentinel whose owner is provably gone. `true` if it is now free.
fn reclaim_if_stale(lock_path: &Path) -> bool {
    let Ok(meta) = fs::metadata(lock_path) else {
        // Gone between the claim and this check: free.
        return true;
    };
    let recent = meta
        .modified()
        .ok()
        .and_then(|m| m.elapsed().ok())
        .is_some_and(|age| age < STALE_AFTER);
    if recent {
        return false;
    }
    let owner = fs::read_to_string(lock_path)
        .ok()
        .and_then(|t| t.trim().parse::<i32>().ok())
        .filter(|pid| *pid > 0);
    match owner {
        Some(pid) if process_is_alive(pid) => false,
        _ => fs::remove_file(lock_path).is_ok(),
    }
}

/// Whether `pid` names a live process.
///
/// Unknown answers say "alive": a sentinel is only reclaimed on proof that its
/// owner is gone, never on the absence of proof that it is running.
#[cfg(unix)]
fn process_is_alive(pid: i32) -> bool {
    fs::metadata(format!("/proc/{pid}")).is_ok()
}

/// Fallback liveness check where `/proc` does not exist.
#[cfg(not(unix))]
fn process_is_alive(_pid: i32) -> bool {
    true
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

    use super::*;

    /// Trace: FR-027-AC-8
    #[test]
    fn tc_381_050_the_lock_is_the_sentinel_ix_cli_core_uses() {
        assert_eq!(
            lock_path_for(Path::new("/c/config.d/quoin.yaml")),
            PathBuf::from("/c/config.d/quoin.yaml.lock")
        );
    }

    /// Trace: FR-027-AC-8
    #[test]
    fn tc_381_051_the_lock_is_held_during_the_body_and_released_after() {
        let dir = tempfile::tempdir().expect("temp dir");
        let target = dir.path().join("config.d/quoin.yaml");
        let lock = lock_path_for(&target);
        with_config_lock(&target, || {
            assert!(lock.exists(), "the sentinel exists while the body runs");
            Ok(())
        })
        .expect("uncontended lock");
        assert!(!lock.exists(), "the sentinel is released afterwards");
    }

    /// Trace: FR-027-AC-8
    #[test]
    fn tc_381_052_a_held_lock_times_out_rather_than_corrupting_the_file() {
        let dir = tempfile::tempdir().expect("temp dir");
        let target = dir.path().join("quoin.yaml");
        let lock = lock_path_for(&target);
        fs::create_dir_all(dir.path()).unwrap();
        // A fresh sentinel owned by this very much alive process: neither
        // claimable nor reclaimable.
        fs::write(&lock, format!("{}\n", std::process::id())).unwrap();

        let err = acquire(&lock).expect_err("a held lock is not claimable");
        assert_eq!(err.code(), crate::error::ConfigErrorCode::ConfigLockTimeout);
    }

    /// Trace: FR-027-AC-8
    #[test]
    fn tc_381_053_a_sentinel_with_no_live_owner_is_reclaimed() {
        let dir = tempfile::tempdir().expect("temp dir");
        let lock = dir.path().join("quoin.yaml.lock");
        // pid 0 is never a live process id, so the sentinel has no owner; the
        // age gate is bypassed by backdating the file's mtime.
        fs::write(&lock, "0\n").unwrap();
        let old = std::time::SystemTime::now() - Duration::from_secs(600);
        fs::File::options()
            .write(true)
            .open(&lock)
            .unwrap()
            .set_modified(old)
            .unwrap();
        assert!(
            reclaim_if_stale(&lock),
            "a dead owner's sentinel is removed"
        );
        assert!(!lock.exists());
    }
}
