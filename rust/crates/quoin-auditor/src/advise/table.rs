// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The lexical characteristic table.
//!
//! A port of `STATEMENT_CHARACTERISTICS` in `src/advisor/advise.ts:155`, entry
//! for entry and **in order** — the order decides nothing about the result
//! (`characteristicsOf` sorts) but it decides which alternative a regex
//! engine tries first, and [`crate::advise::compound`] reads match extents.
//!
//! Deliberately small and lexical. Every entry is a phrase whose presence is a
//! fact about the text, not an inference about intent — the CR-014 lesson,
//! where an open set whose membership had to be *judged* reached ~13%
//! precision.
//!
//! # Why every pattern carries `(?i-u)`
//!
//! The retained patterns are JavaScript `RegExp`s with the `i` flag and
//! **without** `u`. In that mode `\b`, `\w` and `\d` are ASCII and case
//! folding never maps a non-ASCII code unit onto an ASCII one. Rust's `regex`
//! resolves all three over Unicode by default, so `\b` would fire between a
//! letter and a combining mark where JavaScript sees none. `(?i-u)` restores
//! the retained semantics exactly; it is not an optimisation and must not be
//! dropped.
//!
//! Three entries cannot take the flag over their whole pattern, because they
//! mix ASCII classes with `\s` (which JavaScript resolves over Unicode even
//! without `u`) or with the literal `≤`/`≥`. Those are assembled below with
//! `(?i-u:…)` around the ASCII parts only and
//! [`quoin_combinatorial::js::JS_WHITESPACE_CLASS`] in place of `\s`, which
//! leaves every match extent identical.

use std::sync::LazyLock;

use quoin_combinatorial::js::JS_WHITESPACE_CLASS;
use regex::Regex;

/// One row of the table: the characteristic it mints, and what it reads.
pub struct StatementCharacteristic {
    /// The characteristic value the catalog matches against.
    pub name: &'static str,
    /// The pattern, in retained order.
    pub pattern: Regex,
}

/// Every lexical characteristic, in the retained order.
pub static STATEMENT_CHARACTERISTICS: LazyLock<Vec<StatementCharacteristic>> =
    LazyLock::new(build_table);

/// The ASCII-only entries, verbatim from the retained table.
///
/// `(?i-u)` is prefixed to each by [`build_table`] rather than written out
/// fifty times, so a reader can diff this list against the TypeScript
/// line by line.
const ASCII_PATTERNS: &[(&str, &str)] = &[
    (
        "temporal",
        r"\b(always|never|eventually|while|until|continuously)\b",
    ),
    ("liveness", r"\b(eventually|makes progress|terminates)\b"),
    (
        "invariance",
        r"\b(invariant|holds for every|at all times)\b",
    ),
    // `thread([- ]safe(ty)?)?` so `thread-safety` matches as a WHOLE compound —
    // under the compound-token guard a bare `thread` inside it would be
    // rejected as a fragment, and thread safety is a concurrency property.
    (
        "concurrent",
        r"\b(concurrent|parallel|race|thread([- ]safe(ty)?)?|simultaneous)\b",
    ),
    (
        "reliability",
        r"\b(tolerat|degrad|retry|failover|resilien|recover)",
    ),
    ("throughput", r"\b(throughput|per second|requests/s|rps)\b"),
    (
        "security",
        r"\b(authenticat|authoriz|permission|credential|secret|token|attack|exploit)",
    ),
    (
        "untrusted-input",
        r"\b(untrusted|user-supplied|external input|malformed)\b",
    ),
    (
        "input-validation",
        r"\b(reject|validate|malformed|invalid input)\b",
    ),
    ("parser", r"\b(pars|deserializ|decode|lex)"),
    (
        "configuration-matrix",
        r"\b(configuration|feature flag|combination of|matrix of)\b",
    ),
    (
        "third-party-dependency",
        r"\b(dependenc|third-party|vendored|licence|license)\b",
    ),
    // Widened from `depend on|layering|must not import` after the battle test
    // (agent-ix/quoin#167): each added phrase is read off a real architectural
    // statement in that corpus, not invented to fit (the CR-014 line).
    (
        "layering",
        r"\b(depend(s|ing)? (back )?(on|upon)|layering|layered architecture|acyclic|dependency (cycle|check)s?|(zero|no) dependenc(y|ies) on|(must|does|do|shall) not (import|depend))\b",
    ),
    // Distinct from `layering`, which is directional. This is about the surface
    // itself: what is exported, what is internal, what may cross.
    (
        "module-boundary",
        r"\b((module|crate) boundar(y|ies)|top-level (module|crate)s?|(remains?|stays?) absent from|public (api|interface|surface)|encapsulat|internal(s)? (of|to)|exported? (from|by))\b",
    ),
    (
        "user-visible",
        r"\b(user|operator|the UI|displays|screen)\b",
    ),
    (
        "stable-output",
        r"\b(byte-identical|identical output|serializ|snapshot)\b",
    ),
    ("agent-behaviour", r"\b(agent|transcript|prompt)\b"),
    (
        "no-executable-oracle",
        r"\b(review|judgement|readable|documented)\b",
    ),
    // ── agent-ix/quoin#128 ───────────────────────────────────────────────────
    //
    // The catalog declared 60 characteristics and this table produced 20, so 40
    // were asked for and never minted. Only values whose fact genuinely lives
    // IN THE SENTENCE are added here.
    (
        "complexity",
        r"\b(complexity|cyclomatic|nesting depth|deeply nested)\b",
    ),
    (
        "consistency",
        r"\b(consistent|consistency|contradict|mutually exclusive)",
    ),
    (
        "cross-component",
        r"\b(cross-component|between (two |the )?(components|services|processes)|across (the|a|its) [a-z-]* ?boundary|two or more components|end-to-end)\b",
    ),
    (
        "deserializer",
        r"\b(deserializ|unmarshal|decoder|from_json|from_yaml)",
    ),
    (
        "distributed",
        r"\b(distributed|cluster|replica|consensus|quorum|multi-node|across nodes)\b",
    ),
    (
        "fault-tolerance",
        r"\b(fault[- ]toleran|tolerates? (a |an )?fail|survives? [a-z ]*fail|crash recovery)",
    ),
    (
        "feature-flags",
        r"\b(feature flag|feature toggle|cargo feature|--features?\b|opt-in flag)\b",
    ),
    (
        "injection-risk",
        r"\b(injection|\bXSS\b|\bSQL\b|shell escape|escap(e|ing) [a-z]* ?input)\b",
    ),
    (
        "io-boundary",
        r"\b(filesystem|file system|socket|database|to disk|from disk|over the network|subprocess|stdin|stdout)\b",
    ),
    (
        "licence",
        r"\b(licence|license|SPDX|copyleft|allow-?list of licen)",
    ),
    (
        "maintainability",
        r"\b(maintainab|technical debt|code smell|dead code|duplicat(e|ion) of)",
    ),
    // `memory[- ]safe(ty)?` names the concept as a whole token. `unsafe` stays
    // a bare word: the compound-token guard keeps it out of `unsafe-audit`.
    (
        "memory-safety",
        r"\b(memory[- ]safe(ty)?|use[- ]after[- ]free|buffer overflow|double free|dangling pointer|unsafe\b)",
    ),
    (
        "network-exposed",
        r"\b(endpoint|publicly (reachable|exposed|accessible)|listens? on|inbound request|remote client|HTTP request)\b",
    ),
    (
        "non-deterministic-subject",
        r"\b(non-?deterministic|stochastic|temperature|sampled output|model output|\bLLM\b)\b",
    ),
    (
        "precondition-bearing",
        r"\b(precondition|postcondition|requires that|provided that|assumes that|only when)\b",
    ),
    (
        "protocol",
        r"\b(protocol|handshake|wire format|message (format|sequence|order))\b",
    ),
    (
        "published-interface",
        r"\b(published interface|public (api|surface)|exported (api|surface)|semver|breaking change|backward[- ]compatib)",
    ),
    (
        "requirement-set",
        r"\b(set of requirements|across (all )?requirements|no two requirements|every requirement)\b",
    ),
    // `safety` as a bare word is NOT here. Measured: of 11 matches across the
    // five repos, 10 were `path-safety` and none were the 25010 characteristic.
    (
        "safety",
        r"\b(hazard|harm to|injur|fail-safe|mitigat(e|ion) of risk)",
    ),
    (
        "serialization",
        r"\b(serializ|round-?trip|encode[sd]? (to|as)|to_json)",
    ),
    (
        "single-component",
        r"\b(single (function|component|module|unit)|in isolation|pure function)\b",
    ),
    (
        "stakeholder-acceptance",
        r"\b(sign-?off|accepted by|acceptance by|demonstrat(e|ed|ion) to)\b",
    ),
    // `business` is not here: it matched the repo name `spec-objects-business`.
    (
        "stakeholder-facing",
        r"\b(stakeholder|end user|customer|operator-facing)\b",
    ),
    (
        "state-machine",
        r"\b(state machine|state transition|transitions? (from|to|into)|finite state)\b",
    ),
    (
        "structured-input",
        r"\b(grammar|structured input|well-formed|schema-valid|malformed (document|input|payload))\b",
    ),
    (
        "supply-chain",
        r"\b(supply chain|\bSBOM\b|vendored|transitive dependenc|provenance)\b",
    ),
    (
        "undefined-behaviour",
        r"\b(undefined behaviou?r|\bUB\b|data race|uninitializ|out[- ]of[- ]bounds)",
    ),
    (
        "universally-quantified",
        r"\b(for (every|all) [a-z]|every input|any input|universally|holds for)\b",
    ),
    // `scenario` dropped: in this corpus it names a test case or an example.
    (
        "workflow",
        r"\b(workflow|user journey|step \d|then the (user|operator))\b",
    ),
    // ── agent-ix/quoin#158: what makes `concolic-execution` reachable ──
    //
    // THE classic wall a fuzzer cannot climb. `signature` is NOT a bare
    // alternative — "function signature" is everywhere in this corpus.
    (
        "magic-value-comparison",
        r"\b(checksum|CRC(-?\d+)?|magic (number|byte|value)|HMAC|digest match|version header|(digital |cryptographic )signature)\b",
    ),
    // Constant-time code: a requirement forbidding a branch on secret data is a
    // direct instruction to run symbolic execution rather than to sample.
    (
        "secret-dependent-branch",
        r"\b(constant[- ]time|secret[- ]dependent|timing (side[- ]channel|attack)|branch on (a )?secret)\b",
    ),
    // Deliberately narrower than `stable-output`'s `identical output`: approval
    // testing compares against a RECORDED output, equivalence checking proves
    // two IMPLEMENTATIONS agree for all inputs.
    (
        "reference-equivalence",
        r"\b(reference implementation|previous implementation|behaviou?rally identical|semantically equivalent|equivalent to the (old|existing|legacy))\b",
    ),
];

/// Where each `\s`-bearing entry belongs in the retained order.
///
/// The retained table is one array and this port is two, so the insertion
/// points are named rather than implied: `latency` follows `reliability`,
/// `quantified-threshold` follows `throughput`, and `degradation` follows
/// `cross-component`. A test asserts the merged order against the retained
/// names.
const AFTER: [(&str, &str); 3] = [
    ("reliability", "latency"),
    ("throughput", "quantified-threshold"),
    ("cross-component", "degradation"),
];

/// The three entries whose patterns mix ASCII classes with `\s` or with `≤`/`≥`.
fn unicode_patterns() -> [(&'static str, String); 3] {
    let space = JS_WHITESPACE_CLASS;
    [
        // `/\b(latency|response time|within\s+\d|\d\s*ms\b|p9\d)/i`. No trailing
        // `\b`: a trailing one made `within 5ms` fail, because there is no word
        // boundary between the digit and the unit.
        (
            "latency",
            format!(
                r"(?i-u:\b(?:latency|response time))|(?i-u:\bwithin)[{space}]+(?-u:\d)|(?-u:\b\d)[{space}]*(?i-u:ms\b)|(?i-u:\bp9\d)"
            ),
        ),
        // `/[<>≤≥]\s*\d|\b\d+\s*(ms|s|%|MB|GB)\b/i`.
        (
            "quantified-threshold",
            format!(
                r"[<>\x{{2264}}\x{{2265}}][{space}]*(?-u:\d)|(?-u:\b\d+)[{space}]*(?i-u:(?:ms|s|%|MB|GB)\b)"
            ),
        ),
        // `/\b(degrad|graceful(ly)?\s+(fail|fall)|fall back)/i`.
        (
            "degradation",
            format!(
                r"(?i-u:\bdegrad)|(?i-u:\bgraceful(?:ly)?)[{space}]+(?i-u:fail|fall)|(?i-u:\bfall back)"
            ),
        ),
    ]
}

/// Compile the table once, in the retained order.
fn build_table() -> Vec<StatementCharacteristic> {
    #[allow(
        clippy::panic,
        reason = "these are literals; a pattern that fails to compile is a build \
                  defect at startup, and a fallible table is a failure no caller \
                  of `characteristics_of` could act on"
    )]
    fn compile(name: &str, pattern: &str) -> Regex {
        Regex::new(pattern)
            .unwrap_or_else(|cause| panic!("the `{name}` pattern is a literal: {cause}"))
    }

    let unicode = unicode_patterns();
    let mut table = Vec::with_capacity(ASCII_PATTERNS.len() + unicode.len());
    for (name, pattern) in ASCII_PATTERNS {
        table.push(StatementCharacteristic {
            name,
            pattern: compile(name, &format!("(?i-u){pattern}")),
        });
        if let Some((_, follower)) = AFTER.iter().find(|(after, _)| after == name)
            && let Some((_, pattern)) = unicode.iter().find(|(name, _)| name == follower)
        {
            table.push(StatementCharacteristic {
                name: follower,
                pattern: compile(follower, pattern),
            });
        }
    }
    table
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
    use super::{AFTER, ASCII_PATTERNS, STATEMENT_CHARACTERISTICS, unicode_patterns};
    use crate::advise::compound::matches_outside_compound;

    fn fires(name: &str, text: &str) -> bool {
        let entry = STATEMENT_CHARACTERISTICS
            .iter()
            .find(|entry| entry.name == name)
            .unwrap_or_else(|| panic!("no `{name}` entry"));
        matches_outside_compound(&entry.pattern, text)
    }

    #[test]
    fn every_pattern_compiles_and_the_table_is_the_declared_size() {
        assert_eq!(
            STATEMENT_CHARACTERISTICS.len(),
            ASCII_PATTERNS.len() + AFTER.len()
        );
        // Anti-vacuity: a table that silently lost its rows would still pass an
        // equality between two things it derives from itself.
        assert!(
            STATEMENT_CHARACTERISTICS.len() >= 50,
            "the retained table has 53 entries; this has {}",
            STATEMENT_CHARACTERISTICS.len()
        );
    }

    #[test]
    fn the_split_out_entries_sit_where_the_retained_table_put_them() {
        let names: Vec<&str> = STATEMENT_CHARACTERISTICS
            .iter()
            .map(|entry| entry.name)
            .collect();
        for (after, follower) in AFTER {
            let at = names.iter().position(|name| *name == after).unwrap();
            assert_eq!(
                names[at + 1],
                follower,
                "`{follower}` must follow `{after}`"
            );
        }
    }

    #[test]
    fn a_latency_threshold_written_without_a_space_still_fires() {
        assert!(fires("latency", "responds within 5ms"));
        assert!(fires("latency", "p99 under load"));
        assert!(!fires("latency", "nothing about speed"));
    }

    #[test]
    fn a_unicode_comparison_operator_is_a_quantified_threshold() {
        assert!(fires("quantified-threshold", "\u{2264} 4 minutes"));
        assert!(fires("quantified-threshold", "under 500 ms"));
        assert!(!fires("quantified-threshold", "fast enough"));
    }

    #[test]
    fn javascript_whitespace_is_wider_than_ascii_whitespace() {
        // U+00A0 is `\s` to a JavaScript RegExp and not `[[:space:]]` to an
        // ASCII class. If the port had used `\s` under `(?-u)` this would fail.
        assert!(fires("degradation", "gracefully\u{a0}fails over"));
        assert!(fires("quantified-threshold", "<\u{a0}4"));
    }

    #[test]
    fn word_boundaries_are_ascii_as_a_non_unicode_regexp_resolves_them() {
        // Rust's Unicode `\b` sees no boundary between `é` and `s`; a
        // JavaScript RegExp without `u` sees one, because `é` is not `\w`.
        assert!(
            fires("security", "caf\u{e9}token"),
            "an ASCII word boundary must fire after a non-ASCII character"
        );
    }

    #[test]
    fn every_unicode_pattern_is_placed_by_the_builder() {
        for (name, _) in unicode_patterns() {
            assert!(
                STATEMENT_CHARACTERISTICS
                    .iter()
                    .any(|entry| entry.name == name),
                "`{name}` was assembled but never inserted"
            );
        }
    }
}
