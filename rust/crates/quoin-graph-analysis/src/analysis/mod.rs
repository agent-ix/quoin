// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! The three projections, and the head they share.
//!
//! Every function here is pure: it takes an in-memory
//! [`crate::GraphAnalysisInput`] and returns a report. The filesystem is
//! [`crate::load`]'s, and nothing in this module can reach it.
//!
//! # Where the ordering comes from
//!
//! The retained source sorts with `compare()` (`analysis.ts:717`), which is
//! JavaScript `<` on strings — UTF-16 code-unit order. The id newtypes take
//! their `Ord` from [`quoin_store::json::order::cmp_utf16`], so the
//! `BTreeMap`s and `BTreeSet`s below *are* those sorts: a set replaces
//! `new Set(...)` followed by `sorted(...)`, and a map replaces an
//! insertion-ordered `Map` followed by `.sort()`. Nothing is sorted twice and
//! nothing is left to Rust's `str: Ord`, which is scalar order and differs
//! above the BMP.

mod churn;
pub(crate) mod draft;
mod fan_out;
mod impact;
mod paths;

pub use churn::analyze_churn;
pub use fan_out::analyze_fan_out;
pub use impact::analyze_change_impact;
