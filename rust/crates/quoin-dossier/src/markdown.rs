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
        .replace(['\r', '\n'], " ")
}

/// One line of text, such as the tail of a heading.
pub(crate) fn line(text: &str) -> String {
    text.replace(['\r', '\n'], " ").trim().to_owned()
}

/// A block of prose copied from a document: a line that opens with `#` would
/// start a heading of the generated document, so it is backslash-escaped.
pub(crate) fn prose(text: &str) -> String {
    text.lines()
        .map(|row| {
            if row.trim_start().starts_with('#') {
                format!("\\{}", row.trim_start())
            } else {
                row.to_owned()
            }
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
    fn a_prose_line_cannot_open_a_heading() {
        assert_eq!(
            prose("text\n## FR-9: forged\nmore"),
            "text\n\\## FR-9: forged\nmore"
        );
    }
}
