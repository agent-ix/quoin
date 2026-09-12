// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The quoin engine boundary (quoin#373 FR-096, Stage 0 = quoin#375).
//!
//! One subprocess, one operation per invocation, JSON on stdin, JSON on
//! stdout. The boundary is a *subprocess*, not napi-rs and not WASM, so a
//! failure is an exit status a caller can reason about rather than a segfault
//! in the Node process.
//!
//! Stage 0 ports no domain logic. What it fixes is the shape everything else
//! lands in: the exit taxonomy in [`protocol`], the one error envelope in
//! [`error`], the operation table in [`dispatch`], and the caller on the other
//! side of the pipe in `src/core/exec.ts`.
//!
//! The library half exists so the contract is unit-testable without spawning a
//! process; `src/main.rs` is the thin I/O shell over it and
//! `tests/tc_boundary.rs` proves the two agree.

pub mod dispatch;
pub mod error;
pub mod ops;
pub mod protocol;
