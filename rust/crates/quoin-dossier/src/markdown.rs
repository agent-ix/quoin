// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Escaping for text a generator copies out of a source document, so one value
//! can never end its table cell, its heading or its section (FR-192, FR-193).

use quoin_quire::records::{Contract, Corpus};

use crate::error::SourceFailure;

/// One table cell: a `|` would end the cell and a newline the row.
pub(crate) fn cell(text: &str) -> String {
    text.replace('\\', "\\\\")
        .replace('|', "\\|")
        .replace('`', "\\`")
        .replace('<', "\\<")
        .replace(['\r', '\n'], " ")
}

/// One line of text, such as the tail of a heading.
pub(crate) fn line(text: &str) -> String {
    text.replace(['\r', '\n'], " ")
        .replace('<', "\\<")
        .trim()
        .to_owned()
}

/// A setext underline: only `-` or only `=` (CommonMark allows indentation and
/// trailing blanks around it, which the caller trims).
fn is_underline(text: &str) -> bool {
    !text.is_empty() && (text.chars().all(|c| c == '-') || text.chars().all(|c| c == '='))
}

/// A block of prose copied from a document. Every construct that could change
/// the structure of the generated document is backslash-escaped: a line that
/// opens a heading with `#`, a setext underline (a line of only `-` or `=`), a
/// code fence, and every `<`, so no raw HTML comment can hide what follows.
pub(crate) fn prose(text: &str) -> String {
    text.lines()
        .map(|row| {
            let trimmed = row.trim_start();
            let structural = trimmed.starts_with('#')
                || trimmed.starts_with("```")
                || trimmed.starts_with("~~~")
                || is_underline(trimmed.trim_end());
            let row = if structural {
                format!("\\{trimmed}")
            } else {
                row.to_owned()
            };
            row.replace('<', "\\<")
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// The failure for a corpus that holds a file the generator would otherwise
/// leave out silently: a file the walker could not load, a file with an
/// unusable frontmatter, or a document of a checked type that breaks its own
/// contract.
pub(crate) fn unusable_input(source: &str, corpus: &Corpus) -> Option<SourceFailure> {
    let mut problems: Vec<String> = Vec::new();
    for file in &corpus.unreadable {
        problems.push(format!(
            "{}: unreadable: {}",
            file.path.display(),
            file.reason
        ));
    }
    for path in &corpus.malformed {
        problems.push(format!("{}: malformed frontmatter", path.display()));
    }
    for document in &corpus.documents {
        if let Contract::Violated(reason) = &document.contract {
            problems.push(format!("{}: {reason}", document.path.display()));
        }
    }
    if problems.is_empty() {
        return None;
    }
    Some(SourceFailure {
        source: source.to_owned(),
        detail: problems.join("; "),
    })
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]
mod tests {
    use super::*;

    #[test]
    fn a_pipe_and_a_newline_stay_in_one_cell() {
        assert_eq!(cell("a|b\nc"), "a\\|b c");
        assert_eq!(cell("a\\|b"), "a\\\\\\|b");
    }

    #[test]
    fn a_heading_cannot_be_ended_by_a_newline() {
        assert_eq!(line("Title\n## Injected"), "Title ## Injected");
    }

    #[test]
    fn setext_underlines_fences_and_raw_html_are_escaped() {
        assert_eq!(prose("Title\n---"), "Title\n\\---");
        assert_eq!(prose("Title\n  ==="), "Title\n\\===");
        assert_eq!(prose("Title\n---  "), "Title\n\\---  ");
        assert_eq!(prose("Title\n---\t"), "Title\n\\---\t");
        assert_eq!(prose("Title\n=== \t "), "Title\n\\=== \t ");
        for indent in [" ", "  ", "   "] {
            assert_eq!(prose(&format!("Title\n{indent}---")), "Title\n\\---");
        }
        assert_eq!(prose("```\ncode"), "\\```\ncode");
        assert_eq!(prose("a <!-- hide"), "a \\<!-- hide");
        assert_eq!(cell("a<b"), "a\\<b");
        assert_eq!(prose("a -- b"), "a -- b");
    }

    #[test]
    fn a_prose_line_cannot_open_a_heading() {
        assert_eq!(
            prose("text\n## FR-9: forged\nmore"),
            "text\n\\## FR-9: forged\nmore"
        );
    }
}
