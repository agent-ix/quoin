// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Executable evidence for the semantic architecture record metrics (NFR-013).

#![allow(
    clippy::expect_used,
    reason = "test fixtures use expect as the failure report"
)]

use ix_trace_rs::trace;
use std::path::{Path, PathBuf};

const ARCH: &str = "docs/semantic-module-architecture";

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(3)
        .expect("crate is under rust/crates")
        .to_path_buf()
}

#[trace("NFR-013-M-1")]
#[test]
fn normative_architecture_records_name_their_governing_requirement_or_decision() {
    let base = root().join(ARCH);
    let index = std::fs::read_to_string(base.join("index.md")).expect("index reads");
    let normative: Vec<_> = index
        .lines()
        .filter(|line| line.contains("proposed normative"))
        .collect();
    assert_eq!(
        normative.len(),
        3,
        "each proposed normative record is indexed"
    );
    for (file, requirement_ids) in [
        ("planes-and-authority.md", &["FR-117", "FR-119"][..]),
        ("ownership-and-boundaries.md", &["FR-118", "FR-050"][..]),
        ("dynamic-and-generated.md", &["FR-049"][..]),
    ] {
        let line = normative
            .iter()
            .find(|line| line.contains(&format!("({file})")))
            .unwrap_or_else(|| panic!("{file} is missing from the normative index"));
        let content = std::fs::read_to_string(base.join(file)).expect("record reads");
        for requirement in requirement_ids {
            assert!(
                line.contains(requirement),
                "index entry for {file} lacks {requirement}"
            );
            assert!(
                content.contains(requirement),
                "normative record {file} lacks {requirement}"
            );
        }
    }
}

#[trace("NFR-013-M-2")]
#[test]
fn each_external_decision_row_carries_repository_path_status_and_source_revision() {
    let ledger = std::fs::read_to_string(root().join(ARCH).join("decision-ledger.md"))
        .expect("decision ledger reads");
    let rows: Vec<_> = ledger
        .lines()
        .filter(|line| line.starts_with('|') && !line.contains("---") && !line.contains("Decision"))
        .collect();
    assert_eq!(
        rows.len(),
        10,
        "all ten external decisions stay in the ledger"
    );
    for row in rows {
        let cells: Vec<_> = row.split('|').map(str::trim).collect();
        assert!(
            cells.len() >= 7,
            "ledger row must retain all metadata columns: {row}"
        );
        assert!(
            cells[2].starts_with("`agent-ix/"),
            "repository missing: {row}"
        );
        assert!(
            cells[3].contains(".md") || cells[3].contains("README.md"),
            "source path missing: {row}"
        );
        assert!(!cells[4].is_empty(), "decision status missing: {row}");
        let source = cells[5];
        let date = source
            .split_once('(')
            .and_then(|(_, value)| value.split_once(')'))
            .map(|(value, _)| value)
            .expect("source revision/date has a parenthetical date");
        let mut date_parts = date.split('-');
        let (Some(year), Some(month), Some(day)) =
            (date_parts.next(), date_parts.next(), date_parts.next())
        else {
            panic!("source date must be YYYY-MM-DD: {row}");
        };
        assert!(
            date_parts.next().is_none(),
            "source date must be YYYY-MM-DD: {row}"
        );
        assert!(
            year.len() == 4 && year.parse::<u16>().is_ok(),
            "source year is valid: {row}"
        );
        assert!(
            month.len() == 2 && (1..=12).contains(&month.parse::<u8>().unwrap_or(0)),
            "source month is valid: {row}"
        );
        assert!(
            day.len() == 2 && (1..=31).contains(&day.parse::<u8>().unwrap_or(0)),
            "source day is valid: {row}"
        );
        let revisions: Vec<_> = source
            .split_once('(')
            .map(|(revisions, _)| revisions)
            .unwrap_or_default()
            .split(',')
            .map(|revision| revision.trim().trim_matches('`'))
            .collect();
        assert!(
            !revisions.is_empty()
                && revisions.iter().all(|revision| {
                    revision.len() >= 7 && revision.chars().all(|ch| ch.is_ascii_hexdigit())
                }),
            "every source revision is a hex token: {row}"
        );
    }
}

#[trace("NFR-013-M-3")]
#[test]
fn architecture_index_links_resolve_to_existing_local_records() {
    let base = root().join(ARCH);
    for file in ["index.md", "adr/index.md"] {
        let document = std::fs::read_to_string(base.join(file)).expect("architecture index reads");
        for line in document.lines() {
            let Some(open) = line.find("](") else {
                continue;
            };
            let Some(close) = line[open + 2..].find(')') else {
                continue;
            };
            let target = &line[open + 2..open + 2 + close];
            if target.starts_with("http:")
                || target.starts_with("https:")
                || target.starts_with("ix:")
            {
                continue;
            }
            let path = target.split('#').next().unwrap_or(target);
            assert!(
                base.join(file)
                    .parent()
                    .expect("parent")
                    .join(path)
                    .exists(),
                "broken link in {file}: {target}"
            );
        }
    }
}

#[trace("NFR-013-M-4")]
#[test]
fn provisional_and_unresolved_decisions_are_not_presented_as_normative() {
    let base = root().join(ARCH);
    let index = std::fs::read_to_string(base.join("index.md")).expect("index reads");
    let ledger = std::fs::read_to_string(base.join("decision-ledger.md")).expect("ledger reads");
    let adr_index = std::fs::read_to_string(base.join("adr/index.md")).expect("ADR index reads");
    assert_eq!(
        adr_index
            .lines()
            .filter(|line| line.contains("| proposed |"))
            .count(),
        2,
        "every local ADR remains explicitly proposed until its promotion gate is met"
    );
    assert!(adr_index.contains("Named Quoin/Quire maintainer review on issue #289 PR"));
    let arch005 = ledger
        .lines()
        .find(|line| line.contains("`ARCH-005`"))
        .expect("ARCH-005 row exists");
    assert!(arch005.contains("provisional") && arch005.contains("gate `filament-core-data#9`"));
    assert!(
        !arch005.contains("| normative |"),
        "provisional ARCH-005 cannot be unqualified normative"
    );
    let arch006 = ledger
        .lines()
        .find(|line| line.contains("`ARCH-006`"))
        .expect("ARCH-006 row exists");
    assert!(arch006.contains("publication provisional"));
    assert!(index.contains("proposed normative"));
}
