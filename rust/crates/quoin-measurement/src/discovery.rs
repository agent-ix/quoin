// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! The assurance-document walk, once.
//!
//! # Why this module exists at all
//!
//! `plans.ts:29-33,86-96` and `profiles.ts:26-30,62-72` are **byte-identical
//! duplicates** on `origin/main`: the same `assuranceFiles` over
//! `spec/assurance` and `assurance`, the same iterative `markdownFiles` walk,
//! the same `compare`. Porting them twice would carry a known duplication into
//! new code, so the walk lives on [`crate::source::MeasurementSource`] and the
//! frontmatter half lives here. `plans.rs` and `profiles.rs` differ only in the
//! `type` discriminator they accept and the record they build.

use serde_json::Value;

use crate::error::MeasurementError;
use crate::source::MeasurementSource;

/// Read the YAML frontmatter block at the head of `text`.
///
/// Reproduces `/^---\r?\n([\s\S]*?)\r?\n---(?:\r?\n|$)/`: the block opens on
/// the first line, the body is **minimal**, and the closing fence must be a
/// line of its own. A `----` line does not close it, which is why this scans
/// rather than taking the first `\n---`.
#[must_use]
pub fn frontmatter(text: &str) -> Option<&str> {
    let body = text
        .strip_prefix("---\n")
        .or_else(|| text.strip_prefix("---\r\n"))?;
    let mut searched = 0;
    while let Some(offset) = body.get(searched..)?.find("\n---") {
        let start = searched + offset;
        let after = body.get(start + 4..)?;
        let closes = after.is_empty() || after.starts_with('\n') || after.starts_with("\r\n");
        if closes {
            // `\r?\n---`: drop the carriage return the regex allows.
            let end = body.get(..start)?;
            return Some(end.strip_suffix('\r').unwrap_or(end));
        }
        searched = start + 1;
    }
    None
}

/// Every assurance document whose frontmatter declares `type: <kind>`, mapped.
///
/// `build` is called with the repository-relative document path and the parsed
/// frontmatter mapping, and returns the record or refuses. A document with no
/// frontmatter, or with a different `type`, is skipped rather than refused —
/// `plans.ts:41-43` and `profiles.ts:41-43` both `return null` for those.
///
/// # Errors
///
/// [`MeasurementError`] when a document cannot be read, when its frontmatter is
/// not readable YAML, or when `build` refuses.
pub(crate) fn documents<S, T>(
    source: &S,
    kind: &str,
    mut build: impl FnMut(&str, &Value) -> Result<T, MeasurementError>,
) -> Result<Vec<T>, MeasurementError>
where
    S: MeasurementSource + ?Sized,
{
    let mut out = Vec::new();
    for path in source.assurance_documents()? {
        let text = source.document_text(&path)?;
        let Some(block) = frontmatter(&text) else {
            continue;
        };
        let value = quoin_yaml::from_str(block)?;
        if value.get("type").and_then(Value::as_str) != Some(kind) {
            continue;
        }
        out.push(build(&path, &value)?);
    }
    Ok(out)
}

/// A required non-empty string member of a frontmatter mapping.
pub(crate) fn required<'a>(value: &'a Value, name: &str) -> Option<&'a str> {
    value
        .get(name)
        .and_then(Value::as_str)
        .filter(|text| !text.is_empty())
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
    use super::frontmatter;

    #[test]
    fn a_block_must_open_on_the_first_line() {
        assert_eq!(frontmatter("---\nid: A\n---\nbody\n"), Some("id: A"));
        assert_eq!(frontmatter("\n---\nid: A\n---\n"), None);
        assert_eq!(frontmatter("# title\n---\nid: A\n---\n"), None);
    }

    #[test]
    fn the_body_is_minimal_and_the_fence_is_a_whole_line() {
        // A `----` line does not close the block; the next `---` line does.
        assert_eq!(
            frontmatter("---\na: 1\n----\nb: 2\n---\ntail"),
            Some("a: 1\n----\nb: 2")
        );
        assert_eq!(frontmatter("---\na: 1\n---\nb: 2\n---\n"), Some("a: 1"));
    }

    #[test]
    fn carriage_returns_are_accepted_on_both_fences() {
        assert_eq!(frontmatter("---\r\nid: A\r\n---\r\nbody"), Some("id: A"));
    }

    #[test]
    fn a_block_may_close_at_end_of_file() {
        assert_eq!(frontmatter("---\nid: A\n---"), Some("id: A"));
        assert_eq!(frontmatter("---\nid: A\n"), None);
    }
}
