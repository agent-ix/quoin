// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Per-class evidence floors (FR-160).
//!
//! A person states a software safety class on a requirement. A person writes a
//! policy file saying what evidence each class value needs. This module reads
//! both and reports where the evidence falls short. It holds no class value, no
//! default policy and no floor: with no [`ClassPolicy`] in the input, nothing
//! here runs and the report gains nothing (FR-160-AC-1, FR-160-AC-13).
//!
//! The class is read from the Quire payload exactly as written, under the
//! carried name the policy declares. It is never derived from criticality, from
//! an allocated item or from a neighbouring obligation (FR-160-CON-1).

use std::collections::{BTreeMap, BTreeSet};

use quoin_evidence::independence::assess_lines;
use quoin_evidence::types::{
    Binding, IndependenceDimension, IndependenceDimensionAssessment, IndependenceStatus,
};
use quoin_evidence::{ObligationId, SuiteId};
use quoin_finding_types::{Finding, FindingKind, Severity};
use quoin_quire_types::{Obligation, requirement_of};
use serde::{Deserialize, Serialize};

use super::{AuditInput, Indexed};
use crate::error::{AuditorError, AuditorErrorCode};
use crate::jsvalue::number_text;

/// The `source` a class independence finding and assessment carry.
const SOURCE_CLASS: &str = "class";
/// The `source` a profile independence finding carries once a class policy is
/// in force.
pub(super) const SOURCE_PROFILE: &str = "profile";

/// The class policy file: a person's recorded decision about what evidence each
/// software safety class value needs.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct ClassPolicy {
    /// The carried name (quire-rs FR-090) the class is read under.
    pub class_field: String,
    /// The floors, keyed by class value as the requirement states it.
    #[serde(default)]
    pub classes: BTreeMap<String, ClassFloors>,
}

/// What one class value demands. Every floor is optional; none has a built-in
/// value.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct ClassFloors {
    /// Catalog method ids. Each needs bound evidence of the method's catalog
    /// `evidence_kind`.
    #[serde(default)]
    pub required_methods: Vec<String>,
    /// The mutation ratio, in `[0, 1]`, the FR-039 rule evaluates.
    #[serde(default)]
    pub mutation_floor: Option<f64>,
    /// FR-094 dimensions two evidence lines must differ on.
    #[serde(default)]
    pub independence: Vec<String>,
    /// The per-requirement activities FR-161 reports. FR-160 reads none of
    /// them; the key is accepted so one policy file serves both commands.
    #[serde(default)]
    pub obligations: Vec<serde_json::Value>,
    /// Why this class demands this. A recorded decision has a reason.
    pub rationale: String,
}

/// One obligation's class independence assessment (FR-160), the sibling of the
/// profile [`IndependenceAssessment`](quoin_evidence::types::IndependenceAssessment)
/// that can carry a class.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClassIndependenceAssessment {
    /// Always `class`.
    pub source: &'static str,
    /// The class value the obligation's requirement states.
    pub class: String,
    /// The requirement id the obligation derives from.
    pub requirement: String,
    /// The obligation assessed.
    pub obligation: ObligationId,
    /// The verdict.
    pub status: IndependenceStatus,
    /// One entry per listed dimension, in the listed order.
    pub dimensions: Vec<IndependenceDimensionAssessment>,
    /// The two suites that satisfied it, when one pair did.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub satisfied_by: Option<(SuiteId, SuiteId)>,
    /// A sentence naming what was found or what was missing.
    pub summary: String,
}

/// A validated policy joined to the classes the payload states.
pub(super) struct ClassContext<'a> {
    policy: &'a ClassPolicy,
    /// Obligation id → class value as written.
    assigned: BTreeMap<&'a str, String>,
    /// Required method id → the catalog's `evidence_kind` for it.
    kinds: BTreeMap<&'a str, &'a str>,
    /// Class value → its parsed independence dimensions.
    dimensions: BTreeMap<&'a str, Vec<IndependenceDimension>>,
}

impl<'a> ClassContext<'a> {
    /// Validate the policy and join it to the payload's classes.
    ///
    /// `Ok(None)` when the input carries no policy: the audit then adds nothing.
    ///
    /// # Errors
    ///
    /// [`AuditorErrorCode::ClassPolicyRefused`] naming every cause, when the
    /// policy names a class absent from the report's vocabulary, a method the
    /// catalog does not declare, a ratio outside `[0, 1]`, an unknown dimension
    /// or an empty rationale, or when the report carries no vocabulary for the
    /// class field. A floor that silently does not apply reads as a passing
    /// gate, so nothing is audited.
    pub(super) fn prepare(input: &'a AuditInput) -> Result<Option<Self>, AuditorError> {
        let Some(policy) = input.class_policy.as_ref() else {
            return Ok(None);
        };
        let mut causes: Vec<String> = Vec::new();
        let field = policy.class_field.as_str();
        let vocabulary = input
            .carried_vocabulary
            .as_ref()
            .and_then(|vocabulary| vocabulary.get(field));
        if field.trim().is_empty() {
            causes.push("class_field: must not be empty".to_owned());
        } else if vocabulary.is_none() {
            causes.push(format!(
                "the coverage report carries no carried_vocabulary for `{field}`, so nothing says which classes exist"
            ));
        }

        let mut kinds = BTreeMap::new();
        let mut dimensions = BTreeMap::new();
        for (class, floors) in &policy.classes {
            if let Some(vocabulary) = vocabulary
                && !vocabulary.iter().any(|value| value == class)
            {
                causes.push(format!(
                    "class `{class}` is not in the carried_vocabulary for `{field}`"
                ));
            }
            if floors.rationale.trim().is_empty() {
                causes.push(format!("class `{class}`: rationale must not be empty"));
            }
            if let Some(ratio) = floors.mutation_floor
                && !(0.0..=1.0).contains(&ratio)
            {
                causes.push(format!(
                    "class `{class}`: mutation_floor must be a ratio in [0, 1], got {}. 80% is 0.8, not 80.",
                    number_text(ratio)
                ));
            }
            for method in &floors.required_methods {
                match declared_kind(input, method) {
                    Some(kind) => {
                        kinds.insert(method.as_str(), kind);
                    }
                    None => causes.push(format!(
                        "class `{class}`: required method `{method}` is not a catalog method that declares an evidence_kind"
                    )),
                }
            }
            let mut parsed = Vec::new();
            for name in &floors.independence {
                match IndependenceDimension::ALL
                    .into_iter()
                    .find(|dimension| dimension.as_str() == name)
                {
                    Some(dimension) => parsed.push(dimension),
                    None => causes.push(format!(
                        "class `{class}`: `{name}` is not an independence dimension"
                    )),
                }
            }
            dimensions.insert(class.as_str(), parsed);
        }
        if !causes.is_empty() {
            return Err(AuditorError::new(
                AuditorErrorCode::ClassPolicyRefused,
                format!("class policy refused: {}", causes.join("; ")),
            ));
        }

        let mut assigned = BTreeMap::new();
        for entry in input.coverage_matrix.iter().flatten() {
            let Some(value) = entry
                .carried
                .as_ref()
                .and_then(|carried| carried.get(field))
            else {
                continue;
            };
            // As written: a string is its own text, anything else its JSON.
            let class = value
                .as_str()
                .map_or_else(|| value.to_string(), ToOwned::to_owned);
            for criterion in &entry.criteria {
                assigned.insert(criterion.id.as_str(), class.clone());
            }
        }
        Ok(Some(Self {
            policy,
            assigned,
            kinds,
            dimensions,
        }))
    }

    /// The class the obligation's requirement states, if it states one.
    pub(super) fn class_of(&self, obligation: &str) -> Option<&str> {
        self.assigned.get(obligation).map(String::as_str)
    }

    /// The class's mutation ratio, with the class it came from.
    pub(super) fn mutation_floor(&self, obligation: &str) -> Option<(&str, f64)> {
        let class = self.class_of(obligation)?;
        let ratio = self.policy.classes.get(class)?.mutation_floor?;
        Some((class, ratio))
    }

    /// `class-unconfigured`, or one `class-method-missing` per unmet method.
    ///
    /// Asked whatever the ladder found: an obligation with no evidence at all
    /// is also missing the evidence its class requires, and both are true.
    pub(super) fn evidence_findings(
        &self,
        indexed: &Indexed<'_>,
        obligation: &Obligation,
        bindings: &[&Binding],
    ) -> Vec<Finding> {
        let Some(class) = self.class_of(&obligation.id) else {
            return Vec::new();
        };
        let Some(floors) = self.policy.classes.get(class) else {
            return vec![Finding::new(
                FindingKind::CLASS_UNCONFIGURED,
                &obligation.id,
                Severity::medium(),
                format!(
                    "{id} is software safety class {class}, which the class policy does not \
                     configure; no class floor applies to it.",
                    id = obligation.id
                ),
            )];
        };
        let bound_kinds: BTreeSet<&str> = bindings
            .iter()
            .flat_map(|binding| {
                let suite = binding.suite.as_str();
                let run = indexed
                    .runs_by_suite
                    .get(suite)
                    .and_then(|run| run.evidence_kind.as_deref());
                let scan = indexed
                    .scans_by_suite
                    .get(suite)
                    .and_then(|scan| scan.evidence_kind.as_deref());
                [run, scan].into_iter().flatten()
            })
            .collect();
        floors
            .required_methods
            .iter()
            .filter_map(|method| {
                let kind = *self.kinds.get(method.as_str())?;
                if bound_kinds.contains(kind) {
                    return None;
                }
                Some(Finding::new(
                    FindingKind::CLASS_METHOD_MISSING,
                    &obligation.id,
                    Severity::medium(),
                    format!(
                        "{id} is class {class}, whose policy requires method `{method}` \
                         (evidence kind {kind}), and no evidence of that kind is bound to it.",
                        id = obligation.id
                    ),
                ))
            })
            .collect()
    }

    /// The class independence assessment for an obligation whose class lists
    /// dimensions, or `None` when its class lists none.
    pub(super) fn independence(
        &self,
        obligation: &Obligation,
        bindings: &[&Binding],
    ) -> Option<ClassIndependenceAssessment> {
        let class = self.class_of(&obligation.id)?;
        let dimensions = self.dimensions.get(class).filter(|list| !list.is_empty())?;
        let requirement = requirement_of(&obligation.id);
        let lines = assess_lines(requirement, dimensions, bindings);
        Some(ClassIndependenceAssessment {
            source: SOURCE_CLASS,
            class: class.to_owned(),
            requirement: requirement.to_owned(),
            obligation: ObligationId::new(&obligation.id),
            status: lines.status,
            summary: format!(
                "{id} is class {class}: {summary}{shortfall}",
                id = obligation.id,
                summary = lines.summary,
                shortfall = shortfall(&lines.dimensions, bindings.len(), lines.status),
            ),
            dimensions: lines.dimensions,
            satisfied_by: lines.satisfied_by,
        })
    }

    /// The `classReport` member: each classified obligation's class as written,
    /// and the ids of the obligations whose requirement states none.
    pub(super) fn report(&self, obligations: &[&Obligation]) -> serde_json::Value {
        let mut classes = serde_json::Map::new();
        let mut unassigned: Vec<&str> = Vec::new();
        for obligation in obligations {
            match self.class_of(&obligation.id) {
                Some(class) => {
                    classes.insert(obligation.id.clone(), serde_json::json!(class));
                }
                None => unassigned.push(obligation.id.as_str()),
            }
        }
        serde_json::json!({ "classes": classes, "unassigned": unassigned })
    }
}

/// The finding for an insufficient class independence assessment.
pub(super) fn independence_finding(
    obligation: &Obligation,
    assessment: &ClassIndependenceAssessment,
) -> Finding {
    let mut finding = Finding::new(
        FindingKind::INSUFFICIENT_INDEPENDENCE,
        &obligation.id,
        Severity::medium(),
        assessment.summary.clone(),
    );
    finding
        .other
        .insert("source".to_owned(), serde_json::json!(SOURCE_CLASS));
    finding
        .other
        .insert("class".to_owned(), serde_json::json!(assessment.class));
    finding
}

/// The catalog `evidence_kind` of the method with this exact id.
fn declared_kind<'a>(input: &'a AuditInput, method: &str) -> Option<&'a str> {
    input
        .catalog
        .as_ref()?
        .methods
        .iter()
        .find(|entry| entry.id == method)?
        .evidence_kind
        .as_deref()
        .filter(|kind| !kind.is_empty())
}

/// The dimensions still shared or missing, named for a reader.
fn shortfall(
    dimensions: &[IndependenceDimensionAssessment],
    lines: usize,
    status: IndependenceStatus,
) -> String {
    if status == IndependenceStatus::Satisfied {
        return String::new();
    }
    if lines < 2 {
        return " Fewer than two evidence lines are bound.".to_owned();
    }
    let named: Vec<String> = dimensions
        .iter()
        .filter_map(|item| {
            if !item.missing_suites.is_empty() {
                return Some(format!("{} (missing on a line)", item.dimension));
            }
            (item.values.len() < lines).then(|| format!("{} (shared)", item.dimension))
        })
        .collect();
    if named.is_empty() {
        " No single pair of lines differs on every dimension at once.".to_owned()
    } else {
        format!(" Still shared or missing: {}.", named.join(", "))
    }
}
