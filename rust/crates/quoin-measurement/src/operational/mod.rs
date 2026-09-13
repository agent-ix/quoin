// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! Operational control evidence: the record shapes and their report.
//!
//! | retained TypeScript | here |
//! | --- | --- |
//! | `operational-types.ts` | [`record`], [`github_release`] |
//! | `operational-report.ts` | [`report`] |
//! | `operational.ts:42-48` | [`paths`] |
//! | `operational.ts:50-68`, `:320-364` | [`validate`], `semantics` |
//! | `operational.ts:70-125`, `:232-294` | [`intake`] |
//! | `operational.ts:127-172` | [`read`] |
//! | `operational.ts:174-230` | [`discharge`] |
//! | `operational.ts:296-318` | [`lock`] |
//! | `operational.ts:365-421`, `:471-489` | [`clock`] |
//! | `github-release-operational.ts` | [`github_release`] |
//!
//! # One file, nine modules
//!
//! `operational.ts` is 531 lines carrying nine unrelated responsibilities, and
//! a port that kept them in one file would land a 531-line module against a
//! 500-line soft ceiling with no room for the next wave. The seams above are
//! not line-count seams: nothing in [`lock`] knows what a record is, nothing in
//! [`clock`] touches a filesystem, and [`discharge`] is a pure question about
//! two values that intake never asks.

pub mod clock;
pub mod discharge;
pub mod github_release;
pub mod intake;
pub mod lock;
pub mod paths;
pub mod read;
pub mod record;
pub mod report;
mod semantics;
pub mod validate;
