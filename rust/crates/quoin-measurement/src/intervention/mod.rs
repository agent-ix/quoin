// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! Intervention experiments: the record, its refusal vocabulary and its report.
//!
//! What is here is quoin#469's half — the type declarations of
//! `intervention-types.ts` and the pure projection and renderer of
//! `intervention-report.ts`. Intake, record-id encoding, validation and the
//! agent-eval producer are `intervention.ts` and land with quoin#471.

pub mod agent_eval;
pub mod intake;
pub mod record;
pub mod report;
