// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Command adapter to the shared SOUP library (FR-141, FR-144).

pub(crate) use quoin_soup::{ABOUT, SoupFailure, SoupInputs, SoupSection};

pub(crate) fn command() -> clap::Command {
    quoin_soup::command()
}

pub(crate) fn run(arguments: &clap::ArgMatches) -> Result<quoin_core::protocol::Response, String> {
    quoin_soup::run(arguments, &crate::core_runtime::settings())
        .map_err(|error| error.message().to_owned())
}

pub(crate) fn soup_section(inputs: &SoupInputs<'_>) -> Result<SoupSection, SoupFailure> {
    quoin_soup::soup_section(inputs, &crate::core_runtime::settings())
}
