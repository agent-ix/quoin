// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The uncatalogued-method join, read off a coverage payload's diagnostics.

use std::collections::BTreeSet;

use quoin_quire_types::CoverageDiagnostic;

/// The diagnostic reason quire mints for a method the catalog never declared.
pub const UNCATALOGUED_METHOD_REASON: &str = "uncatalogued-verification-method";

/// What the coverage payload's diagnostics say about authored-method vocabulary.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct UncataloguedMethods {
    /// Authored methods the engine diagnosed as uncatalogued — each `value`
    /// byte-equal to the `Obligation.method` it is about (quire-rs CR-091).
    pub values: BTreeSet<String>,
    /// True when a diagnostic carried the reason but no `value`: the engine
    /// predates the CR-091 classification, so uncatalogued values cannot be
    /// told from genuine disagreements. The caller must degrade to the
    /// two-state behaviour and say so, rather than misclassify.
    pub degraded: bool,
}

/// Read the join off a coverage payload's diagnostics.
///
/// No diagnostics, or none with the reason, yields an empty set with
/// `degraded: false`: under an engine of either vintage that means every
/// authored value is catalogued, and the three-state split degenerates to the
/// two-state behaviour *correctly* — nothing was there to classify.
#[must_use]
pub fn uncatalogued_authored_methods(diagnostics: &[CoverageDiagnostic]) -> UncataloguedMethods {
    let mut found = UncataloguedMethods::default();
    for diagnostic in diagnostics {
        if diagnostic.reason != UNCATALOGUED_METHOD_REASON {
            continue;
        }
        match &diagnostic.value {
            None => found.degraded = true,
            Some(value) => {
                found.values.insert(value.clone());
            }
        }
    }
    found
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]
mod tests {
    use quoin_quire_types::CoverageDiagnostic;

    use super::{UNCATALOGUED_METHOD_REASON, uncatalogued_authored_methods};

    fn diagnostic(reason: &str, value: Option<&str>) -> CoverageDiagnostic {
        CoverageDiagnostic {
            reason: reason.to_owned(),
            value: value.map(ToOwned::to_owned),
        }
    }

    #[test]
    fn only_the_uncatalogued_reason_is_read() {
        let found = uncatalogued_authored_methods(&[
            diagnostic(UNCATALOGUED_METHOD_REASON, Some("Smoke")),
            diagnostic("some-other-reason", Some("Ignored")),
        ]);
        assert_eq!(found.values.len(), 1);
        assert!(found.values.contains("Smoke"));
        assert!(!found.degraded);
    }

    #[test]
    fn a_valueless_diagnostic_degrades_rather_than_misclassifies() {
        let found = uncatalogued_authored_methods(&[diagnostic(UNCATALOGUED_METHOD_REASON, None)]);
        assert!(found.degraded);
        assert!(found.values.is_empty());
    }

    #[test]
    fn no_diagnostics_is_not_degraded() {
        let found = uncatalogued_authored_methods(&[]);
        assert!(!found.degraded);
        assert!(found.values.is_empty());
    }
}
