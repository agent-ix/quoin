// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! An adapter transcribes text. It does not execute, read or fetch anything.
//!
//! Restated from `tests/campaign-adapters.test.ts`, which asserted this over
//! `src/evidence/adapters/*.ts` before that tree was deleted (quoin#458). The
//! signature `fn(&str) -> Result<_, EvidenceError>` says an adapter has no
//! other input to reach for, and that is most of the guarantee — but a
//! function can still open a file or spawn a process inside a body the type
//! never sees. Nothing at runtime can observe the difference between an
//! adapter that did not reach for a host capability and one that had no reason
//! to on the input it was given, which is why this is a source census:
//! `tc_source_conventions.rs` in `quoin-core` is the same resort for the same
//! reason.
//!
//! ADR-0011 invariant 1 is what is being defended: quoin transcribes, the
//! consumer's CI executes. An adapter that ran the tool would be quoin
//! producing the evidence it claims to be reporting.

#![allow(
    clippy::panic,
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "integration-test bodies: a panic here is a failing test, which is the intended signal"
)]

use std::fs;
use std::path::{Path, PathBuf};

/// Every capability an adapter must not name, with the reason it must not.
const FORBIDDEN: &[(&str, &str)] = &[
    (
        "std::process",
        "running the tool is the consumer's CI, not quoin's",
    ),
    ("Command::", "as above, by the other spelling"),
    (
        "std::fs",
        "an adapter receives its text; it does not go and get more",
    ),
    ("File::", "as above, by the other spelling"),
    ("reqwest", "no adapter reaches the network"),
    ("http://", "no adapter reaches the network"),
    ("https://", "no adapter reaches the network"),
    (
        "std::env",
        "the environment is not part of the producer's output",
    ),
    (
        "println!",
        "an adapter returns a result; it does not narrate",
    ),
    ("eprintln!", "as above — the boundary owns both streams"),
];

fn adapters_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("src/adapters")
}

/// Every adapter module is pure over the text it was handed.
///
/// Trace: FR-033-AC-3, FR-030-AC-17
/// Provenance: quoin#323, quoin#458
#[test]
fn tc_458_250_no_adapter_module_names_a_host_capability() {
    let mut examined: Vec<String> = Vec::new();
    for entry in fs::read_dir(adapters_dir()).expect("src/adapters is readable") {
        let path = entry.expect("a readable directory entry").path();
        if path.extension().is_none_or(|ext| ext != "rs") {
            continue;
        }
        let name = path
            .file_name()
            .expect("a file has a name")
            .to_string_lossy()
            .into_owned();
        // `mod.rs` is the contract's prose and `registry.rs` the table; both
        // are examined too, because a capability smuggled into either would
        // reach every adapter through it.
        let text = fs::read_to_string(&path).expect("an adapter module is readable");
        // The doc comments state the rule, so the census reads code only.
        let code: String = text
            .lines()
            .filter(|line| !line.trim_start().starts_with("//"))
            .collect::<Vec<_>>()
            .join("\n");
        for (forbidden, why) in FORBIDDEN {
            assert!(
                !code.contains(forbidden),
                "{name} names `{forbidden}` — {why}"
            );
        }
        examined.push(name);
    }
    examined.sort();
    // Anti-vacuity: a census over an empty or shrunken population passes while
    // proving nothing. Both registries plus the prose module, by name.
    assert_eq!(
        examined,
        [
            "agent_eval.rs",
            "audit_script.rs",
            "cargo_mutants.rs",
            "contract_conformance.rs",
            "differential_report.rs",
            "entries.rs",
            "junit.rs",
            "mod.rs",
            "registry.rs",
            "sarif.rs",
            "sbom.rs",
        ],
        "the adapter population changed; the census must name what it examined"
    );
}
