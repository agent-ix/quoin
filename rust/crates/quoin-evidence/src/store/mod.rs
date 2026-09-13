// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Reading and writing the evidence store (FR-030).
//!
//! Every function here takes an [`EvidenceSource`](crate::source::EvidenceSource)
//! rather than a repository path. The retained `store.ts` takes a `repo: string`
//! and calls `node:fs` inline, which is why nothing in it can be tested without
//! a temporary directory and why the library half named a host capability its
//! callers could not substitute.
//!
//! The canonical serialization and the atomic write live in `quoin-store`:
//! change assurance writes its own family beneath the same root, and reaching
//! into evidence for them was one half of the import cycle
//! agent-ix/quoin#376 closed.

pub(crate) mod codec;
mod gc;
mod graph;
mod records;
mod trust_store;

pub use gc::{COLLECTED_FAMILIES, gc};
pub use graph::{
    AffirmOutcome, BindOutcome, affirm, bind, read_baseline, read_bindings, write_baseline,
    write_bindings,
};
pub use records::{list_recorded_suites, scan_is_vacuous};
pub use trust_store::{read_trust_decision, read_trust_decisions, write_trust_decision};

use crate::error::EvidenceError;
use crate::ids::{Commit, SuiteId};
use crate::source::EvidenceSource;
use crate::types::{FindingRecord, MockInspectionRecord, RunRecord};

macro_rules! family {
    (
        $record:ty,
        $write:ident, $read:ident, $list:ident, $read_all:ident, $latest:ident, $latest_each:ident,
        $what:literal
    ) => {
        #[doc = concat!("Write one ", $what, ". Last-write-wins at the same `(suite, commit)`.")]
        ///
        /// # Errors
        ///
        /// [`EvidenceError::Canonicalization`] or [`EvidenceError::StoreIo`].
        pub fn $write<S: EvidenceSource + ?Sized>(
            source: &mut S,
            record: &$record,
        ) -> Result<String, EvidenceError> {
            records::write_record(source, record)
        }

        #[doc = concat!("Read one ", $what, ", or `None` when that `(suite, commit)` has none.")]
        ///
        /// # Errors
        ///
        /// [`EvidenceError::StoreRead`] when the file is present and unparseable.
        pub fn $read<S: EvidenceSource + ?Sized>(
            source: &S,
            suite: &SuiteId,
            commit: &Commit,
        ) -> Result<Option<$record>, EvidenceError> {
            records::read_record(source, suite, commit)
        }

        #[doc = concat!("Every ", $what, " file a suite has, in file-name order.")]
        ///
        /// File-name order is not time order.
        ///
        /// # Errors
        ///
        /// [`EvidenceError::StoreIo`] when the directory cannot be listed.
        pub fn $list<S: EvidenceSource + ?Sized>(
            source: &S,
            suite: &SuiteId,
        ) -> Result<Vec<String>, EvidenceError> {
            records::list::<S, $record>(source, suite)
        }

        #[doc = concat!("Every ", $what, " a suite has, oldest first by timestamp.")]
        ///
        /// # Errors
        ///
        /// [`EvidenceError::StoreIo`] when the store cannot be read.
        pub fn $read_all<S: EvidenceSource + ?Sized>(
            source: &S,
            suite: &SuiteId,
            skipped: &mut Vec<String>,
        ) -> Result<Vec<$record>, EvidenceError> {
            records::read_all(source, suite, skipped)
        }

        #[doc = concat!("The newest ", $what, " of a suite by timestamp.")]
        ///
        /// # Errors
        ///
        /// As the reader above.
        pub fn $latest<S: EvidenceSource + ?Sized>(
            source: &S,
            suite: &SuiteId,
            skipped: &mut Vec<String>,
        ) -> Result<Option<$record>, EvidenceError> {
            records::latest(source, suite, skipped)
        }

        #[doc = concat!("The newest ", $what, " of every recorded suite that has one.")]
        ///
        /// # Errors
        ///
        /// As the reader above.
        pub fn $latest_each<S: EvidenceSource + ?Sized>(
            source: &S,
            skipped: &mut Vec<String>,
        ) -> Result<Vec<$record>, EvidenceError> {
            records::latest_each(source, skipped)
        }
    };
}

family!(
    RunRecord,
    write_run,
    read_run,
    list_runs,
    read_runs,
    latest_run,
    latest_runs,
    "run record"
);
family!(
    FindingRecord,
    write_scan,
    read_scan,
    list_scans,
    read_scans,
    latest_scan,
    latest_scans,
    "finding-shaped scan"
);
family!(
    MockInspectionRecord,
    write_mock_inspection,
    read_mock_inspection,
    list_mock_inspections,
    read_mock_inspections,
    latest_mock_inspection,
    latest_mock_inspections,
    "mock inspection"
);
