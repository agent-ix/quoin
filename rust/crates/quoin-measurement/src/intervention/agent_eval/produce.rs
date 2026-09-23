// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! Turning two retained agent-eval runs into one intervention record.
//!
//! A port of `produceAgentEvalIntervention` (`agent-eval-intervention.ts:14-132`).
//!
//! # What this producer will not claim
//!
//! It always writes `cause_not_established` with `attribution_confidence:
//! "none"`, whatever the numbers say. Two agent-eval runs are an observation,
//! not an experiment: nothing here randomises, blocks or holds a confounder
//! constant, so a pass-rate difference is a difference and not a cause. The
//! retained code is deliberate about this and so is the port — the `gaps` list
//! is where the honesty lives, and it grows a member for each of the three
//! ways this producer knows it is under-powered.

use std::path::{Path, PathBuf};

use engineering_assurance::claim_strength::ClaimStrength;
use serde_json::Number;

use crate::common::identity::{MetricName, WireInstant};
use crate::common::scalar::{EffectValue, EnvironmentValue, ScalarValue};
use crate::intervention::agent_eval::AgentEvalInterventionDefinition;
use crate::intervention::agent_eval::report::AgentEvalReport;
use crate::intervention::agent_eval::version::ImmutableVersion;
use crate::intervention::intake::{
    InterventionIntakeError, InterventionRefusalCode, write_intervention_record,
};
use crate::intervention::record::{
    AttributionConfidence, InterventionArm, InterventionConclusion, InterventionConclusionKind,
    InterventionDisposition, InterventionExperimentRecord, InterventionRecordType,
    InterventionStatus, MeasuredEffect,
};
use crate::json_bridge::from_serde;
use crate::raw_evidence::raw_evidence_for;
use crate::source::{DiskMeasurement, MeasurementSource};

/// The `report_schema_version` this producer reads, and the only one.
pub const REPORT_SCHEMA_VERSION: &str = "cli-agent-evals-report-v1";

/// The media type both retained reports are recorded under.
const REPORT_MEDIA_TYPE: &str = "application/json";

/// What a production produced: the record, and where it was published.
#[derive(Debug, Clone, PartialEq)]
pub struct ProducedIntervention {
    /// Where the record was written.
    pub path: PathBuf,
    /// The record as written.
    pub record: InterventionExperimentRecord,
}

/// Produce and publish an intervention record from two retained runs.
///
/// # Errors
///
/// [`InterventionRefusalCode::DefinitionMismatch`] for a definition this
/// producer will not accept, [`InterventionRefusalCode::InvalidRecord`] for a
/// retained report it cannot read or two reports that do not cover the same
/// scenarios, and every refusal
/// [`write_intervention_record`] raises on the way to disk.
pub fn produce_agent_eval_intervention(
    repo: &Path,
    definition: &AgentEvalInterventionDefinition,
) -> Result<ProducedIntervention, InterventionIntakeError> {
    validate_definition(definition)?;
    let source = DiskMeasurement::new(repo);
    let baseline_raw = read_retained(&source, definition.baseline_evidence_path.as_str())?;
    let treatment_raw = read_retained(&source, definition.treatment_evidence_path.as_str())?;
    // The established FR-042 refusal boundary, named rather than selected from
    // a tool string: this is the only reader of these two files, and a
    // fall-through to the normalized shape would accept a document `agent-eval`
    // refuses (`agent-eval-intervention.ts:21-27`).
    for raw in [&baseline_raw, &treatment_raw] {
        quoin_evidence::adapters::parse_agent_eval(raw)
            .map_err(|error| refuse(error.to_string()))?;
    }
    let baseline = AgentEvalReport::parse(&baseline_raw, "baseline")?;
    let treatment = AgentEvalReport::parse(&treatment_raw, "treatment")?;
    let scenarios: Vec<&String> = baseline.scenarios().keys().collect();
    if scenarios != treatment.scenarios().keys().collect::<Vec<_>>() {
        return Err(refuse(format!(
            "agent-eval scenario mismatch: baseline=[{}], treatment=[{}]",
            join(baseline.scenarios().keys()),
            join(treatment.scenarios().keys())
        )));
    }

    let measured_effects = measured_effects(definition, &baseline, &treatment)?;
    let observed_difference = measured_effects
        .iter()
        .any(|effect| is_nonzero(&effect.effect));
    let record = InterventionExperimentRecord {
        schema_version: 1,
        record_type: InterventionRecordType::InterventionExperiment,
        record_id: definition.record_id.clone(),
        observed_at: WireInstant::from_stored(treatment.generated_at()),
        subject: definition.subject.clone(),
        producer: producer(definition),
        strength: ClaimStrength::Tested,
        design: definition.design.clone(),
        baseline: arm(&definition.baseline, baseline.sample_size()),
        treatments: vec![arm(&definition.treatment, treatment.sample_size())],
        changed_variables: definition.changed_variables.clone(),
        held_constant: definition.held_constant.clone(),
        measured_effects,
        interactions: definition.interactions.clone(),
        confounders: definition.confounders.clone(),
        status: InterventionStatus::Completed,
        conclusion: InterventionConclusion {
            kind: InterventionConclusionKind::CauseNotEstablished,
            statement: statement(observed_difference).to_owned(),
            attribution_confidence: AttributionConfidence::None,
        },
        gaps: gaps(definition, &baseline, &treatment),
        owner: definition.owner.clone(),
        actions: definition.actions.clone(),
        raw_evidence: vec![
            raw_evidence_for(
                &source,
                definition.baseline_evidence_path.as_str(),
                REPORT_MEDIA_TYPE,
            )?
            .into(),
            raw_evidence_for(
                &source,
                definition.treatment_evidence_path.as_str(),
                REPORT_MEDIA_TYPE,
            )?
            .into(),
        ],
    };
    let document = serde_json::to_value(&record)
        .map_err(|error| refuse(format!("the produced record is not representable: {error}")))?;
    let path = write_intervention_record(repo, &from_serde(&document)?)?;
    Ok(ProducedIntervention { path, record })
}

/// The two sentences `agent-eval-intervention.ts:110-113` chooses between.
const fn statement(observed_difference: bool) -> &'static str {
    if observed_difference {
        "The retained agent-evaluation runs show observed pass-rate differences; this adapter does \
         not establish causality."
    } else {
        "The retained agent-evaluation runs show no observed pass-rate difference; this adapter \
         does not establish causality."
    }
}

/// `validateDefinition` (`agent-eval-intervention.ts:183-215`).
///
/// Its first check — `!isRecord(value)` — has no port: the argument is a typed
/// record here, so "not an object" is unrepresentable rather than unchecked.
fn validate_definition(
    definition: &AgentEvalInterventionDefinition,
) -> Result<(), InterventionIntakeError> {
    if definition.report_schema_version != REPORT_SCHEMA_VERSION {
        return Err(InterventionIntakeError::new(
            InterventionRefusalCode::DefinitionMismatch,
            vec![format!(
                "producer definition requires report_schema_version {REPORT_SCHEMA_VERSION}"
            )],
        ));
    }
    ImmutableVersion::parse(
        &definition.cli_agent_evals_version,
        "cli_agent_evals_version",
    )?;
    ImmutableVersion::parse(definition.subject.revision.as_str(), "subject.revision")?;
    ImmutableVersion::parse(
        definition.producer.source_revision.as_str(),
        "producer.source_revision",
    )?;
    let baseline = definition.baseline_evidence_path.as_str();
    let treatment = definition.treatment_evidence_path.as_str();
    if baseline.is_empty() || treatment.is_empty() || baseline == treatment {
        return Err(InterventionIntakeError::new(
            InterventionRefusalCode::DefinitionMismatch,
            vec![
                "producer definition requires distinct baseline and treatment evidence paths"
                    .to_owned(),
            ],
        ));
    }
    if definition.treatment.id.as_str().is_empty() {
        return Err(InterventionIntakeError::new(
            InterventionRefusalCode::DefinitionMismatch,
            vec!["producer definition requires a treatment id".to_owned()],
        ));
    }
    Ok(())
}

/// `readRetained` (`agent-eval-intervention.ts:217-221`).
///
/// Minting the reference first is not a formality: it is what confines the
/// path to the evidence root and refuses a symlink escape, and it must happen
/// before anything reads the bytes.
fn read_retained<S: MeasurementSource + ?Sized>(
    source: &S,
    path: &str,
) -> Result<String, InterventionIntakeError> {
    let reference = raw_evidence_for(source, path, REPORT_MEDIA_TYPE)?;
    Ok(source.raw_evidence_text(&reference.path)?)
}

/// The producer as recorded: the definition's, with the two versions this
/// production was run at recorded into its environment.
fn producer(definition: &AgentEvalInterventionDefinition) -> crate::common::producer::Producer {
    let mut producer = definition.producer.clone();
    producer.environment.insert(
        "cli_agent_evals_version".to_owned(),
        EnvironmentValue::String(definition.cli_agent_evals_version.clone()),
    );
    producer.environment.insert(
        "report_schema_version".to_owned(),
        EnvironmentValue::String(definition.report_schema_version.clone()),
    );
    producer
}

/// A definition's arm, with the sample size its report supplies.
fn arm(
    definition: &crate::intervention::agent_eval::InterventionArmDefinition,
    sample_size: u64,
) -> InterventionArm {
    InterventionArm {
        id: definition.id.clone(),
        population: definition.population.clone(),
        sample_size,
        configuration: definition.configuration.clone(),
    }
}

/// One effect per scenario, in scenario-id order.
fn measured_effects(
    definition: &AgentEvalInterventionDefinition,
    baseline: &AgentEvalReport,
    treatment: &AgentEvalReport,
) -> Result<Vec<MeasuredEffect>, InterventionIntakeError> {
    baseline
        .scenarios()
        .iter()
        .map(|(id, before)| {
            let after = treatment.scenarios().get(id).ok_or_else(|| {
                // Unreachable: the key sets were compared above. A refusal
                // rather than a panic, because a panic in a producer is what
                // the workspace panic lints exist to stop.
                refuse(format!("scenario {id} is absent from the treatment report"))
            })?;
            Ok(MeasuredEffect {
                treatment_id: definition.treatment.id.clone(),
                metric: MetricName::from_stored(scenario_metric(id)),
                baseline_value: ScalarValue::Number(number(before.rate())?),
                treatment_value: ScalarValue::Number(number(after.rate())?),
                effect: EffectValue::Number(number(after.rate() - before.rate())?),
                unit: "fraction".to_owned(),
            })
        })
        .collect()
}

/// `agent-eval.${id.toLowerCase().replace(/[^a-z0-9._-]+/g, "-")}.pass-rate`.
fn scenario_metric(id: &str) -> String {
    let mut safe = String::with_capacity(id.len());
    let mut in_run = false;
    for character in id.to_lowercase().chars() {
        if character.is_ascii_lowercase()
            || character.is_ascii_digit()
            || matches!(character, '.' | '_' | '-')
        {
            safe.push(character);
            in_run = false;
        } else if !in_run {
            // A **run** of unsafe characters collapses to one `-`, which is
            // what the `+` in the retained regular expression does.
            safe.push('-');
            in_run = true;
        }
    }
    format!("agent-eval.{safe}.pass-rate")
}

/// The declared gaps, plus the three this producer knows about itself.
fn gaps(
    definition: &AgentEvalInterventionDefinition,
    baseline: &AgentEvalReport,
    treatment: &AgentEvalReport,
) -> Vec<String> {
    let mut gaps = definition.gaps.clone();
    let unrepeated = [baseline, treatment]
        .iter()
        .flat_map(|report| report.scenarios().values())
        .any(|rate| rate.total < 2);
    if unrepeated {
        gaps.push(
            "agent-eval reports do not contain repeated samples for every scenario".to_owned(),
        );
    }
    let open = definition
        .interactions
        .iter()
        .chain(&definition.confounders)
        .any(|item| {
            matches!(
                item.disposition,
                InterventionDisposition::Uncontrolled | InterventionDisposition::Unknown
            )
        });
    if open {
        gaps.push(
            "one or more declared interactions or confounders are uncontrolled or unknown"
                .to_owned(),
        );
    }
    if definition
        .attribution_method
        .as_ref()
        .is_none_or(String::is_empty)
    {
        gaps.push("no justified attribution method was supplied".to_owned());
    }
    // `[...new Set(gaps)]` — first occurrence wins and the order is kept.
    let mut unique = Vec::with_capacity(gaps.len());
    for gap in gaps {
        if !unique.contains(&gap) {
            unique.push(gap);
        }
    }
    unique
}

/// `item.effect !== 0` — an exact comparison, on a difference of two exact
/// quotients, which is what the oracle branches on.
fn is_nonzero(effect: &EffectValue) -> bool {
    match effect {
        EffectValue::Number(number) => number.as_f64().is_none_or(|value| value != 0.0),
        EffectValue::Null | EffectValue::String(_) => true,
    }
}

/// A rate as a JSON number.
fn number(value: f64) -> Result<Number, InterventionIntakeError> {
    // Unreachable: every rate is a finite quotient of two finite integers.
    Number::from_f64(value).ok_or_else(|| refuse(format!("{value} is not a JSON number")))
}

/// `[...ids].join(", ")` for the mismatch sentence.
fn join<'a>(ids: impl Iterator<Item = &'a String>) -> String {
    ids.map(String::as_str).collect::<Vec<_>>().join(", ")
}

/// A refusal naming the record this producer could not assemble.
fn refuse(finding: String) -> InterventionIntakeError {
    InterventionIntakeError::new(InterventionRefusalCode::InvalidRecord, vec![finding])
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]
mod tests {
    use super::scenario_metric;

    /// Trace: FR-100-AC-6
    /// Provenance: quoin#471
    ///
    /// The `+` in the retained regular expression is the whole content of this
    /// test: a run of unsafe characters is **one** dash, not one each.
    #[test]
    fn a_scenario_id_becomes_one_metric_name() {
        for (id, expected) in [
            ("plain", "agent-eval.plain.pass-rate"),
            ("Mixed.Case_1", "agent-eval.mixed.case_1.pass-rate"),
            ("a  b", "agent-eval.a-b.pass-rate"),
            ("a/b:c", "agent-eval.a-b-c.pass-rate"),
            ("  ", "agent-eval.-.pass-rate"),
        ] {
            assert_eq!(scenario_metric(id), expected, "{id}");
        }
    }
}
