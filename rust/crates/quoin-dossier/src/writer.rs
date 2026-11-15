// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The `DossierDocument` writer (FR-191, "Document set", "Gaps section").
//!
//! Frontmatter is exactly the `DossierDocument` archetype's fields; the file
//! ends with the level-2 `Gaps` section. The output is a pure function of its
//! arguments, so two runs over the same inputs are byte-identical.

use std::fmt::Write as _;

use crate::gap::Gap;
use crate::registry::DocumentSpec;

/// The `generated_by` frontmatter value.
const GENERATED_BY: &str = "quoin dossier";

/// The `type` frontmatter value.
const DOCUMENT_TYPE: &str = "DossierDocument";

/// Write one complete document file.
///
/// `as_of` is written only for a document whose spec asks for it.
#[must_use]
pub(crate) fn compose(
    spec: &DocumentSpec,
    version: &str,
    as_of: Option<&str>,
    body: &str,
    gaps: &[Gap],
) -> String {
    let mut out = String::new();
    out.push_str("---\n");
    push_field(&mut out, "id", spec.id);
    push_field(&mut out, "title", spec.title);
    push_field(&mut out, "version", version);
    let _ = writeln!(out, "type: {DOCUMENT_TYPE}");
    push_field(&mut out, "document_kind", spec.kind);
    let _ = writeln!(out, "generated_by: {GENERATED_BY}");
    if spec.needs_as_of
        && let Some(instant) = as_of
    {
        push_field(&mut out, "as_of", instant);
    }
    out.push_str("---\n\n");
    let body = body.trim_matches('\n');
    if !body.is_empty() {
        out.push_str(body);
        out.push_str("\n\n");
    }
    out.push_str("## Gaps\n\n");
    if gaps.is_empty() {
        out.push_str("Gaps: none\n");
    } else {
        for gap in gaps {
            out.push_str(&gap.bullet());
            out.push('\n');
        }
    }
    out
}

/// A double-quoted scalar. JSON string syntax is valid YAML, so a value with a
/// colon, quote or line break cannot change the shape of the frontmatter.
fn push_field(out: &mut String, key: &str, value: &str) {
    let quoted = serde_json::to_string(value).unwrap_or_else(|_| "\"\"".to_owned());
    let _ = writeln!(out, "{key}: {quoted}");
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
    use crate::gap::GapKind;
    use ix_trace_rs::trace;

    const SPEC: DocumentSpec = DocumentSpec {
        kind: "srs",
        id: "DOS-SRS",
        title: "Software Requirements Specification",
        needs_as_of: false,
    };

    #[trace("FR-191-AC-14")]
    #[test]
    fn a_complete_document_reads_gaps_none_and_carries_exactly_the_archetype_frontmatter() {
        let text = compose(&SPEC, "1.0", None, "# SRS\n", &[]);
        assert_eq!(
            text,
            "---\nid: \"DOS-SRS\"\ntitle: \"Software Requirements Specification\"\nversion: \"1.0\"\ntype: DossierDocument\ndocument_kind: \"srs\"\ngenerated_by: quoin dossier\n---\n\n# SRS\n\n## Gaps\n\nGaps: none\n"
        );
    }

    #[trace("FR-191-AC-14")]
    #[test]
    fn gaps_are_bullets_under_the_final_heading() {
        let gaps = [
            Gap::new(GapKind::NoRequirements, "document", "none found"),
            Gap::new(GapKind::NoEvidence, "FR-1-AC-1", "no test"),
        ];
        let text = compose(&SPEC, "1.0", None, "", &gaps);
        assert!(text.ends_with(
            "## Gaps\n\n- `no_requirements` document: none found\n- `no_evidence` FR-1-AC-1: no test\n"
        ));
        assert!(!text.contains("Gaps: none"));
    }

    #[trace("FR-191")]
    #[test]
    fn as_of_is_written_only_where_the_document_needs_it() {
        let mut spec = SPEC;
        let without = compose(&spec, "1", Some("2026-11-15T00:00:00Z"), "", &[]);
        assert!(!without.contains("as_of"));
        spec.needs_as_of = true;
        let with = compose(&spec, "1", Some("2026-11-15T00:00:00Z"), "", &[]);
        assert!(with.contains("as_of: \"2026-11-15T00:00:00Z\"\n---"));
    }

    #[trace("FR-191")]
    #[test]
    fn a_hostile_version_cannot_add_a_frontmatter_key() {
        let text = compose(&SPEC, "1\"\ntype: Other", None, "", &[]);
        assert!(text.lines().all(|line| line != "type: Other"));
        assert_eq!(
            text.lines()
                .filter(|line| line.starts_with("type:"))
                .count(),
            1
        );
    }
}
