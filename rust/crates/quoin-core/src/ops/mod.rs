// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The operation table.
//!
//! The unit of IPC is a command-shaped operation (`<domain>.<op>`), never a
//! function (quoin#373). One module per domain; one entry per operation in
//! [`crate::dispatch`].

pub mod assurance;
pub mod core;
pub mod validators;
