// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Map retained test evidence to an in-toto Test Result statement (FR-185).
//!
//! This module only transforms caller-provided values. It performs no store,
//! filesystem, Git, process, network, or signing operation.

use std::collections::{BTreeMap, BTreeSet};

use ix_trace_rs::implements;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::stale::failed_bound_symbols;
use crate::types::{Binding, Outcome, RunRecord};

/// The in-toto Statement v1 type identifier.
pub const STATEMENT_TYPE: &str = "https://in-toto.io/Statement/v1";
/// The in-toto Test Result v0.1 predicate identifier.
pub const PREDICATE_TYPE: &str = "https://in-toto.io/attestation/test-result/v0.1";

/// A closed in-toto statement containing the caller's subject and test result.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TestResultStatement {
    /// Statement version.
    #[serde(rename = "_type")]
    pub statement_type: String,
    /// Resource descriptors supplied by the caller, preserved structurally.
    pub subject: Vec<Value>,
    /// Predicate schema identifier.
    #[serde(rename = "predicateType")]
    pub predicate_type: String,
    /// Test Result predicate.
    pub predicate: TestResultPredicate,
}

/// Closed Test Result predicate.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TestResultPredicate {
    /// Overall result derived from the three test lists.
    pub result: TestResult,
    /// Suite configurations used to classify obligations.
    pub configuration: Vec<TestConfiguration>,
    /// Obligation ids whose bound symbols all passed.
    #[serde(rename = "passedTests")]
    pub passed_tests: Vec<String>,
    /// Obligation ids with skipped or absent bound symbols.
    #[serde(rename = "warnedTests")]
    pub warned_tests: Vec<String>,
    /// Stale or failing obligation ids.
    #[serde(rename = "failedTests")]
    pub failed_tests: Vec<String>,
}

/// Test Result predicate verdict.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum TestResult {
    /// No obligations were warned or failed.
    Passed,
    /// At least one obligation has incomplete evidence, and none failed.
    Warned,
    /// At least one obligation failed or is stale.
    Failed,
}

/// One suite entry in `configuration`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TestConfiguration {
    /// Suite identifier.
    pub name: String,
    /// Producer tool and record timestamp.
    pub annotations: TestAnnotations,
}

/// Annotations from the selected run record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TestAnnotations {
    /// RunRecord.tool.
    pub tool: String,
    /// RunRecord.timestamp.
    pub timestamp: String,
}

/// Why a caller-supplied subject cannot be used for a statement.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum SubjectError {
    /// Subject was not a non-empty array.
    #[error("subject must be a non-empty array")]
    EmptyOrNotArray,
    /// A descriptor lacked the required sha256 value.
    #[error("every subject descriptor must contain digest.sha256")]
    MissingSha256,
    /// A sha256 value was not 64 lowercase hexadecimal characters.
    #[error("digest.sha256 must contain exactly 64 lowercase hexadecimal characters")]
    InvalidSha256,
}

/// Build the FR-185 statement from validated subject, bindings and latest runs.
///
/// The caller supplies exactly one latest run per suite. The evidence store's
/// latest-run reader orders records by lexical timestamp and stable path order.
///
/// # Errors
/// [`SubjectError`] when the subject is not a non-empty array of resource
/// descriptors with valid `digest.sha256` members.
#[implements("FR-185")]
pub fn build_statement(
    subject: Value,
    bindings: &[Binding],
    latest_runs: &[RunRecord],
) -> Result<TestResultStatement, SubjectError> {
    let subject = validate_subject(subject)?;
    let runs: BTreeMap<&str, &RunRecord> = latest_runs
        .iter()
        .map(|run| (run.suite.as_str(), run))
        .collect();

    let mut configuration: BTreeMap<&str, TestConfiguration> = BTreeMap::new();
    for binding in bindings {
        if let Some(run) = runs.get(binding.suite.as_str()) {
            configuration
                .entry(binding.suite.as_str())
                .or_insert_with(|| TestConfiguration {
                    name: binding.suite.as_str().to_owned(),
                    annotations: TestAnnotations {
                        tool: run.tool.clone(),
                        timestamp: run.timestamp.clone(),
                    },
                });
        }
    }

    let mut passed = BTreeSet::new();
    let mut warned = BTreeSet::new();
    let mut failed = BTreeSet::new();
    let mut obligations: BTreeMap<&str, Vec<&Binding>> = BTreeMap::new();
    for binding in bindings {
        obligations
            .entry(binding.obligation.as_str())
            .or_default()
            .push(binding);
    }

    for (obligation, obligation_bindings) in obligations {
        let mut is_failed = false;
        let mut is_warned = false;
        for binding in obligation_bindings {
            let Some(run) = runs.get(binding.suite.as_str()) else {
                is_failed = true;
                break;
            };
            if !failed_bound_symbols(binding, run).is_empty() {
                is_failed = true;
                break;
            }
            for symbol in &binding.symbols {
                let mut found = false;
                for entry in run.entries.iter().filter(|entry| entry.symbol == *symbol) {
                    found = true;
                    if entry.outcome == Outcome::Skip {
                        is_warned = true;
                    }
                }
                if !found {
                    is_warned = true;
                }
            }
        }
        if is_failed {
            failed.insert(obligation.to_owned());
        } else if is_warned {
            warned.insert(obligation.to_owned());
        } else {
            passed.insert(obligation.to_owned());
        }
    }

    let passed_tests: Vec<String> = passed.into_iter().collect();
    let warned_tests: Vec<String> = warned.into_iter().collect();
    let failed_tests: Vec<String> = failed.into_iter().collect();
    let result = if !failed_tests.is_empty() {
        TestResult::Failed
    } else if !warned_tests.is_empty() {
        TestResult::Warned
    } else {
        TestResult::Passed
    };

    Ok(TestResultStatement {
        statement_type: STATEMENT_TYPE.to_owned(),
        subject,
        predicate_type: PREDICATE_TYPE.to_owned(),
        predicate: TestResultPredicate {
            result,
            configuration: configuration.into_values().collect(),
            passed_tests,
            warned_tests,
            failed_tests,
        },
    })
}

fn validate_subject(subject: Value) -> Result<Vec<Value>, SubjectError> {
    let Value::Array(descriptors) = subject else {
        return Err(SubjectError::EmptyOrNotArray);
    };
    if descriptors.is_empty() {
        return Err(SubjectError::EmptyOrNotArray);
    }
    for descriptor in &descriptors {
        let sha256 = descriptor
            .get("digest")
            .and_then(|digest| digest.get("sha256"))
            .and_then(Value::as_str)
            .ok_or(SubjectError::MissingSha256)?;
        if sha256.len() != 64
            || !sha256
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        {
            return Err(SubjectError::InvalidSha256);
        }
    }
    Ok(descriptors)
}
