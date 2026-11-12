// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! NFR-003, class `catalog-conflict`: every command whose help says it checks for
//! duplicate types, run over two modules that declare the same
//! type, fails naming the type and both modules.

use super::context::{Context, write_module};
use super::failure::{Failure, judge, names};

const TYPE: &str = "NfrDuplicate";
const MODULES: [&str; 2] = ["nfr003-first", "nfr003-second"];

pub(crate) fn probe(cx: &Context) -> Vec<Failure> {
    let scratch = tempfile::tempdir().expect("scratch is creatable");
    let roots = MODULES.map(|name| scratch.path().join(name));
    for (root, name) in roots.iter().zip(MODULES) {
        write_module(root, name, &[TYPE]);
    }
    let paths = std::env::join_paths(&roots).expect("paths join");
    cx.commands()
        .iter()
        .filter(|command| command.summary.to_lowercase().contains("duplicate"))
        .map(|command| {
            let output = cx.run_with(command, &[], |process| {
                process.env("QUOIN_MODULE_PATHS", &paths);
            });
            judge(
                format!("{}:duplicate-type", command.id()),
                &output,
                |report| names(report, TYPE) && MODULES.iter().all(|m| names(report, m)),
            )
        })
        .collect()
}
