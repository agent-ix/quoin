// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! Putting producer-supplied text inside a markdown table cell, and inside a
//! markdown list line, without either of them escaping.
//!
//! Ports `markdownCell` and `inlineText` (`graph-portfolio.ts:848-857`). Both
//! exist because a partition's `measure`, `dimension` and `key` are producer
//! strings: an unescaped `|` splits a table row into extra columns and a
//! newline ends it altogether, which turns one bad dimension key into a
//! rendered report that is silently missing rows.

/// `markdownCell` (`graph-portfolio.ts:848-853`).
///
/// Backslash first, then `|`, then every line break as `<br>`. The order is
/// the retained order and it matters: escaping `|` first would then have its
/// own backslash escaped.
#[must_use]
pub fn markdown_cell(value: &str) -> String {
    let escaped: String = value
        .chars()
        .flat_map(|character| match character {
            '\\' => Some('\\').into_iter().chain(Some('\\')),
            '|' => Some('\\').into_iter().chain(Some('|')),
            other => None.into_iter().chain(Some(other)),
        })
        .collect();
    replace_line_breaks(&escaped, "<br>")
}

/// `inlineText` (`graph-portfolio.ts:855-857`).
#[must_use]
pub fn inline_text(value: &str) -> String {
    replace_line_breaks(value, " ")
}

/// `/\r\n?|\n/g` — a CRLF pair, a lone CR, or a lone LF, each one occurrence.
fn replace_line_breaks(value: &str, with: &str) -> String {
    let mut out = String::with_capacity(value.len());
    let mut characters = value.chars().peekable();
    while let Some(character) = characters.next() {
        match character {
            '\r' => {
                if characters.peek() == Some(&'\n') {
                    characters.next();
                }
                out.push_str(with);
            }
            '\n' => out.push_str(with),
            other => out.push(other),
        }
    }
    out
}

#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]
#[cfg(test)]
mod tests {
    use super::{inline_text, markdown_cell};

    /// Each expectation is what the retained regular expressions produce.
    #[test]
    fn a_cell_cannot_escape_its_column_or_its_row() {
        assert_eq!(markdown_cell("a|b"), "a\\|b");
        assert_eq!(markdown_cell("a\\b"), "a\\\\b");
        // The backslash is escaped first, so the escape of `|` is not
        // re-escaped: `\|` in, `\\\|` out.
        assert_eq!(markdown_cell("a\\|b"), "a\\\\\\|b");
        assert_eq!(markdown_cell("a\r\nb\rc\nd"), "a<br>b<br>c<br>d");
        assert_eq!(markdown_cell("plain"), "plain");
    }

    #[test]
    fn inline_text_only_flattens_line_breaks() {
        assert_eq!(inline_text("a\r\nb\rc\nd"), "a b c d");
        assert_eq!(inline_text("a|b\\c"), "a|b\\c");
    }
}
