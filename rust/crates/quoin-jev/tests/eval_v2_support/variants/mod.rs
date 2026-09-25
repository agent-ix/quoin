// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Experiment variants, one module per experiment ticket (PLAT-1024).
//!
//! The baselines stay in `../variant.rs`; each experiment's own wordings and
//! derive rules live here and are added to its `REGISTRY`.

pub(crate) mod battery;
pub(crate) mod exceeds;
pub(crate) mod intent;
pub(crate) mod refusal;
pub(crate) mod severity;
pub(crate) mod soundness;
pub(crate) mod statement;
