// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! NFR-003, class `malformed-input`: every flag whose help says it reads
//! stdin, on a command that requires nothing else, given malformed JSON as a
//! file and on stdin, fails saying the input is not JSON and where it went
//! wrong. A command with another required argument is outside the population:
//! a placeholder for that argument is read first and decides the outcome.

use std::process::Stdio;

use super::context::Context;
use super::failure::{Failure, judge, names};

pub(crate) fn probe(cx: &Context) -> Vec<Failure> {
    let malformed = cx.file("malformed.json", "{not json");
    let path = malformed.to_str().expect("a UTF-8 path");
    let mut failures = Vec::new();
    for command in cx.commands() {
        let targets = command.targets(
            |flag| flag.description.to_lowercase().contains("stdin"),
            |_| false,
        );
        for target in targets.iter().filter(|t| command.requires_only(t)) {
            for (source, value) in [("file", path), ("stdin", "-")] {
                let line = command.line(target, value);
                let line: Vec<&str> = line.iter().map(String::as_str).collect();
                let output = cx.run_with(command, &line, |process| {
                    if source == "stdin" {
                        process.stdin(Stdio::from(
                            std::fs::File::open(&malformed).expect("the document is readable"),
                        ));
                    }
                });
                failures.push(judge(
                    format!("{} {}:malformed-{source}", command.id(), target.label),
                    &output,
                    |message| {
                        (names(message, "json")
                            || names(message, target.label.trim_start_matches('-')))
                            && ["line", "column", "byte"]
                                .iter()
                                .any(|at| names(message, at))
                    },
                ));
            }
        }
    }
    failures
}
