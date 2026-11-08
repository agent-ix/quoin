// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The ways annotating a report can fail.

use std::path::PathBuf;

/// A refusal to annotate, each naming what to fix.
#[derive(Debug, thiserror::Error)]
pub(crate) enum Error {
    /// The command line was not understood.
    #[error("{0}")]
    Usage(String),
    /// A file could not be read or written.
    #[error("{path}: {reason}", path = .path.display())]
    Io {
        /// The file concerned.
        path: PathBuf,
        /// What the operating system said.
        reason: String,
    },
    /// `cargo metadata` failed or printed something that is not its JSON shape.
    #[error("cargo metadata: {0}")]
    Metadata(String),
    /// The quire engine could not produce the tag relations.
    #[error("trace tags: {0}")]
    Engine(String),
    /// No test case received a tag, so the run would bind nothing.
    #[error(
        "no test case in the report matched a trace-tagged test ({tagged} tagged tests, {cases} test cases); a report that binds nothing is not recorded"
    )]
    NothingMatched {
        /// Tagged tests the engine found.
        tagged: usize,
        /// Test cases in the report.
        cases: usize,
    },
}
