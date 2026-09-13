// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! Canonical JSON: the value model, the strict reader, and the two canonical
//! writers.
//!
//! There are exactly two canonical serializations in Quoin and they are not
//! interchangeable:
//!
//! | writer | shape | what it is for |
//! |---|---|---|
//! | [`jcs::canonicalize_jcs`] | RFC 8785, compact, UTF-16 key order | the bytes a digest is taken over |
//! | [`pretty::canonical_json`] | 2-space indent, ECMAScript own-property order, trailing newline | the bytes `spec/evidence/**.json` holds |
//!
//! Using the pretty form where the JCS form belongs changes every digest;
//! using the JCS form where the pretty form belongs rewrites every store file.

/// The deepest array/object nesting this crate will read or write.
///
/// # Why a bound exists
///
/// The reader and both writers are recursive, and Rust does not grow the
/// stack. Without a bound, a document of 40,000 `[` ends the process with
/// `fatal runtime error: stack overflow` (SIGABRT, exit 134) — an abort, not a
/// refusal, on a path that reads untrusted evidence.
///
/// # Why this number, and the divergence it leaves
///
/// The TypeScript oracle has no explicit bound either; it refuses deep input
/// by exhausting V8's stack, so its threshold is an artefact of stack size and
/// frame layout rather than a stated limit. Measured on the pinned toolchain
/// (node 22.15.0, `oracle`-equivalent call path, three runs):
///
/// | oracle path | deepest accepted | first refused |
/// |---|---|---|
/// | `parseStrictJson` | 5,119 – 6,143 (varies by run) | 5,120 – 6,144 |
/// | `parseStrictJson` + `canonicalizeJcs` | 2,943 | 2,944 |
/// | `parseStrictJson` + `canonicalJson` | 2,573 | 2,574 |
///
/// A fixed number cannot equal a varying one, so this budget is set below
/// every measured threshold and the residual band is **declared rather than
/// hidden**: for documents nested deeper than 1,000 and no deeper than the
/// oracle's own threshold, the oracle accepts and this crate refuses with
/// [`StoreError::JsonNestingTooDeep`](crate::error::StoreError::JsonNestingTooDeep).
/// That is the safe direction of the two — this crate never accepts a document
/// the oracle refuses — and it is unreachable in retained evidence, whose
/// deepest store file nests in the low tens.
pub const MAX_NESTING_DEPTH: usize = 1_000;

pub mod escape;
pub mod jcs;
pub mod number;
pub mod order;
pub mod parse;
pub mod pretty;
pub mod value;
