// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Wall-clock budget for the bundle-scale commands (NFR-011, quoin#133).
//!
//! A generated bundle of 250 documents is walked by `completeness`, `advise`,
//! `evidence audit` and `matrix`. The budget is NFR-011's 5 second threshold
//! (target 2 seconds). It exists so that a change making a bundle walk an order
//! of magnitude slower, such as one that goes quadratic in documents, fails a
//! test instead of being absorbed as "CI got slower".
//!
//! The pass count (NFR-011-M-2) is NOT asserted here. The four commands read
//! the bundle through different readers in different crates and the repository
//! has no read-counting seam, so counting reads would need a production change.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "integration-test bodies: a panic here is a failing test, which is the intended signal"
)]

use ix_trace_rs::trace;
use std::path::Path;
use std::process::{Command, Output};
use std::time::{Duration, Instant};

const DOCUMENTS: usize = 250;

/// NFR-011's threshold.
const BUDGET: Duration = Duration::from_secs(5);

const MODULE: &str = r"name: quoin-cli-budget-fixture
version: 0.1.0
description: Fixture module for the bundle-scale budget test
artifact_types:
  - name: FR
    grammar_ref: iso-spec-core
    defaults:
      id_pattern: FR-{next:03d}
verification_catalog:
  analysis:
    name: Analysis
    class: Analysis
    definition: inspect
traceability:
  trace_targets:
    - name: acceptance-criterion
      archetype: FR
      section: Acceptance Criteria
      id_column: ID
  obligations:
    - name: acceptance-criterion
      target: acceptance-criterion
      statement_column: Criteria
      method_column: Verification
";

fn document(number: usize) -> String {
    let rows: Vec<String> = (1..=3)
        .map(|criterion| {
            format!(
                "| FR-{number:03}-AC-{criterion} | The reader shall handle case {criterion} of requirement {number}. |  |"
            )
        })
        .collect();
    format!(
        "---\nid: FR-{number:03}\ntype: FR\ntitle: Requirement {number}\n---\n\n## Acceptance Criteria\n\n| ID | Criteria | Verification |\n|----|----------|--------------|\n{}\n",
        rows.join("\n")
    )
}

fn run(repo: &Path, home: &Path, arguments: &[&str]) -> (Output, Duration) {
    let started = Instant::now();
    let output = Command::new(env!("CARGO_BIN_EXE_quoin"))
        .args(arguments)
        .current_dir(repo)
        .env("IX_HOME", home)
        .env("GIT_CEILING_DIRECTORIES", repo.parent().unwrap())
        .output()
        .expect("the native quoin binary runs");
    (output, started.elapsed())
}

/// Each bundle-scale command finishes a 250-document bundle inside the
/// threshold.
#[trace("NFR-011-M-1")]
#[test]
fn tc_133_100_bundle_scale_commands_stay_inside_the_time_budget() {
    let repo = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();
    std::fs::write(repo.path().join("manifest.yaml"), MODULE).unwrap();
    std::fs::create_dir_all(repo.path().join("spec")).unwrap();
    for number in 1..=DOCUMENTS {
        std::fs::write(
            repo.path().join(format!("spec/FR-{number:03}-example.md")),
            document(number),
        )
        .unwrap();
    }

    // The first launch of a freshly built binary can cost seconds to the
    // operating system (code-signing and malware scans on macOS) before quoin
    // runs a line. That is not bundle cost, so it is paid before the clock.
    let _ = run(repo.path(), home.path(), &["--help"]);

    let commands: [&[&str]; 4] = [
        &["completeness", "--module", ".", "--json"],
        &["advise", "--module", ".", "--json"],
        &["evidence", "audit", "--module", ".", "--json"],
        &["matrix", "--module", ".", "--json"],
    ];
    for arguments in commands {
        let (output, elapsed) = run(repo.path(), home.path(), arguments);
        eprintln!("{arguments:?}: {elapsed:?} exit {:?}", output.status.code());
        assert!(
            elapsed < BUDGET,
            "{arguments:?} took {elapsed:?} over {DOCUMENTS} documents, budget {BUDGET:?}"
        );
        // A command that fails fast would pass the budget while walking nothing.
        assert!(
            output.status.code().is_some_and(|code| code <= 1),
            "{arguments:?} did not run to a verdict: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(
            !output.stdout.is_empty(),
            "{arguments:?} printed nothing: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}
