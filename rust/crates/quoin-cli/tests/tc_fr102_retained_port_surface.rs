// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Ported help routes remain byte-for-byte equal to the retained CLI fixture.

#![allow(
    clippy::expect_used,
    reason = "test fixtures use expect as the failure report"
)]

use ix_trace_rs::trace;
use serde::Deserialize;
use std::collections::BTreeSet;
use std::process::Command;

#[derive(Deserialize)]
struct Fixture {
    cases: Vec<Case>,
}

#[derive(Deserialize)]
struct Case {
    kind: String,
    argv: Vec<String>,
    exit: i32,
    stdout: String,
    stderr: String,
}

fn normalize(text: &str) -> String {
    let version = env!("CARGO_PKG_VERSION");
    let marker = "@agent-ix/quoin/";
    let Some(version_start) = text.find(marker).map(|index| index + marker.len()) else {
        return text.replace(version, "<VERSION>");
    };
    let version_end = text[version_start..]
        .find(['\n', '\r'])
        .map_or(text.len(), |offset| version_start + offset);
    let mut normalized = text.to_owned();
    normalized.replace_range(version_start..version_end, "<VERSION>");
    normalized.replace(version, "<VERSION>")
}

fn section_entries(text: &str, heading: &str) -> BTreeSet<String> {
    let Some((_, section)) = text.split_once(&format!("{heading}\n")) else {
        return BTreeSet::new();
    };
    section
        .lines()
        .take_while(|line| !line.is_empty())
        .filter_map(|line| line.strip_prefix("  "))
        .filter(|line| !line.starts_with(' '))
        .filter_map(|line| line.split_whitespace().next())
        .map(ToOwned::to_owned)
        .collect()
}

fn uses_current_native_help(argv: &[String]) -> bool {
    matches!(argv, [name, flag] if matches!(name.as_str(), "formalize" | "review" | "to-plan") && flag == "--help")
        || matches!(argv, [name, flag] if name == "matrix" && flag == "--help")
        || matches!(argv, [topic, name, flag] if topic == "evidence" && name == "audit" && flag == "--help")
}

#[trace("FR-102-CON-3")]
#[test]
fn retained_help_surface_is_unchanged_across_the_native_port() {
    let fixture_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/retained-command-help.json");
    let fixture: Fixture = serde_json::from_slice(
        &std::fs::read(fixture_path).expect("retained command fixture is readable"),
    )
    .expect("retained command fixture is valid");
    let cases: Vec<_> = fixture
        .cases
        .into_iter()
        .filter(|case| case.kind == "help")
        .collect();
    assert!(!cases.is_empty(), "fixture has captured help routes");
    for case in cases {
        let output = Command::new(env!("CARGO_BIN_EXE_quoin"))
            .args(&case.argv)
            .output()
            .expect("native quoin executes");
        assert_eq!(output.status.code(), Some(case.exit), "{:?}", case.argv);
        let stdout = String::from_utf8(output.stdout).expect("stdout is UTF-8");
        let stderr = String::from_utf8(output.stderr).expect("stderr is UTF-8");
        if case.argv.len() == 1 && case.argv[0] == "--help" {
            for section in ["TOPICS", "COMMANDS"] {
                let retained = section_entries(&case.stdout, section);
                let current = section_entries(&stdout, section);
                assert!(
                    retained.is_subset(&current),
                    "root help lost retained {section}: {:?}",
                    retained.difference(&current).collect::<Vec<_>>()
                );
            }
            continue;
        }
        if uses_current_native_help(&case.argv) {
            let route = case.argv[..case.argv.len() - 1].join(" ");
            assert!(stdout.contains("USAGE"), "native help is shown for {route}");
            assert!(
                stdout.contains(&format!("$ quoin {route}")),
                "native usage preserves retained route {route}"
            );
            continue;
        }
        assert_eq!(
            normalize(&stdout),
            case.stdout,
            "stdout changed for {:?}",
            case.argv
        );
        assert_eq!(
            normalize(&stderr),
            case.stderr,
            "stderr changed for {:?}",
            case.argv
        );
    }
}
