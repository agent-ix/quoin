// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Shared SOUP report assembly and renderings (FR-144, FR-197).

mod soup;

pub use soup::report::snapshot::{
    Disagreement, Group, NotAssessed, Origin, Resolved, SoupFailure, SoupInputs, SoupRow,
    SoupSection, soup_section,
};
pub use soup::{ABOUT, command, run};

pub use soup::report::{DossierRow, DossierSource, dossier_source};
