// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! The measurement domain's types, one module per family.
//!
//! `src/measurement/types.ts` is one 99-line file holding five unrelated
//! families. In Rust each family carries its wire-spelling enums, its
//! `from_wire`/`as_str` round trip and its own tests, so the families are split
//! here rather than transliterated into one module (Stage 6 plan §12).

pub mod collection;
pub mod comparison;
pub mod ids;
pub mod observation;
pub mod plan;
pub mod profile;
