// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Mocked confirmation: the behaviour the criterion verifies, substituted by
//! the test that verifies it (agent-ix/quoin#204).
//!
//! Pass 2's case: FR-017-AC-7's trusted-UI confirmation had NO implementation,
//! and its test passed by injecting `Confirmation::allow()`. Green test, green
//! AC, absent behaviour.

use std::collections::BTreeSet;

use quoin_evidence::types::{Binding, MockInjection};
use quoin_quire_types::Obligation;

/// How much of an injected identifier must overlap the statement's own words
/// before the injection is read as standing in for the verified behaviour.
///
/// Deliberately high. A test legitimately mocks a clock, a filesystem, a
/// network — all of which share nothing with the statement. What this is
/// looking for is the narrow case where the mock's NAME is the statement's
/// subject, which is what `Confirmation::allow` against a trusted-UI
/// confirmation criterion looks like.
pub const MOCK_SUBJECT_FLOOR: f64 = 0.5;

/// The words every sentence shares, and which therefore say nothing.
const NOISE: [&str; 21] = [
    "the", "a", "an", "and", "or", "of", "to", "for", "in", "is", "be", "shall", "with", "that",
    "it", "its", "on", "by", "as", "not", "new",
];

/// Lowercased word-ish tokens, minus the words every sentence shares.
///
/// `text.split(/[^A-Za-z0-9]+/)`: the separator class is ASCII, so a non-ASCII
/// letter is a separator here and not a word character. Retained behaviour,
/// and the reason this splits on `!char::is_ascii_alphanumeric` rather than on
/// `char::is_alphanumeric`.
#[must_use]
pub fn words(text: &str) -> BTreeSet<String> {
    text.split(|character: char| !character.is_ascii_alphanumeric())
        .map(str::to_lowercase)
        .filter(|token| token.chars().count() > 2 && !NOISE.contains(&token.as_str()))
        .collect()
}

/// Join source inspection to the test result that established the binding.
///
/// Cargo-style result adapters commonly qualify a test as
/// `module::test_name`, while source inspection can only see `test_name`.
/// Accept that exact terminal qualification, but never substring or suite-only
/// matches: a stand-in in one test must not accuse every obligation discharged
/// by a large workspace suite.
#[must_use]
pub fn same_test_symbol(recorded: &str, inspected: &str) -> bool {
    recorded == inspected
        || recorded.ends_with(&format!("::{inspected}"))
        || inspected.ends_with(&format!("::{recorded}"))
}

/// Does this injected identifier name the statement's own subject?
fn injects_the_subject(identifier: &str, subject: &BTreeSet<String>) -> bool {
    let tokens = words(identifier);
    if tokens.is_empty() {
        return false;
    }
    let shared = tokens
        .iter()
        .filter(|token| subject.contains(*token))
        .count();
    // Both counts are small, and the ratio is what the retained code compares.
    #[allow(
        clippy::cast_precision_loss,
        reason = "token counts are bounded by an identifier's length; f64 is exact \
                  far past any value either side can take"
    )]
    let ratio = shared as f64 / tokens.len() as f64;
    ratio >= MOCK_SUBJECT_FLOOR
}

/// Bindings whose suite injects a stand-in for this obligation's subject.
///
/// One entry per binding that has one, in binding order — the retained
/// `for…of` over the sorted bindings, keeping the FIRST injection that
/// qualifies for each.
#[must_use]
pub fn mocked_bindings<'a>(
    obligation: &Obligation,
    bindings: &[&Binding],
    injections: &'a [MockInjection],
) -> Vec<&'a MockInjection> {
    if injections.is_empty() {
        return Vec::new();
    }
    let subject = words(&obligation.statement);
    if subject.is_empty() {
        return Vec::new();
    }

    let mut found = Vec::new();
    for binding in bindings {
        let hit = injections
            .iter()
            .filter(|injection| {
                injection.suite.as_str() == binding.suite.as_str()
                    && binding
                        .symbols
                        .iter()
                        .any(|symbol| same_test_symbol(symbol.as_str(), injection.symbol.as_str()))
            })
            .find(|injection| {
                injection
                    .injects
                    .iter()
                    .any(|identifier| injects_the_subject(identifier, &subject))
            });
        if let Some(hit) = hit {
            found.push(hit);
        }
    }
    found
}

/// The `mocked-confirmation` finding, once every binding is mocked.
///
/// Reported only when EVERY binding is mocked. One suite standing in a
/// dependency while another exercises the real path is ordinary test design,
/// and flagging it would fire on most of the corpus for a reason that has
/// nothing to do with this defect.
///
/// `medium`, not `high`: this is a heuristic over identifiers, and a
/// legitimate mock can share a noun with the statement it appears under.
#[must_use]
pub fn mocked_finding(
    obligation: &Obligation,
    mocked: &[&MockInjection],
) -> Option<quoin_finding_types::Finding> {
    use quoin_combinatorial::js;
    use quoin_finding_types::{Finding, FindingKind, Severity};

    let mut ordered: Vec<&&MockInjection> = mocked.iter().collect();
    ordered.sort_by(|left, right| {
        js::compare(
            left.path.as_deref().unwrap_or_default(),
            right.path.as_deref().unwrap_or_default(),
        )
        .then_with(|| left.line.unwrap_or(0).cmp(&right.line.unwrap_or(0)))
        .then_with(|| js::compare(left.symbol.as_str(), right.symbol.as_str()))
    });
    let located = *ordered.first()?;

    // `located.path ? … : {}` and `located.line ? … : {}` — the empty string
    // and zero are falsy, so neither is written as a key.
    let path = located.path.as_deref().filter(|path| !path.is_empty());
    let line = located.line.filter(|line| *line != 0);
    let symbol = located.symbol.as_str();
    let change_target = match (path, line) {
        (Some(path), Some(line)) => format!("{path}:{line} in {symbol}"),
        (Some(path), None) => format!("{path} in {symbol}"),
        (None, _) => symbol.to_owned(),
    };

    let mut injects: Vec<String> = mocked
        .iter()
        .map(|injection| {
            let locus = match (
                injection.path.as_deref().filter(|path| !path.is_empty()),
                injection.line.filter(|line| *line != 0),
            ) {
                (Some(path), Some(line)) => format!("{path}:{line} "),
                (Some(path), None) => format!("{path} "),
                (None, _) => String::new(),
            };
            format!(
                "{locus}{symbol} injects {what}",
                symbol = injection.symbol,
                what = injection.injects.join(", ")
            )
        })
        .collect();
    injects.sort_by(|left, right| js::compare(left, right));

    let mut finding = Finding::new(
        FindingKind::MOCKED_CONFIRMATION,
        &obligation.id,
        Severity::medium(),
        format!(
            "{id} is discharged only by tests that inject a stand-in for the behaviour it \
             verifies: {injects}. A test that substitutes the behaviour under verification \
             passes whether or not that behaviour exists.",
            id = obligation.id,
            injects = injects.join("; ")
        ),
    );
    finding.path = path.map(ToOwned::to_owned);
    finding.line = line;
    finding.symbol = Some(symbol.to_owned());
    finding.subject = Some(format!("{} evidence", obligation.id));
    finding.change_target = Some(change_target);
    finding.next_diagnostic_step = Some(format!(
        "inspect whether the real behaviour for {id} exists; add or bind an independent test \
         that exercises it without substituting the behaviour under verification",
        id = obligation.id
    ));
    Some(finding)
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]
mod tests {
    use super::{same_test_symbol, words};

    #[test]
    fn short_words_and_noise_words_are_dropped() {
        let found = words("The user shall be shown a Confirmation dialog");
        assert!(found.contains("confirmation"));
        assert!(found.contains("user"));
        assert!(found.contains("dialog"));
        assert!(!found.contains("the"), "noise word");
        assert!(!found.contains("be"), "two letters");
        assert!(!found.contains("shall"), "noise word");
    }

    #[test]
    fn a_non_ascii_letter_separates_words_as_the_retained_split_does() {
        // `/[^A-Za-z0-9]+/` is an ASCII class: `caf\u{e9}bar` splits in two.
        let found = words("caf\u{e9}bar");
        assert!(found.contains("caf"));
        assert!(found.contains("bar"));
    }

    #[test]
    fn a_terminal_qualification_joins_and_a_substring_does_not() {
        assert!(same_test_symbol("module::test_name", "test_name"));
        assert!(same_test_symbol("test_name", "module::test_name"));
        assert!(same_test_symbol("test_name", "test_name"));
        assert!(
            !same_test_symbol("module::test_name_longer", "test_name"),
            "a substring must not accuse a whole suite"
        );
        assert!(!same_test_symbol("a::b", "c::d"));
    }
}
