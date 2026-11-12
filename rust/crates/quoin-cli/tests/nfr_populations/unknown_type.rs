// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! NFR-003, class `unknown-type`: every flag or positional the help names
//! `type` or `types`, given a type the catalog does not have, names it and
//! lists the types that are available.

use super::context::{Context, PROBE_TYPES};
use super::failure::{Failure, judge, names};

const TYPE: &str = "NoSuchTypeNfr003";

fn is_type(name: &str) -> bool {
    matches!(
        name.trim_start_matches('-').to_lowercase().as_str(),
        "type" | "types"
    )
}

pub(crate) fn probe(cx: &Context) -> Vec<Failure> {
    let mut failures = Vec::new();
    for command in cx.commands() {
        let targets = command.targets(
            |flag| is_type(&flag.name),
            |argument| is_type(&argument.name),
        );
        for target in &targets {
            let line = command.line(target, TYPE);
            let line: Vec<&str> = line.iter().map(String::as_str).collect();
            let output = cx.run(command, &line);
            failures.push(judge(
                format!("{} {}:unknown-type", command.id(), target.label),
                &output,
                |message| names(message, TYPE) && PROBE_TYPES.iter().all(|ty| names(message, ty)),
            ));
        }
    }
    failures
}
