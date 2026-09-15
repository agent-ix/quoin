// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The quoin engine boundary (quoin#373 FR-096, Stage 0 = quoin#375).
//!
//! The pure operation protocol is callable in-process through [`runtime`].
//! The temporary `quoin-core` executable retains its one-operation JSON
//! protocol only as a wire-parity oracle until the final deletion cutover.
//!
//! Stage 0 fixed the shape everything else lands in: the exit taxonomy in
//! [`protocol`], the one error envelope in [`error`], and the operation table
//! in [`dispatch`]. Stage 9 reuses the same operation table without a second
//! Rust process or JSON reparse.
//!
//! The library half exists so the contract is unit-testable without spawning a
//! process; `src/main.rs` is the thin I/O shell over it and
//! `tests/tc_boundary.rs` proves the two agree.

pub mod capabilities;
pub mod dispatch;
pub mod error;
pub mod ops;
pub mod protocol;
/// Production capability construction, shared by the temporary protocol shell
/// and the final native CLI during Stage 9.
pub mod runtime;
