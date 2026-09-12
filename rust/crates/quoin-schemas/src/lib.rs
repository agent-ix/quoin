// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The schema-sourced type surface of the quoin boundary (FR-097).
//!
//! Deliberately empty at Stage 0 (quoin#375). It exists so the place a
//! generated type lands is decided before the first one is generated, and so
//! the workspace gates — `clippy -D warnings`, `cargo deny`, `rustfmt` — are
//! already running over it when that happens.
//!
//! FR-097 says TypeScript never hand-writes a response type: `schemars` emits
//! JSON Schema from the types declared here, a build step generates
//! `src/core/types.ts`, and the result is hash-asserted. Nothing in this crate
//! may be hand-written on both sides of that boundary.
//!
//! # What must NOT be added here
//!
//! A type whose canonical definition lives in `filament-core-data`. fcd is the
//! multi-language schema source of truth (quoin#373, gate phase B / fcd#11);
//! re-declaring one of its types here would shadow it, which is the defect
//! this crate exists to prevent rather than to commit.

/// The IPC protocol revision the boundary speaks.
///
/// Lives here rather than in `quoin-core` because both the Rust side and the
/// generated TypeScript side must read the same number, and the generated side
/// is generated from this crate.
pub const PROTOCOL_VERSION: u32 = 1;
