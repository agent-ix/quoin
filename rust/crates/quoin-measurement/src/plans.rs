// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! Loading `MeasurementPlan`s out of assurance-document frontmatter.
//!
//! Ports `src/measurement/plans.ts`. The walk itself is
//! [`crate::discovery::documents`] — see that module for why it is shared with
//! [`crate::profiles`] rather than written twice.
//!
//! # `preregistration` (PLAT-936)
//!
//! A plan's optional `preregistration.bar_digest` is a tamper-evidence digest
//! over the document's own [`BAR_SECTION_HEADING`] section, read here at load
//! time so [`crate::validate::measurement_collection`] can compare it against
//! what the document says today. A malformed block (not an object, no valid
//! `bar_digest`, or no such section to digest at all) refuses the whole plan
//! load with [`MeasurementErrorCode::PlanInvalid`], the same as any other
//! shape error this module already refuses — a *mismatched* digest is a
//! different question (does this bar still read the way it was pre-registered
//! to?) and is answered per observation, under
//! [`crate::error::MeasurementErrorCode::CollectionInvalid`], not here.

use quoin_store::{RawFileSha256Digest, digest_bytes_sha256};

use crate::discovery;
use crate::error::{MeasurementError, MeasurementErrorCode};
use crate::source::MeasurementSource;
use crate::types::ids::NonEmptyText;
use crate::types::plan::{LifecycleStatus, MeasurementPlan, MeasurementStage, PlanPreregistration};

/// The code every refusal in this module carries.
const CODE: MeasurementErrorCode = MeasurementErrorCode::PlanInvalid;

/// The heading a `preregistration` block digests.
///
/// Fixed rather than configurable: PLAT-936 scopes this check to one named
/// section, and a configurable heading would let an author point the digest
/// at prose nobody actually reviews as the enforcement bar.
const BAR_SECTION_HEADING: &str = "Comparison and Enforcement";

/// What a plan load should include beyond the members every caller needs.
///
/// `plans.ts:20` takes `{ includeGovernance?: boolean }` and spreads `owner`
/// and `action` in only when it is set, so a caller that did not ask cannot
/// read them by accident.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct PlanLoadOptions {
    /// Carry `owner` and `action` through when the document states them.
    pub include_governance: bool,
}

/// Load every `MeasurementPlan` the repository's assurance roots declare.
///
/// The result is sorted by metric, then id (`plans.ts:25`).
///
/// # Errors
///
/// [`MeasurementErrorCode::PlanInvalid`] when a `MeasurementPlan` document
/// lacks a required member, or names an unknown stage or status;
/// [`MeasurementErrorCode::Yaml`] for unreadable frontmatter; and
/// [`MeasurementErrorCode::Io`] when a document cannot be read.
pub fn load_measurement_plans<S: MeasurementSource + ?Sized>(
    source: &S,
    options: PlanLoadOptions,
) -> Result<Vec<MeasurementPlan>, MeasurementError> {
    let mut plans = discovery::documents(source, "MeasurementPlan", |path, value, text| {
        plan_from(path, value, text, options)
    })?;
    // Stable, so documents that agree on metric and id keep the walk's own
    // path order, as `Array.prototype.sort` does in V8.
    plans.sort_by(|a, b| a.sort_key().cmp(&b.sort_key()));
    Ok(plans)
}

fn plan_from(
    path: &str,
    value: &serde_json::Value,
    text: &str,
    options: PlanLoadOptions,
) -> Result<MeasurementPlan, MeasurementError> {
    let required = |name: &str| -> Result<NonEmptyText, MeasurementError> {
        discovery::required(value, name)
            .ok_or_else(|| {
                MeasurementError::new(
                    CODE,
                    format!("{path}: MeasurementPlan requires non-empty `{name}`"),
                )
            })
            .and_then(|text| NonEmptyText::parse(text, CODE, name))
    };
    // `plans.ts:44-56` checks the six required members before it looks at the
    // stage or the status, so a document missing `stage` entirely is refused
    // for the missing member rather than for an unknown stage.
    let id = required("id")?;
    let title = required("title")?;
    let status = required("status")?;
    let stage = required("stage")?;
    let metric = required("metric")?;
    let definition_version = required("definition_version")?;

    let stage = MeasurementStage::from_wire(stage.as_str()).ok_or_else(|| {
        MeasurementError::new(CODE, format!("{path}: unknown measurement stage `{stage}`"))
    })?;
    let status = LifecycleStatus::from_wire(status.as_str()).ok_or_else(|| {
        MeasurementError::new(
            CODE,
            format!("{path}: unknown MeasurementPlan status `{status}`"),
        )
    })?;

    let governance = |name: &str| -> Option<String> {
        if options.include_governance {
            discovery::required(value, name).map(str::to_owned)
        } else {
            None
        }
    };
    let preregistration = preregistration_from(path, value, text)?;
    Ok(MeasurementPlan {
        id,
        title,
        status,
        stage,
        metric,
        definition_version,
        path: path.to_owned(),
        owner: governance("owner"),
        action: governance("action"),
        preregistration,
    })
}

/// Read the optional `preregistration` block, or `None` when the document
/// declares no such attestation.
///
/// # Errors
///
/// [`MeasurementErrorCode::PlanInvalid`] when `preregistration` is present but
/// is not an object carrying a valid `bar_digest` (`sha256:<64 hex>`), or when
/// the document has no [`BAR_SECTION_HEADING`] section for it to digest.
fn preregistration_from(
    path: &str,
    value: &serde_json::Value,
    text: &str,
) -> Result<Option<PlanPreregistration>, MeasurementError> {
    let Some(block) = value.get("preregistration") else {
        return Ok(None);
    };
    let malformed = || {
        MeasurementError::new(
            CODE,
            format!("{path}: preregistration must be an object with a `bar_digest` sha256 digest"),
        )
    };
    let declared_digest = block
        .as_object()
        .and_then(|object| object.get("bar_digest"))
        .and_then(serde_json::Value::as_str)
        .ok_or_else(malformed)
        .and_then(|raw| RawFileSha256Digest::parse_stored(raw).map_err(|_| malformed()))?;
    let section = bar_section(text).ok_or_else(|| {
        MeasurementError::new(
            CODE,
            format!(
                "{path}: declares preregistration but has no \"{BAR_SECTION_HEADING}\" section \
                 to digest"
            ),
        )
    })?;
    Ok(Some(PlanPreregistration {
        declared_digest,
        computed_digest: digest_bytes_sha256(section.as_bytes()),
    }))
}

/// The document's own [`BAR_SECTION_HEADING`] section, normalized, or `None`
/// when no such heading exists.
///
/// Extraction: an ATX heading line (`#` through `######`) whose text, after
/// the hashes and surrounding whitespace, is exactly [`BAR_SECTION_HEADING`].
/// The section runs from the line after that heading to the next heading of
/// the same or a shallower level, or to the end of the document.
///
/// Normalization: `\r\n` becomes `\n`, each line's trailing whitespace is
/// trimmed, and leading and trailing blank lines are dropped. This is
/// deliberately insensitive to whitespace an editor's save action might touch
/// without changing the bar's meaning, and deliberately sensitive to
/// everything else the section's own prose says.
fn bar_section(text: &str) -> Option<String> {
    let normalized = text.replace("\r\n", "\n");
    let lines: Vec<&str> = normalized.lines().collect();
    let start = lines.iter().position(|line| {
        heading_text(line).is_some_and(|(_, title)| title == BAR_SECTION_HEADING)
    })?;
    let (level, _) = heading_text(lines.get(start)?)?;
    let after_heading = lines.get(start + 1..)?;
    let body_end = after_heading
        .iter()
        .position(|line| heading_text(line).is_some_and(|(found, _)| found <= level))
        .unwrap_or(after_heading.len());
    let body: Vec<&str> = after_heading
        .get(..body_end)?
        .iter()
        .map(|line| line.trim_end())
        .collect();
    let Some(first) = body.iter().position(|line| !line.is_empty()) else {
        return Some(String::new());
    };
    let last = body
        .iter()
        .rposition(|line| !line.is_empty())
        .unwrap_or(first);
    Some(body.get(first..=last)?.join("\n"))
}

/// An ATX heading's level (1 through 6 `#`s) and title text, or `None` when
/// `line` is not a heading at all.
///
/// A bare `#` with nothing after it, or more than six `#`s, is not a heading —
/// matching how every common Markdown renderer reads ATX headings.
fn heading_text(line: &str) -> Option<(usize, &str)> {
    let trimmed = line.trim();
    let hashes = trimmed
        .chars()
        .take_while(|character| *character == '#')
        .count();
    if hashes == 0 || hashes > 6 {
        return None;
    }
    let rest = trimmed.get(hashes..)?;
    (rest.is_empty() || rest.starts_with(char::is_whitespace)).then(|| (hashes, rest.trim()))
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
    use super::{PlanLoadOptions, load_measurement_plans};
    use crate::error::MeasurementErrorCode;
    use crate::source::MemoryMeasurement;
    use crate::types::plan::{LifecycleStatus, MeasurementStage};

    fn document(id: &str, metric: &str, stage: &str) -> String {
        format!(
            "---\ntype: MeasurementPlan\nid: {id}\ntitle: T\nstatus: active\nstage: \
             {stage}\nmetric: {metric}\ndefinition_version: v1\nowner: o\n---\n\n# {id}\n"
        )
    }

    #[test]
    fn plans_come_back_sorted_by_metric_then_id() {
        let source = MemoryMeasurement::new()
            .with_document("spec/assurance/b.md", document("MP-2", "zeta", "observe"))
            .with_document("spec/assurance/a.md", document("MP-3", "alpha", "gate"))
            .with_document("assurance/c.md", document("MP-1", "alpha", "trend"));
        let plans = load_measurement_plans(&source, PlanLoadOptions::default()).unwrap();
        let order: Vec<&str> = plans.iter().map(|plan| plan.id.as_str()).collect();
        assert_eq!(order, ["MP-1", "MP-3", "MP-2"]);
        assert_eq!(plans[0].stage, MeasurementStage::Trend);
        assert_eq!(plans[0].status, LifecycleStatus::Active);
    }

    #[test]
    fn governance_members_are_carried_only_when_asked_for() {
        let source =
            MemoryMeasurement::new().with_document("assurance/a.md", document("MP-1", "m", "gate"));
        let without = load_measurement_plans(&source, PlanLoadOptions::default()).unwrap();
        assert_eq!(without[0].owner, None);
        let with = load_measurement_plans(
            &source,
            PlanLoadOptions {
                include_governance: true,
            },
        )
        .unwrap();
        assert_eq!(with[0].owner.as_deref(), Some("o"));
    }

    #[test]
    fn a_document_of_another_type_is_skipped_and_a_bad_stage_is_refused() {
        let other = MemoryMeasurement::new()
            .with_document(
                "assurance/a.md",
                "---\ntype: AssuranceProfile\nid: AP-1\n---\n",
            )
            .with_document("assurance/b.md", "# no frontmatter\n");
        assert!(
            load_measurement_plans(&other, PlanLoadOptions::default())
                .unwrap()
                .is_empty()
        );

        let bad = MemoryMeasurement::new()
            .with_document("assurance/a.md", document("MP-1", "m", "speculate"));
        let error = load_measurement_plans(&bad, PlanLoadOptions::default()).unwrap_err();
        assert_eq!(error.code(), MeasurementErrorCode::PlanInvalid);
        assert!(error.subject().contains("unknown measurement stage"));
    }

    /// The digest of the normalized text `"Pass when agreement exceeds 0.8."`,
    /// computed once with `sha256sum` and asserted the same way every other
    /// literal-expectation test in this repo is: as a fixed value the test
    /// carries, not a re-derivation of the code under test.
    const BAR_DIGEST: &str =
        "sha256:369547844a3d9b02bc99dde37cf4628787a0d78c1d98aba19caf110c416dcb8e";

    fn document_with_bar(bar_digest: &str, bar_text: &str) -> String {
        format!(
            "---\ntype: MeasurementPlan\nid: MP-1\ntitle: T\nstatus: active\nstage: gate\n\
             metric: m\ndefinition_version: v1\nowner: o\npreregistration:\n  bar_digest: \
             {bar_digest}\n---\n\n# MP-1\n\n## Comparison and Enforcement\n\n{bar_text}\n"
        )
    }

    #[test]
    fn a_bar_digest_matching_the_section_text_loads_and_matches() {
        let source = MemoryMeasurement::new().with_document(
            "spec/assurance/a.md",
            document_with_bar(BAR_DIGEST, "Pass when agreement exceeds 0.8."),
        );
        let plans = load_measurement_plans(&source, PlanLoadOptions::default()).unwrap();
        let prereg = plans[0]
            .preregistration
            .as_ref()
            .expect("a preregistration block");
        assert!(prereg.matches());
    }

    #[test]
    fn a_stale_bar_digest_still_loads_but_does_not_match() {
        // The bar text was edited after `BAR_DIGEST` was recorded; loading a
        // plan never refuses for this by itself (PLAT-936: that refusal
        // belongs to `measurement_collection`, per observation, not here).
        let source = MemoryMeasurement::new().with_document(
            "spec/assurance/a.md",
            document_with_bar(BAR_DIGEST, "Pass when agreement exceeds 0.9."),
        );
        let plans = load_measurement_plans(&source, PlanLoadOptions::default()).unwrap();
        let prereg = plans[0]
            .preregistration
            .as_ref()
            .expect("a preregistration block");
        assert!(!prereg.matches());
    }

    #[test]
    fn a_preregistration_block_with_no_valid_bar_digest_is_refused() {
        let source = MemoryMeasurement::new().with_document(
            "spec/assurance/a.md",
            document_with_bar("not-a-digest", "Pass when agreement exceeds 0.8."),
        );
        let error = load_measurement_plans(&source, PlanLoadOptions::default()).unwrap_err();
        assert_eq!(error.code(), MeasurementErrorCode::PlanInvalid);
        assert!(error.subject().contains("bar_digest"));
    }

    #[test]
    fn a_preregistration_block_with_no_bar_section_is_refused() {
        let source = MemoryMeasurement::new().with_document(
            "spec/assurance/a.md",
            format!(
                "---\ntype: MeasurementPlan\nid: MP-1\ntitle: T\nstatus: active\nstage: gate\n\
                 metric: m\ndefinition_version: v1\nowner: o\npreregistration:\n  bar_digest: \
                 {BAR_DIGEST}\n---\n\n# MP-1\n\nNo such heading here.\n"
            ),
        );
        let error = load_measurement_plans(&source, PlanLoadOptions::default()).unwrap_err();
        assert_eq!(error.code(), MeasurementErrorCode::PlanInvalid);
        assert!(error.subject().contains("Comparison and Enforcement"));
    }

    #[test]
    fn a_plan_with_no_preregistration_block_loads_with_none() {
        let source =
            MemoryMeasurement::new().with_document("assurance/a.md", document("MP-1", "m", "gate"));
        let plans = load_measurement_plans(&source, PlanLoadOptions::default()).unwrap();
        assert_eq!(plans[0].preregistration, None);
    }

    #[test]
    fn bar_section_extraction_trims_blank_edges_and_stops_at_the_next_heading() {
        let text = "# Title\n\n## Comparison and Enforcement\n\n\nLine one.  \nLine two.\n\n\
                     ## Next Section\n\nignored\n";
        assert_eq!(
            super::bar_section(text).as_deref(),
            Some("Line one.\nLine two.")
        );
    }

    #[test]
    fn bar_section_extraction_stops_at_a_shallower_heading_but_not_a_deeper_one() {
        let text = "## Comparison and Enforcement\n\nIntro.\n\n### Detail\n\nMore.\n\n\
                     # Next Top-Level Section\n\nignored\n";
        assert_eq!(
            super::bar_section(text).as_deref(),
            Some("Intro.\n\n### Detail\n\nMore.")
        );
    }

    #[test]
    fn bar_section_extraction_finds_nothing_when_the_heading_is_absent() {
        assert_eq!(super::bar_section("# Title\n\nNo such heading.\n"), None);
    }
}
