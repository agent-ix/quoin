// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The native advice command filters rendered rows while retaining full-population tallies (FR-031).

#![allow(
    clippy::expect_used,
    reason = "test fixtures deliberately panic to report a failed command contract"
)]

use ix_trace_rs::trace;
use serde_json::Value;
use std::fs;
use std::process::{Command, Output};

const MODULE: &str = r##"
name: advisor-fixture
verification_catalog:
  security-review:
    name: Security review
    class: Analysis
    definition: Review the stated security characteristic.
    applicability:
      characteristics: [security]
  unit-testing:
    name: Unit testing
    class: Test
    definition: Exercise the behavior under test.
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
  trace_tags:
    markers:
      - name: rust-trace-attribute
        language: rust
        pattern: '#\[trace\(([^)]*)\)\]'
        template: "#[trace({ids})]"
"##;

const SPEC: &str = "---\nid: FR-001\ntype: FR\ntitle: Advice fixture\n---\n\n\
## Acceptance Criteria\n\n\
| ID | Criteria | Verification |\n\
|----|----------|--------------|\n\
| FR-001-AC-1 | The security value is retained. | Test |\n\
| FR-001-AC-2 | The value is retained. | Test |\n\
| FR-001-AC-3 | The security value is retained. | Smoke |\n";

struct Fixture {
    repo: tempfile::TempDir,
    home: tempfile::TempDir,
    module: tempfile::TempDir,
}

impl Fixture {
    fn new() -> Self {
        let repo = tempfile::tempdir().expect("repository fixture");
        let home = tempfile::tempdir().expect("isolated config root");
        let module = tempfile::tempdir().expect("module fixture");
        fs::create_dir_all(repo.path().join("spec")).expect("spec directory");
        fs::write(repo.path().join("spec/FR-001.md"), SPEC).expect("spec document");
        fs::write(module.path().join("manifest.yaml"), MODULE).expect("module manifest");
        Self { repo, home, module }
    }

    fn run(&self, extra: &[&str]) -> Output {
        let mut command = Command::new(env!("CARGO_BIN_EXE_quoin"));
        command
            .args(["advise", "--repo"])
            .arg(self.repo.path())
            .args(["--module"])
            .arg(self.module.path())
            .args(extra)
            .env("IX_HOME", self.home.path())
            .output()
            .expect("the native quoin binary runs")
    }

    fn rendered(&self, extra: &[&str]) -> String {
        let output = self.run(extra);
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let response: Value = serde_json::from_slice(&output.stdout).expect("response JSON");
        response["rendered"]
            .as_str()
            .expect("rendered advice")
            .to_owned()
    }
}

#[trace("FR-031-AC-24")]
#[test]
fn tc_fr031_combined_advice_filters_are_a_union_with_full_population_footer() {
    let fixture = Fixture::new();
    let rendered = fixture.rendered(&["--mismatch-only", "--inconclusive-only"]);

    assert!(rendered.contains("FR-001-AC-1"), "{rendered}");
    assert!(rendered.contains("FR-001-AC-2"), "{rendered}");
    assert!(!rendered.contains("FR-001-AC-3"), "{rendered}");
    assert!(
        rendered.contains(
            "2 of 3 obligation(s) shown. Of all 3: 1 mismatch, 1 uncatalogued, 1 inconclusive."
        ),
        "{rendered}"
    );
}
