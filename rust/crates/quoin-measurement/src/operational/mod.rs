// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! Operational control evidence: the record shapes and their report.
//!
//! What is here is quoin#469's half — the type declarations of
//! `operational-types.ts` and the pure projection and renderer of
//! `operational-report.ts`. The pair writer, the store-wide directory lock and
//! the GitHub-release producer are `operational.ts` and land with quoin#472.

pub mod github_release;
pub mod record;
pub mod report;
