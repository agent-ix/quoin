// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Statement prose, and the hyphenated-compound guard over it.

use std::sync::LazyLock;

use regex::Regex;

/// `/\]\([^)]*\)/g` — a markdown link target, text and all brackets aside.
static LINK_TARGET: LazyLock<Regex> = LazyLock::new(|| {
    #[allow(
        clippy::expect_used,
        reason = "a literal pattern that fails to compile is a build defect, and \
                  the only alternative is a fallible constructor no caller can act on"
    )]
    Regex::new(r"\]\([^)]*\)").expect("the link-target pattern is a literal")
});

/// The prose of a statement, with markdown link **targets** removed.
///
/// A criterion cites its neighbours constantly —
/// `([StR-005-AC-4](../stakeholder/StR-005-no-runtime.md))` — and the path is
/// not something the author said about the requirement. Measured across the
/// five repos in this ecosystem when this was added: of 13 statements matching
/// `stakeholder`, **12 matched the directory name `../stakeholder/` inside a
/// link target** and one was the word in prose.
///
/// Link *text* is kept: `[the evidence store](./FR-030.md)` is prose the author
/// wrote. Only the destination goes.
#[must_use]
pub fn prose(statement: &str) -> String {
    LINK_TARGET.replace_all(statement, "]").into_owned()
}

/// Is a byte an ECMAScript `\w` — `[A-Za-z0-9_]`, ASCII and nothing wider?
///
/// A JavaScript `RegExp` without the `u` flag indexes by UTF-16 code unit and
/// resolves `\w` over ASCII only, so a byte test is exact here: a continuation
/// byte of a multi-byte character can only stand where JavaScript would have
/// seen a non-ASCII code unit, and `\w` rejects both.
const fn is_ascii_word(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_'
}

/// Does the regex match anywhere OUTSIDE a hyphenated compound token?
///
/// `\b` treats a hyphen as a boundary, so `\bunsafe\b` matched inside
/// `unsafe-audit` and a CI wall-clock budget was advised as a memory-safety
/// check (agent-ix/quoin#167). A hyphenated compound is ONE token: a match is
/// discarded when its edge is glued by a hyphen to more word characters —
/// i.e. when it caught a fragment of a compound rather than the whole of one.
///
/// A match that *contains* its hyphens (`use-after-free`, `constant-time`) is
/// untouched: the guard looks only at the characters just outside the match.
#[must_use]
pub fn matches_outside_compound(pattern: &Regex, text: &str) -> bool {
    let bytes = text.as_bytes();
    // `matchAll` with `g`: every non-overlapping leftmost-first match, which is
    // what `Regex::find_iter` yields.
    pattern.find_iter(text).any(|found| {
        let (start, end) = (found.start(), found.end());
        let glued_left = start >= 1
            && bytes.get(start - 1) == Some(&b'-')
            && start >= 2
            && bytes.get(start - 2).copied().is_some_and(is_ascii_word);
        let glued_right =
            bytes.get(end) == Some(&b'-') && bytes.get(end + 1).copied().is_some_and(is_ascii_word);
        !glued_left && !glued_right
    })
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]
mod tests {
    use regex::Regex;

    use super::{matches_outside_compound, prose};

    #[test]
    fn a_link_target_is_removed_and_its_text_is_kept() {
        assert_eq!(
            prose("cites ([StR-005-AC-4](../stakeholder/StR-005.md)) here"),
            "cites ([StR-005-AC-4]) here"
        );
        assert_eq!(prose("no links at all"), "no links at all");
    }

    #[test]
    fn a_bare_word_inside_a_hyphenated_compound_does_not_count() {
        let unsafe_word = Regex::new(r"(?i-u)\bunsafe\b").unwrap();
        assert!(
            !matches_outside_compound(&unsafe_word, "the unsafe-audit lane"),
            "a fragment of a compound is not the compound"
        );
        assert!(matches_outside_compound(
            &unsafe_word,
            "the lane forbids unsafe code"
        ));
    }

    #[test]
    fn a_match_that_contains_its_own_hyphens_is_untouched() {
        let whole = Regex::new(r"(?i-u)\bmemory[- ]safe(ty)?").unwrap();
        assert!(matches_outside_compound(&whole, "proves memory-safety"));
    }

    #[test]
    fn a_match_at_the_very_start_of_the_text_is_not_read_out_of_bounds() {
        let word = Regex::new(r"(?i-u)\bunsafe\b").unwrap();
        assert!(
            matches_outside_compound(&word, "unsafe"),
            "index -1 is `undefined` in JavaScript, not a panic"
        );
    }

    #[test]
    fn a_non_ascii_neighbour_is_not_a_word_character() {
        let word = Regex::new(r"(?i-u)\bunsafe\b").unwrap();
        // `é-unsafe`: glued left by a hyphen, but the character before the
        // hyphen is not `\w` under a non-`u` RegExp, so the match stands.
        assert!(matches_outside_compound(&word, "\u{e9}-unsafe"));
    }
}
