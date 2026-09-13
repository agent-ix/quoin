// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! The store-wide operational write lock excludes, and two threads say so.
//!
//! # Why threads and not a comment
//!
//! `operational.ts:296-318` describes its mutual exclusion in prose and asserts
//! none of it; quoin#386 is open on exactly that. A lock whose exclusion is
//! only described is a lock whose exclusion is only hoped for, so here real
//! threads race for one store root on a real filesystem and the test fails if
//! two of them are ever inside at once.
//!
//! # Why the refusal test does not sleep
//!
//! The deadline is measured on an injected [`Clock`], so the ten-second
//! refusal is reached by advancing a counter rather than by waiting. A test
//! that slept for the deadline would be ten seconds of nothing on every run,
//! and the first thing anyone would do about that is delete it.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Barrier};

use quoin_measurement::intervention::intake::InterventionRefusalCode;
use quoin_measurement::operational::lock::{
    LOCK_DEADLINE_MILLIS, LOCK_DIRECTORY_NAME, OperationalWriteLock,
};
use quoin_measurement::source::{Clock, SystemClock};

/// How many threads race for the one lock.
const RACERS: usize = 2;

/// How many times each racer takes it.
const ROUNDS: usize = 20;

/// A clock that never sleeps: every wait advances its own reading instead.
struct VirtualClock {
    millis: std::cell::Cell<i64>,
}

impl Clock for VirtualClock {
    fn now_millis(&self) -> i64 {
        self.millis.get()
    }

    fn wait(&self, millis: u32) {
        self.millis.set(self.millis.get() + i64::from(millis));
    }
}

/// Trace: FR-100-AC-9
/// Provenance: quoin#472
#[test]
fn tc_472_two_threads_are_never_inside_the_lock_at_once() {
    let store = tempfile::tempdir().unwrap();
    let root = store.path().to_path_buf();
    let inside = Arc::new(AtomicUsize::new(0));
    let entered = Arc::new(AtomicUsize::new(0));
    let start = Arc::new(Barrier::new(RACERS));

    std::thread::scope(|scope| {
        for _ in 0..RACERS {
            let root = root.clone();
            let inside = Arc::clone(&inside);
            let entered = Arc::clone(&entered);
            let start = Arc::clone(&start);
            scope.spawn(move || {
                start.wait();
                for _ in 0..ROUNDS {
                    let lock = OperationalWriteLock::acquire(&root, &SystemClock)
                        .expect("the lock is reachable within its deadline");
                    let concurrent = inside.fetch_add(1, Ordering::SeqCst);
                    assert_eq!(
                        concurrent, 0,
                        "a second writer was inside the store-wide lock (quoin#386)"
                    );
                    entered.fetch_add(1, Ordering::SeqCst);
                    // Widen the window the exclusion has to survive: without
                    // the lock, this is long enough for the other thread to
                    // observe the count at 1.
                    std::thread::yield_now();
                    inside.fetch_sub(1, Ordering::SeqCst);
                    drop(lock);
                }
            });
        }
    });

    assert_eq!(
        entered.load(Ordering::SeqCst),
        RACERS * ROUNDS,
        "anti-vacuity floor: every racer must actually have taken the lock"
    );
    assert!(
        !root.join(LOCK_DIRECTORY_NAME).exists(),
        "the last writer left the lock directory behind"
    );
}

/// Trace: FR-100-AC-9
/// Provenance: quoin#472
#[test]
fn tc_472_a_second_writer_refuses_at_the_deadline_and_says_who_may_clear_it() {
    let store = tempfile::tempdir().unwrap();
    let held = OperationalWriteLock::acquire(store.path(), &SystemClock).unwrap();
    let clock = VirtualClock {
        millis: std::cell::Cell::new(0),
    };

    let error = OperationalWriteLock::acquire(store.path(), &clock).unwrap_err();

    assert_eq!(error.code(), InterventionRefusalCode::IntakeBusy);
    assert!(
        error.findings()[0].contains("remove it only after confirming no writer is active"),
        "a timeout is a refusal, not a seizure: {error}"
    );
    assert!(
        clock.now_millis() >= LOCK_DEADLINE_MILLIS,
        "the refusal must be reached at the stated deadline, not before it"
    );
    assert!(
        held.directory().is_dir(),
        "the refused writer must not have touched the held lock"
    );
}
