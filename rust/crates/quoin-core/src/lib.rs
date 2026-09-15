// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The quoin engine boundary (quoin#373 FR-096, Stage 0 = quoin#375).
//!
//! The pure operation protocol is callable in-process through [`runtime`].
//! The shipped `quoin` executable is its sole production caller.
//!
//! Stage 0 fixed the shape everything else lands in: the exit taxonomy in
//! [`protocol`], the one error envelope in [`error`], and the operation table
//! in [`dispatch`]. Stage 9 reuses the same operation table without a second
//! Rust process or JSON reparse.
//!
//! The library is unit-testable without spawning a process; `quoin-cli` owns
//! the user-facing grammar, I/O, and executable delivery.

pub mod capabilities;
pub mod dispatch;
pub mod error;
pub mod ops;
pub mod protocol;
/// Production capability construction for the native CLI.
pub mod runtime;
