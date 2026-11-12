// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! NFR-003, class `unknown-flag`: every command the binary describes, given a
//! flag it does not have, names that flag and points to usage.

use super::context::Context;
use super::failure::{Failure, judge, names, points_forward};

const FLAG: &str = "--no-such-flag-for-nfr003";

pub(crate) fn probe(cx: &Context) -> Vec<Failure> {
    cx.commands()
        .iter()
        .map(|command| {
            let output = cx.run(command, &[FLAG]);
            judge(
                format!("{}:unknown-flag", command.id()),
                &output,
                |message| names(message, FLAG) && points_forward(message),
            )
        })
        .collect()
}
