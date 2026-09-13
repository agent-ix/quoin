// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! The cross-member rules a graph-quality observation must satisfy.
//!
//! Ports the `.superRefine` on `graphQualityObservationSchema`
//! (`src/measurement/graph-adapters.ts:317-363`), plus the four `.min(n)` array
//! floors serde's derive cannot express.
//!
//! # Why these are not in the types
//!
//! Everything a type can carry is carried by one: a bounded integer is a
//! [`std::num::NonZeroU64`], a bounded string is a newtype, a closed set is an
//! enum. What is left here is exactly what relates **two** members — a
//! population state and the presence of results — which no single field can
//! hold. Keeping them in one place means the rule set is a list a reader can
//! count, rather than conditions scattered through a constructor.
//!
//! Every rule is collected, not short-circuited, because the retained
//! `superRefine` collects: a caller that fixes the first refusal should not
//! then discover the second.

use super::GraphQualityObservationV1;
use super::population::PopulationState;

/// Check every rule, returning all failures in the order the retained
/// refinement raises them.
pub(crate) fn violations(record: &GraphQualityObservationV1) -> Vec<String> {
    let mut found: Vec<String> = Vec::new();
    let mut note = |detail: String| found.push(detail);

    if record.producer.parser_grammars.is_empty() {
        note("producer.parser_grammars: expected at least 1, got 0".to_owned());
    }
    if let Some(results) = record.results.as_ref()
        && let Err(error) = results.check_floors()
    {
        note(format!("results.{error}"));
    }

    let population = &record.population;
    if population.state == PopulationState::Measured {
        if record.results.is_none() {
            note("results: measured population requires results".to_owned());
        }
        if population.supported_files < 1 || population.unreadable_files != 0 {
            note(
                "population: measured population requires supported files and no unreadable files"
                    .to_owned(),
            );
        }
    } else if record.results.is_some() {
        note(format!(
            "results: {} population must omit results",
            population.state.as_str()
        ));
    }

    if population.state == PopulationState::Empty
        && (population.files_seen != 0
            || population.supported_files != 0
            || population.unreadable_files != 0
            || population.unsupported_files != 0)
    {
        note("population: empty population requires all file counts to be zero".to_owned());
    }
    if population.state == PopulationState::Unreadable && population.unreadable_files < 1 {
        note(
            "population.unreadable_files: unreadable population requires at least one unreadable \
             file"
                .to_owned(),
        );
    }
    if population.state == PopulationState::Unsupported
        && (population.files_seen < 1
            || population.supported_files != 0
            || population.unreadable_files != 0
            || population.unsupported_files < 1)
    {
        note("population: unsupported population has inconsistent file counts".to_owned());
    }
    found
}
