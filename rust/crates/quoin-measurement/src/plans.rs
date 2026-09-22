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
//! `bar_digest`, or the section missing or empty) refuses the whole plan load
//! with [`MeasurementErrorCode::PlanInvalid`], the same as any other shape
//! error this module already refuses — a *mismatched* digest is a different
//! question (does this bar still read the way it was pre-registered to?) and
//! is answered once per plan actually referenced by a collection, under
//! [`crate::error::MeasurementErrorCode::CollectionInvalid`], not here.
//!
//! # `ground_truth_kind` and `statistical_design` (PLAT-960)
//!
//! Both are optional. A document that states neither loads exactly as it did
//! before PLAT-960. A document that states one with an unacceptable value —
//! a `ground_truth_kind` outside its three spellings, a `statistical_design`
//! that is not an object, or a `minimum_population` or `repetitions` that is
//! not a whole number from 1 to 4,294,967,295 — refuses the plan load with
//! [`MeasurementErrorCode::PlanInvalid`], the same as `preregistration`.
//! Enforcing `minimum_population` and `repetitions` against a collection is
//! [`crate::validate::measurement_collection`]'s job, not this module's.
//!
//! # `objective` (PLAT-958)
//!
//! Optional, and parsed into engineering-assurance's own
//! [`engineering_assurance::measurement::Objective`]. A block EA refuses — an
//! unknown `direction`, an unknown member, a non-numeric or non-finite
//! `bound`, or a `target` with no `bound` — refuses the plan load with
//! [`MeasurementErrorCode::PlanInvalid`].

mod design;

use crate::discovery;
use crate::error::{MeasurementError, MeasurementErrorCode};
use crate::source::MeasurementSource;
use crate::types::ids::NonEmptyText;
use crate::types::plan::{
    BarDigest, LifecycleStatus, MeasurementPlan, MeasurementStage, PlanPreregistration,
};

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
    let ground_truth_kind = design::ground_truth_kind_from(path, value)?;
    let statistical_design = design::statistical_design_from(path, value)?;
    let objective = design::objective_from(path, value)?;
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
        ground_truth_kind,
        statistical_design,
        objective,
    })
}

/// Read the optional `preregistration` block, or `None` when the document
/// declares no such attestation.
///
/// # Errors
///
/// [`MeasurementErrorCode::PlanInvalid`] when `preregistration` is present but
/// is not an object carrying a valid `bar_digest` (`sha256:<64 hex>`), or when
/// the document has no non-empty [`BAR_SECTION_HEADING`] section for it to
/// digest — an empty section is refused here rather than admitted with a
/// vacuous digest over nothing.
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
        .and_then(BarDigest::parse_stored)
        .ok_or_else(malformed)?;
    let section = bar_section(text).ok_or_else(|| {
        MeasurementError::new(
            CODE,
            format!(
                "{path}: declares preregistration but its \"{BAR_SECTION_HEADING}\" section is \
                 missing or empty"
            ),
        )
    })?;
    Ok(Some(PlanPreregistration {
        declared_digest,
        computed_digest: BarDigest::of(section.as_bytes()),
    }))
}

/// The document's own [`BAR_SECTION_HEADING`] section, normalized, or `None`
/// when no such non-empty heading exists.
///
/// Extraction: an ATX heading line (`#` through `######`) whose text, after
/// the hashes and surrounding whitespace, is exactly [`BAR_SECTION_HEADING`].
/// The section runs from the line after that heading to the next heading of
/// the same or a shallower level, or to the end of the document. A heading
/// line found inside a fenced code block does not count — see
/// [`fenced_code_lines`] — so a `#`-prefixed shell or Python comment inside a
/// ```` ``` ```` or `~~~` fence pasted into the section cannot be misread as
/// the heading that ends it.
///
/// Normalization: `\r\n` becomes `\n`, each line's trailing whitespace is
/// trimmed, and leading and trailing blank lines are dropped. This is
/// deliberately insensitive to whitespace an editor's save action might touch
/// without changing the bar's meaning, and deliberately sensitive to
/// everything else the section's own prose says.
///
/// `None` for a section that is present but reduces to nothing after
/// normalization (blank, or entirely inside a code fence): an empty digest
/// would pass vacuously forever, which is not tamper evidence over anything.
fn bar_section(text: &str) -> Option<String> {
    let normalized = text.replace("\r\n", "\n");
    let lines: Vec<&str> = normalized.lines().collect();
    let fenced = fenced_code_lines(&lines);
    let start = lines.iter().enumerate().position(|(index, line)| {
        !fenced.get(index).copied().unwrap_or(true)
            && heading_text(line).is_some_and(|(_, title)| title == BAR_SECTION_HEADING)
    })?;
    let (level, _) = heading_text(lines.get(start)?)?;
    let after_heading = lines.get(start + 1..)?;
    let after_fenced = fenced.get(start + 1..)?;
    let body_end = after_heading
        .iter()
        .zip(after_fenced)
        .position(|(line, in_fence)| {
            !in_fence && heading_text(line).is_some_and(|(found, _)| found <= level)
        })
        .unwrap_or(after_heading.len());
    let body: Vec<&str> = after_heading
        .get(..body_end)?
        .iter()
        .map(|line| line.trim_end())
        .collect();
    let first = body.iter().position(|line| !line.is_empty())?;
    let last = body
        .iter()
        .rposition(|line| !line.is_empty())
        .unwrap_or(first);
    Some(body.get(first..=last)?.join("\n"))
}

/// Whether each line of `lines` sits inside a fenced code block (` ``` ` or
/// `~~~`, three or more of the same character, opened with at most three
/// leading spaces — matching `CommonMark`'s own fence rule closely enough for
/// this purpose). Both the opening and closing fence line are marked `true`
/// along with everything between them: neither is a heading candidate.
///
/// A closing fence must repeat the same character at least as many times as
/// the opener; anything else (a mismatched character, or a shorter run) does
/// not close it, matching how every common Markdown renderer reads fences.
/// An unclosed fence runs to the end of the document.
fn fenced_code_lines(lines: &[&str]) -> Vec<bool> {
    let mut in_fence: Option<(char, usize)> = None;
    lines
        .iter()
        .map(|line| {
            let indent = line.len() - line.trim_start_matches(' ').len();
            let trimmed = line.trim_start_matches(' ');
            let marker = |character: char| -> Option<usize> {
                if indent > 3 {
                    return None;
                }
                let run = trimmed
                    .chars()
                    .take_while(|found| *found == character)
                    .count();
                (run >= 3).then_some(run)
            };
            match in_fence {
                None => {
                    if let Some(run) = marker('`').or_else(|| marker('~')) {
                        let character = trimmed.chars().next().unwrap_or('`');
                        in_fence = Some((character, run));
                    }
                    // `true` exactly when this line just opened a fence: the
                    // opening delimiter itself is never a heading candidate.
                    in_fence.is_some()
                }
                Some((character, run)) => {
                    if marker(character).is_some_and(|closing| closing >= run) {
                        in_fence = None;
                    }
                    // The closing delimiter line is also never a heading
                    // candidate, regardless of whether it just closed the
                    // fence.
                    true
                }
            }
        })
        .collect()
}

/// An ATX heading's level (1 through 6 `#`s) and title text, or `None` when
/// `line` is not a heading at all.
///
/// A bare `#` with nothing after it on the line **is** a heading, with an
/// empty title — matching `CommonMark` and every common Markdown renderer.
/// More than six `#`s, or fewer than four leading spaces of indentation past
/// that, is not.
///
/// Indentation past three spaces disqualifies a line from being a heading at
/// all, matching `CommonMark`'s own rule that four or more leading spaces start
/// an indented code block instead: `heading_text` is never fooled by a
/// `#`-prefixed comment sitting in one.
fn heading_text(line: &str) -> Option<(usize, &str)> {
    let indent = line.len() - line.trim_start_matches(' ').len();
    if indent > 3 || line.starts_with('\t') {
        return None;
    }
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

    /// The blocking defect the second review round found: a `#`-prefixed
    /// shell comment inside a fenced code block pasted into the section was
    /// misread as a heading, silently truncating the digested span so
    /// everything after it (including real bar text) sat outside the digest,
    /// free to edit with no refusal.
    #[test]
    fn bar_section_extraction_is_not_fooled_by_a_hash_comment_inside_a_code_fence() {
        let text = "## Comparison and Enforcement\n\n\
                     Run the gate before merging.\n\n\
                     ```bash\n\
                     # run the gate\n\
                     make rust-gate\n\
                     ```\n\n\
                     Pass when the gate is green.\n\n\
                     ## Next Section\n\nignored\n";
        assert_eq!(
            super::bar_section(text).as_deref(),
            Some(
                "Run the gate before merging.\n\n\
                 ```bash\n\
                 # run the gate\n\
                 make rust-gate\n\
                 ```\n\n\
                 Pass when the gate is green."
            )
        );
    }

    #[test]
    fn bar_section_extraction_is_not_fooled_by_a_hash_comment_in_a_tilde_fence() {
        let text = "## Comparison and Enforcement\n\n\
                     ~~~python\n\
                     # not a heading\n\
                     ~~~\n\n\
                     Pass when agreement exceeds 0.8.\n";
        assert_eq!(
            super::bar_section(text).as_deref(),
            Some("~~~python\n# not a heading\n~~~\n\nPass when agreement exceeds 0.8.")
        );
    }

    #[test]
    fn bar_section_extraction_ignores_a_shorter_or_mismatched_closing_fence() {
        // A two-backtick line does not close a three-backtick fence, and a
        // tilde line does not close a backtick fence either — so the `#` line
        // that follows stays inside the (still-open) fence in both cases.
        let text = "## Comparison and Enforcement\n\n\
                     ```\n\
                     ``\n\
                     ~~~\n\
                     # still fenced\n\
                     ```\n\n\
                     Pass when agreement exceeds 0.8.\n";
        assert_eq!(
            super::bar_section(text).as_deref(),
            Some("```\n``\n~~~\n# still fenced\n```\n\nPass when agreement exceeds 0.8.")
        );
    }

    #[test]
    fn bar_section_extraction_ignores_a_heading_indented_as_code() {
        // Four or more leading spaces is CommonMark's own indented-code-block
        // rule; `heading_text` must not be fooled by a `#`-prefixed line
        // sitting in one, fenced or not.
        let text = "## Comparison and Enforcement\n\n\
                     Some prose.\n\n    \
                     # not a heading, this is indented code\n\n\
                     Pass when agreement exceeds 0.8.\n";
        assert_eq!(
            super::bar_section(text).as_deref(),
            Some(
                "Some prose.\n\n    # not a heading, this is indented code\n\n\
                 Pass when agreement exceeds 0.8."
            )
        );
    }

    /// PLAT-936 review finding #4: an empty section must refuse rather than
    /// admit a vacuous digest over nothing, which would pass forever.
    #[test]
    fn a_preregistration_block_over_an_empty_bar_section_is_refused() {
        let source = MemoryMeasurement::new().with_document(
            "spec/assurance/a.md",
            format!(
                "---\ntype: MeasurementPlan\nid: MP-1\ntitle: T\nstatus: active\nstage: gate\n\
                 metric: m\ndefinition_version: v1\nowner: o\npreregistration:\n  bar_digest: \
                 {BAR_DIGEST}\n---\n\n# MP-1\n\n## Comparison and Enforcement\n\n## Next\n"
            ),
        );
        let error = load_measurement_plans(&source, PlanLoadOptions::default()).unwrap_err();
        assert_eq!(error.code(), MeasurementErrorCode::PlanInvalid);
        assert!(error.subject().contains("missing or empty"));
    }

    #[test]
    fn bar_section_extraction_returns_none_for_an_empty_section() {
        assert_eq!(
            super::bar_section("## Comparison and Enforcement\n\n## Next\n"),
            None
        );
    }

    /// A bare `#` with nothing after it is still a heading (empty title),
    /// matching `CommonMark` — confirmed here because the doc comment on
    /// `heading_text` previously claimed the opposite (review finding #1).
    #[test]
    fn bar_section_extraction_treats_a_bare_hash_as_a_heading_that_ends_the_section() {
        let text =
            "## Comparison and Enforcement\n\nPass when agreement exceeds 0.8.\n\n#\n\nignored\n";
        assert_eq!(
            super::bar_section(text).as_deref(),
            Some("Pass when agreement exceeds 0.8.")
        );
    }
}
