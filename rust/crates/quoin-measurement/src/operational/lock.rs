// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! The store-wide operational write lock.
//!
//! Ports `operational.ts:296-318` — its own module, because the mutual
//! exclusion it provides is the whole of its contract and quoin#386 asks for
//! that contract to be **asserted** rather than described in a comment. Two
//! threads do the asserting in `tests/tc_472_operational_lock.rs`.
//!
//! # Why a directory
//!
//! `mkdir` is the one filesystem operation that is atomic and exclusive on
//! every host quoin runs on: it either creates the directory or fails with
//! `EEXIST`, with no window between the test and the creation. An
//! `existsSync`-then-create pair has that window and would let two intakes
//! into the store at once, which is the collision the lock exists to stop.
//!
//! # Why the deadline is injected
//!
//! The retained code spins against `Date.now() + 10_000`, so a test of the
//! refusal has to sleep ten seconds to reach it. Here the deadline is measured
//! on a [`Clock`] the caller passes, so the refusal is stated by the test
//! rather than waited for. [`crate::source::SystemClock`] is what production
//! passes, and it is the retained behaviour exactly: `Date.now()` and a 5 ms
//! wait between attempts.
//!
//! # Fail closed
//!
//! A timeout is a refusal, never a seizure. The retained message says so out
//! loud and is kept verbatim: a stale lock directory is removed by a person who
//! has confirmed no writer is running, not by the next writer to arrive.

use std::path::{Path, PathBuf};

use crate::intervention::intake::{InterventionIntakeError, InterventionRefusalCode};
use crate::source::Clock;

/// How long a writer waits for the lock before refusing.
///
/// `operational.ts:299`.
pub const LOCK_DEADLINE_MILLIS: i64 = 10_000;

/// How long a writer waits between two attempts.
///
/// `operational.ts:313`'s `Atomics.wait(…, 5)`.
pub const LOCK_RETRY_MILLIS: u32 = 5;

/// The lock directory's name inside the store root.
///
/// `operational.ts:297`.
pub const LOCK_DIRECTORY_NAME: &str = ".operational-write.lock";

/// A held store-wide operational write lock.
///
/// Holding this value *is* the exclusion: it is minted only by
/// [`OperationalWriteLock::acquire`], which creates the directory, and the
/// directory is removed when it drops — including when the guarded work panics,
/// which is the retained `finally`.
#[derive(Debug)]
pub struct OperationalWriteLock {
    directory: PathBuf,
}

impl OperationalWriteLock {
    /// Take the lock, waiting on `clock` until the deadline.
    ///
    /// # Errors
    ///
    /// [`InterventionRefusalCode::IntakeBusy`] when the lock is still held at
    /// the deadline, and the same code carrying the underlying failure when the
    /// store root or the lock directory cannot be created for any other reason.
    pub fn acquire(store_root: &Path, clock: &dyn Clock) -> Result<Self, InterventionIntakeError> {
        let directory = store_root.join(LOCK_DIRECTORY_NAME);
        std::fs::create_dir_all(store_root)
            .map_err(|error| busy(format!("{}: {error}", store_root.display())))?;
        let deadline = clock.now_millis().saturating_add(LOCK_DEADLINE_MILLIS);
        loop {
            match std::fs::create_dir(&directory) {
                Ok(()) => return Ok(Self { directory }),
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                    if clock.now_millis() >= deadline {
                        return Err(busy(format!(
                            "{}: timed out waiting for the store-wide operational intake lock; \
                             fail closed and remove it only after confirming no writer is active",
                            directory.display()
                        )));
                    }
                    clock.wait(LOCK_RETRY_MILLIS);
                }
                Err(error) => {
                    return Err(busy(format!("{}: {error}", directory.display())));
                }
            }
        }
    }

    /// The directory that is the lock.
    #[must_use]
    pub fn directory(&self) -> &Path {
        &self.directory
    }
}

impl Drop for OperationalWriteLock {
    fn drop(&mut self) {
        // `rmdirSync` in the retained `finally`. A failure here cannot be
        // reported — a `Drop` has nowhere to report to — and must not panic,
        // so the only honest thing is to leave the directory and let the next
        // writer refuse on the deadline, which is what fail-closed means.
        drop(std::fs::remove_dir(&self.directory));
    }
}

fn busy(finding: String) -> InterventionIntakeError {
    InterventionIntakeError::new(InterventionRefusalCode::IntakeBusy, vec![finding])
}

#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]
#[cfg(test)]
mod tests {
    use std::cell::Cell;

    use super::{LOCK_DEADLINE_MILLIS, OperationalWriteLock};
    use crate::intervention::intake::InterventionRefusalCode;
    use crate::source::Clock;

    /// A clock that never sleeps: every wait advances its own reading instead.
    struct VirtualClock {
        millis: Cell<i64>,
    }

    impl Clock for VirtualClock {
        fn now_millis(&self) -> i64 {
            self.millis.get()
        }

        fn wait(&self, millis: u32) {
            self.millis.set(self.millis.get() + i64::from(millis));
        }
    }

    #[test]
    fn a_second_writer_refuses_at_the_deadline_without_waiting_for_it() {
        let store = tempfile::tempdir().unwrap();
        let held = OperationalWriteLock::acquire(
            store.path(),
            &VirtualClock {
                millis: Cell::new(0),
            },
        )
        .unwrap();
        let clock = VirtualClock {
            millis: Cell::new(0),
        };
        let error = OperationalWriteLock::acquire(store.path(), &clock).unwrap_err();
        assert_eq!(error.code(), InterventionRefusalCode::IntakeBusy);
        assert!(
            error.findings()[0].contains("fail closed"),
            "the refusal must say a stale lock is a person's call: {error}"
        );
        // The refusal took the whole stated deadline of virtual time, and no
        // wall-clock time at all.
        assert!(clock.now_millis() >= LOCK_DEADLINE_MILLIS);
        drop(held);
    }

    #[test]
    fn the_directory_is_gone_once_the_guard_is_dropped() {
        let store = tempfile::tempdir().unwrap();
        let clock = VirtualClock {
            millis: Cell::new(0),
        };
        let directory = {
            let lock = OperationalWriteLock::acquire(store.path(), &clock).unwrap();
            assert!(lock.directory().is_dir());
            lock.directory().to_path_buf()
        };
        assert!(!directory.exists());
        // And the lock is takeable again, which is the point of the removal.
        drop(OperationalWriteLock::acquire(store.path(), &clock).unwrap());
    }
}
