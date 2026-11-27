// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Observable version-command contracts across the native CLI entry points.

use std::process::Command;

use ix_trace_rs::trace;

#[trace("FR-002-AC-1", "FR-002-AC-2")]
#[test]
fn fr_002_ac_1_ac_2_version_aliases_print_the_nonempty_baked_version() {
    let expected = env!("QUOIN_VERSION");
    assert!(!expected.is_empty(), "the built CLI version is non-empty");

    for argument in ["version", "--version", "-v"] {
        let output = Command::new(env!("CARGO_BIN_EXE_quoin"))
            .arg(argument)
            .output()
            .expect("the native CLI binary runs");

        assert!(
            output.status.success(),
            "{argument} failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(output.stderr.is_empty(), "{argument} wrote to stderr");

        let stdout = String::from_utf8(output.stdout).expect("the CLI version is UTF-8");
        assert_eq!(stdout, format!("{expected}\n"), "{argument}");
        assert!(
            !stdout.trim().is_empty(),
            "{argument} printed an empty version"
        );
    }
}
