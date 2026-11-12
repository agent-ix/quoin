// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! NFR-003, class `missing-path`: every flag or positional whose help
//! describes a directory, a root or a JSON path, on a command that requires
//! nothing else, given a path that does not exist, fails naming that path and
//! what is wrong with it. A command with another required argument is outside
//! the population: a placeholder for that argument is read first and decides
//! the outcome.

use super::context::Context;
use super::failure::{Failure, judge, names};

/// What a diagnostic says about a path it could not use.
const REASONS: [&str; 7] = [
    "not a directory",
    "no such file",
    "not found",
    "does not exist",
    "not a readable",
    "cannot read",
    "could not",
];

/// Whether a help description says its value is a single filesystem location.
fn describes_location(description: &str) -> bool {
    let description = description.to_lowercase();
    ["directory", "root", "json path"]
        .iter()
        .any(|word| description.contains(word))
}

pub(crate) fn probe(cx: &Context) -> Vec<Failure> {
    let mut failures = Vec::new();
    for command in cx.commands().iter().filter(|c| !c.path.is_empty()) {
        let targets = command.targets(
            |flag| describes_location(&flag.description),
            |argument| describes_location(&argument.description),
        );
        for target in targets.iter().filter(|t| command.requires_only(t)) {
            let missing = cx.missing_path(&target.label);
            let missing = missing.to_str().expect("a UTF-8 path");
            let line = command.line(target, missing);
            let line: Vec<&str> = line.iter().map(String::as_str).collect();
            let output = cx.run(command, &line);
            failures.push(judge(
                format!("{} {}:missing-path", command.id(), target.label),
                &output,
                |message| {
                    names(message, missing) && REASONS.iter().any(|reason| names(message, reason))
                },
            ));
        }
    }
    failures
}
