// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The auditor's report vocabulary: [`Finding`], [`Severity`],
//! [`UnevaluatedCheck`] and [`AuditReport`] (quoin#384, quoin#385, quoin#383).
//!
//! # One home for four types, and why it is this crate
//!
//! quoin#384 created this crate holding a three-field *reader* `Finding`, on
//! the measured argument that the assurance view observes three of twelve
//! fields and declaring the other nine would put surface in the payload that
//! no byte comparison could check.
//!
//! quoin#385 added [`AuditReport`] and [`UnevaluatedCheck`] beside it, because
//! `src/graph-analysis/` is a **second** view of the auditor's output and it
//! reads more than `build_case` does. The ruling was recorded once: those
//! types live here rather than in the view crate that happens to need them
//! first. Two homes for one producer's shape is how two readers end up
//! disagreeing about it.
//!
//! quoin#383 ports the **producer** — `src/auditor/` and `src/advisor/` — into
//! `quoin-auditor`, in the same workspace. The producer writes all twelve. The
//! choice was therefore between a second `Finding` next to the producer, and
//! widening this one. Two declarations of one wire shape drift, and the drift
//! is exactly the silent kind this program exists to catch, so the type stayed
//! here and grew nine `Option` fields.
//!
//! Nothing either view does changed: a finding carrying only the three read
//! fields still serialises to those three keys, because every added field is
//! `skip_serializing_if = "Option::is_none"`.
//!
//! # Members nobody here declares are preserved, not dropped
//!
//! The graph view copies the auditor's findings and unevaluated checks
//! **verbatim** into its own report — `src/graph-analysis/input.ts` validates
//! the join surface with a zod `.passthrough()` and preserves the rest. A type
//! that dropped an undeclared member would change those bytes. So every type
//! in this crate keeps a flattened [`OtherMembers`] map even now that the
//! producer's own fields are declared: what a *future* producer adds is
//! carried, and still not declared. The original argument stands — a declared
//! field nothing observes is untested surface — and it is a different thing
//! from a member kept because its producer wrote it.
//!
//! # `severity` is held open, because the retained data is wider than the type
//!
//! `src/auditor/audit.ts:34` types it `"low" | "medium" | "high"`, and the
//! obvious port is a three-variant enum. PR #492 wrote exactly that for the
//! graph-analysis port and the workspace gate then refused a finding
//! `build_case` has always accepted, because `quoin-assurance`'s captured
//! corpus carries `severity: "error"`. A TypeScript union is erased before any
//! value reaches the wire. Where the retained data is wider than the declared
//! type, the retained data wins — so [`Severity`] is an open newtype and
//! [`Finding::severity`] is an [`Option`]. See the doc on [`Severity`] for the
//! ten spellings actually retained.
//!
//! # What this crate does NOT own
//!
//! The checks. `quoin-auditor` owns every rule that decides *whether* a
//! finding exists; this crate owns only what one looks like once it does.

#![forbid(unsafe_code)]

use std::collections::BTreeMap;

mod finding;
mod report;
mod severity;

pub use finding::{Finding, FindingKind};
pub use report::{AuditReport, UnevaluatedCheck};
pub use severity::Severity;

/// Members the producer wrote that no type here declares.
///
/// A `BTreeMap` rather than a `serde_json::Map`: the map is only ever written
/// back out through a canonical serializer, so a stable order here costs
/// nothing and makes equality on these types independent of input order.
pub type OtherMembers = BTreeMap<String, serde_json::Value>;
