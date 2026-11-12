// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! NFR-003, class `unknown-subcommand`: the root and every command that lists
//! subcommands, given a word that names none, names that word and lists what
//! it could have been.

use super::context::Context;
use super::failure::{Failure, judge, names, points_forward};

const WORD: &str = "no-such-subcommand-nfr003";

pub(crate) fn probe(cx: &Context) -> Vec<Failure> {
    cx.commands()
        .iter()
        .filter(|command| command.path.is_empty() || !command.subcommands.is_empty())
        .map(|command| {
            let output = cx.run(command, &[WORD]);
            judge(
                format!("{}:unknown-subcommand", command.id()),
                &output,
                |message| names(message, WORD) && points_forward(message),
            )
        })
        .collect()
}
