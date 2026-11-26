// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

#![forbid(unsafe_code)]

//! Shared SOUP report assembly and renderings (FR-144, FR-197).

mod error;
mod soup;

pub use error::{SoupError, SoupErrorCode};

pub use soup::report::snapshot::{
    Disagreement, Group, NotAssessed, Origin, Resolved, SoupFailure, SoupInputs, SoupRow,
    SoupSection, soup_section,
};
pub use soup::{ABOUT, command, run};

pub use soup::report::{DossierRow, DossierSource, dossier_source};
