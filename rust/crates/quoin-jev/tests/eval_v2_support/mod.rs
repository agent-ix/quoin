// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The corpus-v2 experiment harness (PLAT-1027, parent PLAT-1024).
//!
//! PLAT-917, PLAT-839 and PLAT-1014 asked each Jev question one way, on a
//! 29-triple corpus. PLAT-1024 asks several ways (variants) per question on a
//! corpus of several hundred rows, and in four modes: requirement alone (`R`),
//! requirement plus test (`RT`), requirement plus code (`RC`), and the full
//! triple (`RTC`). This module is the machinery those experiments share. It
//! decides nothing about which wording wins.
//!
//! - [`corpus`] reads and validates `fixtures/eval-v2/corpus.json` and the
//!   optional external corpus, materializes by-reference rows from a local
//!   checkout, and holds the held-out seal.
//! - [`keys`] names every question key and its answer space.
//! - [`variant`] is the registry: a variant is a named, versioned question set
//!   plus a `derive` step from raw answers to graded answers. It also holds
//!   the per-unit fan-out and the runner.
//! - [`units`] cuts one named item out of a Rust or Python file, and splits a
//!   body into functions or branches for fan-out, by one rule in both.
//! - [`fixtures`] holds the synthetic rows the offline tests and the
//!   request-digest pins share.
//! - [`preflight`] is every check a run passes before its first request.
//! - [`patch`] applies a mutation's unified diff to a by-reference file.
//! - [`variants`] holds the experiment variants, one module per question:
//!   [`variants::exceeds`] is PLAT-1029's `code_exceeds_requirement` (MP-241).
//! - [`metrics`] grades, reusing `../support/grading.rs`, and renders the
//!   report broken down by mode and by truth kind.
//!
//! Evaluation only: nothing here is reachable from `quoin-jev`'s `src/`.

#![allow(
    dead_code,
    reason = "each test binary links this module separately, so items only one binary uses read as dead in the other"
)]

#[path = "../support/grading.rs"]
pub(crate) mod grading;

pub(crate) mod corpus;
pub(crate) mod fixtures;
pub(crate) mod keys;
pub(crate) mod metrics;
pub(crate) mod patch;
pub(crate) mod preflight;
pub(crate) mod units;
pub(crate) mod variant;
pub(crate) mod variants;
