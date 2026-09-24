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
//!
//! # `statistical_design.estimator` and `.decision_rule` (PLAT-961)
//!
//! Optional, and parsed into engineering-assurance's own `Estimator` and
//! `DecisionRule` (its FR-021). A value EA refuses, a `constant-predictor`
//! baseline under an estimator other than `proportion`, or a comparator that
//! disagrees with the plan's `objective` refuses the plan load with
//! [`MeasurementErrorCode::PlanInvalid`] naming the member.
//!
//! # `protected_apparatus` and `negative_controls` (PLAT-975)
//!
//! Optional, and parsed into engineering-assurance's own types (its FR-024).
//! A list EA refuses, a `gate` plan missing either list, or an
//! `apparatus-edit` control with no `protected_apparatus`, refuses the plan
//! load with
//! [`MeasurementErrorCode::PlanInvalid`] naming the member.

mod apparatus;
mod design;
mod history;

pub use history::{definition_changed_without_version_bump, plan_from_text};

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

pub(crate) fn plan_from(
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
    let execution_procedure = execution_procedure_from(path, value)?;
    // The suffix is reserved for constant-predictor item observations
    // (PLAT-1016): a plan governing `x.constant-predictor-item` would read
    // plan `x`'s item rows as its own run, and intake and `compare` could no
    // longer tell an item row from a governed observation.
    if crate::types::observation::constant_predictor_governed_metric(metric.as_str()).is_some() {
        return Err(MeasurementError::new(
            CODE,
            format!(
                "{path}: MeasurementPlan metric `{metric}` ends with the reserved \
                 constant-predictor item suffix `{}`",
                crate::types::observation::CONSTANT_PREDICTOR_ITEM_METRIC_SUFFIX
            ),
        ));
    }

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
    design::rule_agrees_with_objective(path, statistical_design.as_ref(), objective.as_ref())?;
    let (protected_apparatus, negative_controls) = apparatus::apparatus_from(path, value, stage)?;
    if let Some(procedure) = execution_procedure.as_deref() {
        let covered = protected_apparatus.as_ref().is_some_and(|apparatus| {
            apparatus.iter().any(|entry| {
                entry.as_str() == procedure
                    || entry.directory().is_some_and(|directory| {
                        procedure.starts_with(directory)
                            && procedure.as_bytes().get(directory.len()) == Some(&b'/')
                    })
            })
        });
        if !covered {
            return Err(MeasurementError::new(
                CODE,
                format!("{path}: execution_procedure must be protected by protected_apparatus"),
            ));
        }
    }
    Ok(MeasurementPlan {
        id,
        title,
        status,
        stage,
        metric,
        definition_version,
        execution_procedure,
        path: path.to_owned(),
        owner: governance("owner"),
        action: governance("action"),
        preregistration,
        ground_truth_kind,
        statistical_design,
        objective,
        protected_apparatus,
        negative_controls,
    })
}

fn execution_procedure_from(
    plan_path: &str,
    value: &serde_json::Value,
) -> Result<Option<String>, MeasurementError> {
    let Some(raw) = value.get("execution_procedure") else {
        return Ok(None);
    };
    let procedure_path = raw.as_str().ok_or_else(|| {
        MeasurementError::new(
            CODE,
            format!("{plan_path}: execution_procedure must be a path"),
        )
    })?;
    let parsed = engineering_assurance::measurement::ApparatusPath::new(procedure_path).map_err(
        |error| {
            MeasurementError::new(
                CODE,
                format!("{plan_path}: invalid execution_procedure: {error}"),
            )
        },
    )?;
    let json_file = std::path::Path::new(procedure_path)
        .extension()
        .is_some_and(|extension| extension.eq_ignore_ascii_case("json"));
    if parsed.directory().is_some() || !json_file {
        return Err(MeasurementError::new(
            CODE,
            format!("{plan_path}: execution_procedure must name one JSON file"),
        ));
    }
    Ok(Some(procedure_path.to_owned()))
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
mod tests;
