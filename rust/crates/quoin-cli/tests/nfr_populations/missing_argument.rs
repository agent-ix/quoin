// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! NFR-003, class `missing-argument`: every command without subcommands whose help marks a flag
//! `(required)` or lists an unbracketed positional, run bare, names each
//! thing it lacks and points to usage.

use super::context::Context;
use super::failure::{Failure, judge, names, points_forward};

pub(crate) fn probe(cx: &Context) -> Vec<Failure> {
    cx.commands()
        .iter()
        .filter(|command| command.subcommands.is_empty() && !command.required_names().is_empty())
        .map(|command| {
            let output = cx.run(command, &[]);
            let required = command.required_names();
            judge(
                format!("{}:missing-argument", command.id()),
                &output,
                |message| {
                    required.iter().all(|name| names(message, name)) && points_forward(message)
                },
            )
        })
        .collect()
}
