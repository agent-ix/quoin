// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! FR-040's rendering and mermaid criteria, against the public API (quoin#447).
//!
//! Byte-identical rendering, mermaid punctuation, label truncation on code
//! points while node ids count code units, and the empty case. The fixtures
//! are in `common/fixtures.rs`, which also carries the preamble this suite was
//! split out of.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]

use quoin_assurance::case::build_case;

#[path = "common/fixtures.rs"]
mod fixtures;
#[path = "common/render.rs"]
mod render_hop;

use crate::fixtures::{bundle, input, obligation};
use crate::render_hop::render;

/// The contents of the first mermaid fence, exclusive of the fences.
fn mermaid(rendered: &str) -> String {
    rendered
        .split("```mermaid")
        .nth(1)
        .expect("a mermaid block is rendered")
        .split("```")
        .next()
        .expect("the fence closes")
        .to_owned()
}

/// Trace: FR-040-AC-8
/// Provenance: agent-ix/quoin#447, tests/assurance.test.ts
#[test]
fn tc_447_410_rendering_is_byte_identical_over_unchanged_inputs() {
    // A store view, never a hand-maintained document: a diff must mean the
    // evidence changed, not that somebody re-ran it.
    let first = render(&build_case(&input(
        bundle(),
        vec![obligation("FR-001-AC-1", "It holds.")],
        vec![],
    )));
    let second = render(&build_case(&input(
        bundle(),
        vec![obligation("FR-001-AC-1", "It holds.")],
        vec![],
    )));
    assert_eq!(first, second);

    // The literal, not a re-derivation: the whole document for the retained
    // test's own bundle, heading by heading and blank line by blank line.
    assert_eq!(
        first,
        "# Assurance case\n\
         \n\
         1 claim(s), 0 with an open branch.\n\
         \n\
         ## ✅ StR-001 — The system is usable\n\
         \n\
         \x20 - ✅ FR-001 — Parse the input\n\
         \x20   - ✅ **FR-001-AC-1** — It holds.\n\
         \n\
         ```mermaid\n\
         flowchart TD\n\
         \x20 StR_001[\"StR-001: The system is usable\"]\n\
         \x20 FR_001[\"FR-001: Parse the input\"]\n\
         \x20 FR_001_AC_1([\"FR-001-AC-1: It holds.\"])\n\
         \x20 FR_001 --> FR_001_AC_1\n\
         \x20 StR_001 --> FR_001\n\
         ```\n"
    );
}

/// Trace: FR-040-AC-9
/// Provenance: agent-ix/quoin#447, tests/assurance.test.ts
#[test]
fn tc_447_411_mermaid_cannot_be_broken_by_a_statements_punctuation() {
    // Each of these renders a broken diagram rather than a wrong one, which is
    // worse: nobody reviews a diagram that will not draw.
    let result = build_case(&input(
        bundle(),
        vec![obligation(
            "FR-001-AC-1",
            "Rejects (bad) input; logs \"why\"",
        )],
        vec![],
    ));
    let block = mermaid(&render(&result));

    assert!(block.contains("FR_001_AC_1"), "{block}");
    assert!(!block.contains("FR-001-AC-1("), "{block}");
    assert!(!block.contains(';'), "{block}");
    // `/"[^"\n]*"[^"\n]*"/` — a third quote on one line ends the label early.
    for line in block.lines() {
        assert!(
            line.matches('"').count() <= 2,
            "a label must carry no nested quote: {line}"
        );
    }

    // The literal the sanitiser produces, so the test says what the reader
    // sees rather than only what they will not see: `"` becomes `'` and `;`
    // becomes `,`, and the parentheses are left alone because the quotes are
    // what protects them.
    assert!(
        block.contains("  FR_001_AC_1([\"FR-001-AC-1: Rejects (bad) input, logs 'why'\"])\n"),
        "{block}"
    );
}

/// Trace: FR-040-AC-15
/// Provenance: agent-ix/quoin#447, tests/assurance.test.ts, quoin#432
#[test]
fn tc_447_412_a_mermaid_label_truncates_on_a_code_point_while_node_ids_count_code_units() {
    // quoin#432. `String.prototype.slice` counts UTF-16 CODE UNITS, so a label
    // whose 80th unit fell inside a surrogate pair was cut in half and rendered
    // as U+FFFD.
    //
    // Constructed rather than taken from the corpus, and deliberately: no
    // label in the retained repository's 991 obligations reaches the boundary
    // with an astral character today, and waiting for a real one is waiting
    // for the bug.
    //
    // The label is `{id}: {statement}`, so the id and its separator are part
    // of the budget — the statement is padded to put the emoji astride the
    // 80-unit boundary of the whole label.
    let id = "FR-001-AC-1";
    let prefix = format!("{id}: ");
    let statement = format!("{}\u{1f600}", "x".repeat(80 - prefix.len()));
    let result = build_case(&input(bundle(), vec![obligation(id, &statement)], vec![]));
    let block = mermaid(&render(&result));

    // The emoji is dropped WHOLE rather than halved. In Rust a lone surrogate
    // cannot be held in a `String` at all, so the assertion that carries the
    // criterion is the truncation boundary itself, stated as a literal.
    assert!(
        block.contains(&format!(
            "([\"{prefix}{}\"])",
            "x".repeat(80 - prefix.len())
        )),
        "{block}"
    );
    assert!(!block.contains('\u{1f600}'), "{block}");
    assert!(!block.contains('\u{fffd}'), "{block}");
    // The whole block, not just the label: a replacement character anywhere in
    // the output is the defect, wherever it came from.
    assert!(std::str::from_utf8(block.as_bytes()).is_ok());

    // The other half, and the one a `chars()` port gets wrong in the opposite
    // direction: the node-id sanitiser emits one underscore per UTF-16 CODE
    // UNIT, so an astral character becomes TWO. Same defect family, read from
    // the retained regex's lack of a `u` flag rather than from preference.
    let astral_id = "FR-001-AC-\u{1f600}";
    let result = build_case(&input(
        bundle(),
        vec![obligation(astral_id, "It holds.")],
        vec![],
    ));
    let block = mermaid(&render(&result));
    // Three underscores after `AC`: one for the hyphen, TWO for the emoji.
    // A `chars()` port emits two and the node would still draw — which is why
    // the count is asserted rather than the diagram merely being checked.
    assert!(
        block.contains("  FR_001_AC___([\"FR-001-AC-\u{1f600}: It holds.\"])\n"),
        "{block}"
    );
    assert!(!block.contains("FR_001_AC__(["), "{block}");
}

/// Trace: FR-040-AC-15
/// Provenance: agent-ix/quoin#447, tests/assurance.test.ts, quoin#432
#[test]
fn tc_447_413_a_label_with_no_astral_character_truncates_byte_for_byte_as_before() {
    // The fix must not move any byte that was already correct, which is 833 of
    // the retained repository's 835 truncated labels. A 100-character ASCII
    // statement exercises the truncation path; code points and code units
    // agree on every input where one of each is the other.
    let id = "FR-001-AC-1";
    let statement = "y".repeat(100);
    let result = build_case(&input(bundle(), vec![obligation(id, &statement)], vec![]));
    let block = mermaid(&render(&result));

    // No finding, so the node is supported and carries no ` ◇` suffix.
    let full = format!("{id}: {statement}");
    let expected: String = full.chars().take(80).collect();
    assert_eq!(expected.len(), 80);
    assert!(block.contains(&format!("([\"{expected}\"])")), "{block}");
}

/// Trace: FR-040-AC-10
/// Provenance: agent-ix/quoin#447, tests/assurance.test.ts
#[test]
fn tc_447_414_an_empty_case_says_there_is_no_case_rather_than_rendering_one() {
    // An empty document reads as "nothing to argue". The truth is that nothing
    // declared itself a claim, and those are different problems.
    let rendered = render(&build_case(&input(vec![], vec![], vec![])));

    assert!(rendered.contains("No document declares itself a top-level claim"));
    // The whole document: an empty case is short enough to state literally,
    // and "contains a sentence" would pass over a document that also rendered
    // a spurious claim count.
    assert_eq!(
        rendered,
        "# Assurance case\n\
         \n\
         No document declares itself a top-level claim, so there is no case to\n\
         argue. Declare an `StR` (or pass `--claim-type`) and re-run.\n"
    );
    assert!(!rendered.contains("claim(s)"));
}
