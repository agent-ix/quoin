// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The evidence gaps behind each post-control estimate (FR-147).
//!
//! A gap is a link or a record that is missing. Whether a control works or a
//! verification passed is recorded nowhere, and is not read.

use std::collections::BTreeSet;

use serde::Serialize;

use crate::finding::{Finding, Token};
use crate::links::Links;
use crate::records::{Estimate, Phase, Records};

/// One control a post-control estimate assumes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ControlEvidence {
    /// The control id.
    pub control: String,
    /// Ids of the requirements that satisfy it.
    pub requirements: Vec<String>,
    /// Ids of the documents that verify it.
    pub verifications: Vec<String>,
    /// The tokens naming this control.
    pub findings: Vec<Finding>,
}

/// An open anomaly whose assessment names something the estimate rests on.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub struct AnomalyRef {
    /// The anomaly id.
    pub anomaly: String,
    /// The assessment row's kind.
    pub kind: String,
    /// The assessment row's impact.
    pub impact: String,
    /// The id the assessment names.
    pub names: String,
}

/// One post-control estimate and what it rests on.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct EvidenceEntry {
    /// The post-control estimate id.
    pub estimate: String,
    /// The estimate's situation, or `null`.
    pub situation: Option<String>,
    /// Ids of the hazards of the estimate.
    pub hazards: Vec<String>,
    /// Each control the estimate assumes.
    pub controls: Vec<ControlEvidence>,
    /// Each open anomaly listed.
    pub anomalies: Vec<AnomalyRef>,
    /// The tokens naming the estimate.
    pub findings: Vec<Finding>,
}

fn exists(records: &Records, id: &str, kind: &str) -> bool {
    records.others.get(id).is_some_and(|doc| {
        if kind == "hazard" {
            doc.hazard
        } else {
            doc.kind == kind
        }
    })
}

/// The controls the estimate assumes that exist as `control` records.
pub(crate) fn assumed_controls<'a>(records: &Records, estimate: &'a Estimate) -> Vec<&'a String> {
    estimate
        .controls
        .iter()
        .filter(|id| exists(records, id, "control"))
        .collect()
}

/// A control with the requirements and verifications that link to it, and the
/// tokens for the links that are missing (FR-147).
pub(crate) fn control_evidence(links: &Links, control: &str) -> ControlEvidence {
    let requirements = links.requirements.get(control).cloned().unwrap_or_default();
    let verifications = links
        .verifications
        .get(control)
        .cloned()
        .unwrap_or_default();
    let mut findings = Vec::new();
    if requirements.is_empty() {
        findings.push(Finding::bare(Token::ControlWithoutRequirement));
    }
    if verifications.is_empty() {
        findings.push(Finding::bare(Token::ControlWithoutVerification));
    }
    ControlEvidence {
        control: control.to_owned(),
        requirements,
        verifications,
        findings,
    }
}

fn entry(records: &Records, links: &Links, estimate: &Estimate) -> EvidenceEntry {
    let situations: Vec<&String> = estimate
        .situations
        .iter()
        .filter(|id| exists(records, id, "hazardous_situation"))
        .collect();
    let mut hazards: Vec<String> = situations
        .iter()
        .filter_map(|id| records.others.get(*id))
        .flat_map(|situation| &situation.arises_from)
        .filter(|id| exists(records, id, "hazard"))
        .cloned()
        .collect();
    hazards.sort();
    hazards.dedup();
    let controls = assumed_controls(records, estimate);

    let mut findings = Vec::new();
    if controls.is_empty() {
        findings.push(Finding::bare(Token::PostControlWithoutControl));
    }
    if situations.is_empty() {
        findings.push(Finding::bare(Token::EstimateWithoutSituation));
    }
    for situation in &situations {
        let arises = records.others.get(*situation).is_some_and(|doc| {
            doc.arises_from
                .iter()
                .any(|id| exists(records, id, "hazard"))
        });
        if !arises {
            findings.push(Finding::naming(
                Token::SituationWithoutHazard,
                vec![(*situation).clone()],
            ));
        }
    }

    let control_entries: Vec<ControlEvidence> = controls
        .iter()
        .map(|control| control_evidence(links, control))
        .collect();

    let mut names: BTreeSet<&str> = hazards.iter().map(String::as_str).collect();
    for control in &control_entries {
        for requirement in &control.requirements {
            names.insert(requirement);
            if let Some(doc) = records.others.get(requirement) {
                names.extend(doc.criteria.iter().map(String::as_str));
            }
        }
    }
    let anomalies: BTreeSet<AnomalyRef> = records
        .anomalies
        .iter()
        .filter(|anomaly| anomaly.status_word != "closed")
        .flat_map(|anomaly| {
            anomaly.assessment.iter().flatten().filter_map(|row| {
                let target = row.target.as_deref()?;
                names.contains(target).then(|| AnomalyRef {
                    anomaly: anomaly.id.clone(),
                    kind: row.kind.clone(),
                    impact: row.impact.clone(),
                    names: target.to_owned(),
                })
            })
        })
        .collect();

    EvidenceEntry {
        estimate: estimate.id.clone(),
        situation: situations.first().map(|id| (*id).clone()),
        hazards,
        controls: control_entries,
        anomalies: anomalies.into_iter().collect(),
        findings,
    }
}

pub(crate) fn gaps(records: &Records, links: &Links) -> Vec<EvidenceEntry> {
    records
        .estimates
        .values()
        .filter(|estimate| estimate.phase == Phase::PostControl)
        .map(|estimate| entry(records, links, estimate))
        .collect()
}
