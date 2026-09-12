// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The gate-capability validator (quoin#224, ported for quoin#377).
//!
//! Find a declared, wired shell gate that counts forbidden matches but never
//! asserts the count. The three-way join — a claim in the script, wiring in the
//! build, and an unasserted count on a line — is deliberate: **script text alone
//! cannot distinguish a gate from a report.** A file that counts `unwrap()` and
//! prints the number is a perfectly good report; the same file named in a
//! `Makefile` target called `gate` is a gate that passes while the forbidden
//! text is present.
//!
//! Ported from `src/validators/gates.ts` at quoin `4d27dcf`. Every regex below
//! is the JavaScript source literal, translated only where the `regex` crate
//! spells a construct differently; the divergences that remain are recorded in
//! `tests/golden/PROVENANCE.md`.

use std::path::Path;
use std::sync::LazyLock;

use regex::Regex;

use crate::error::ValidatorError;
use crate::finding::{EmptyGateFinding, FindingKind};
use crate::ids::{LineNumber, ObligationId, RepoPath};
use crate::repo::{read_text, relative_to, scan};

/// A gate's declared obligation and the claim it makes about it.
#[derive(Debug, Clone, PartialEq, Eq)]
struct GateClaim {
    obligation: ObligationId,
    statement: String,
}

/// Compile a pattern that is a source literal.
///
/// Compilation is a build-time property of this file, not a runtime condition:
/// every pattern is exercised by the unit tests at the bottom of the module, so
/// an unparseable one fails the suite rather than reaching a caller.
#[allow(
    clippy::expect_used,
    reason = "patterns are module literals covered by tc_377_010..017; a failure here is a typo caught by the test suite, not a runtime condition"
)]
fn literal_regex(pattern: &'static str) -> Regex {
    Regex::new(pattern).expect("validator pattern literal must compile")
}

/// `/^\s*#\s*(?:gate|check)\s+for\s+([A-Z][A-Z0-9-]+):\s*(.+?)\s*$/im`
static CLAIM: LazyLock<Regex> = LazyLock::new(|| {
    literal_regex(r"(?im)^\s*#\s*(?:gate|check)\s+for\s+([A-Z][A-Z0-9\-]+):\s*(.+?)\s*$")
});

/// `/\b(?:no|never|not|forbid(?:s|den)?|without)\b/i`
static NEGATIVE: LazyLock<Regex> =
    LazyLock::new(|| literal_regex(r"(?i)\b(?:no|never|not|forbid(?:s|den)?|without)\b"));

/// `/^\s*(?:if|while|until)\b/` — a line that already branches on the match is a
/// gate with a failure path, not an unasserted count.
static CONTROL_FLOW: LazyLock<Regex> = LazyLock::new(|| literal_regex(r"^\s*(?:if|while|until)\b"));

/// `/\|\s*wc\s+-?l\b/`
static COUNTER: LazyLock<Regex> = LazyLock::new(|| literal_regex(r"\|\s*wc\s+-?l\b"));

/// `/\b(?:grep|rg)\b/`
static MATCHER: LazyLock<Regex> = LazyLock::new(|| literal_regex(r"\b(?:grep|rg)\b"));

/// `/\|\s*(?:grep|test)\b/` — the count is consumed by something that can fail.
static CONSUMED: LazyLock<Regex> = LazyLock::new(|| literal_regex(r"\|\s*(?:grep|test)\b"));

/// `/(?:==|!=|\b(?:eq|ne|gt|ge|lt|le)\b)/` — the count is compared.
static COMPARED: LazyLock<Regex> =
    LazyLock::new(|| literal_regex(r"(?:==|!=|\b(?:eq|ne|gt|ge|lt|le)\b)"));

/// `/\b(?:grep|rg)\b[^\n]*?["']([^"']+)["']/`
static QUOTED_PATTERN: LazyLock<Regex> =
    LazyLock::new(|| literal_regex(r#"\b(?:grep|rg)\b[^\n]*?["']([^"']+)["']"#));

/// `/[a-z_][a-z0-9_]*/g`, applied to an already-lowercased string.
static IDENTIFIER: LazyLock<Regex> = LazyLock::new(|| literal_regex(r"[a-z_][a-z0-9_]*"));

/// Find every declared, wired shell gate in `repo` that counts forbidden
/// matches without asserting the count.
///
/// Findings are ordered by `(path, line, obligation)`. An empty result means the
/// validator looked and found nothing: every filesystem refusal is an
/// [`Err`], never a silently short walk.
///
/// # Errors
///
/// [`ValidatorError::RepoRootUnreadable`] when `repo` is not a listable
/// directory, [`ValidatorError::DirectoryUnreadable`] when a directory inside it
/// cannot be listed, and [`ValidatorError::FileUnreadable`] when a file the walk
/// found cannot be read.
pub fn inspect_empty_gates(repo: &Path) -> Result<Vec<EmptyGateFinding>, ValidatorError> {
    let files = scan(repo)?;

    // Wiring bodies are read once each, not once per candidate script: the
    // TypeScript re-reads every wiring file inside the per-script loop, which is
    // O(scripts x wiring) syscalls for an identical answer.
    let wiring: Vec<(String, String)> = files
        .wiring
        .iter()
        .map(|path| Ok((relative_to(repo, path), read_text(path)?)))
        .collect::<Result<_, ValidatorError>>()?;

    let mut findings = Vec::new();
    for path in &files.shell {
        let source = read_text(path)?;
        let Some(claim) = gate_claim(&source) else {
            continue;
        };
        if !NEGATIVE.is_match(&claim.statement) {
            continue;
        }
        let repo_path = relative_to(repo, path);
        let Some(wired_by) = wiring
            .iter()
            .find(|(_, body)| references_script(body, &repo_path))
            .map(|(wire_path, _)| wire_path.as_str())
        else {
            continue;
        };

        findings.extend(
            split_lines(&source)
                .enumerate()
                .filter_map(|(index, line)| {
                    let pattern = unasserted_count_pattern(line)?;
                    claim_mentions(&claim.statement, pattern).then(|| {
                        finding(
                            &claim,
                            &repo_path,
                            wired_by,
                            LineNumber::from_zero_based(index),
                            pattern,
                        )
                    })
                }),
        );
    }

    findings.sort_by(|a, b| a.sort_key().cmp(&b.sort_key()));
    Ok(findings)
}

fn finding(
    claim: &GateClaim,
    repo_path: &str,
    wired_by: &str,
    line: LineNumber,
    pattern: &str,
) -> EmptyGateFinding {
    let obligation = &claim.obligation;
    EmptyGateFinding {
        kind: FindingKind::GateThatGatesNothing,
        obligation: obligation.clone(),
        path: RepoPath::new(repo_path),
        line,
        wired_by: RepoPath::new(wired_by),
        subject: format!("gate for {obligation}"),
        change_target: format!("{repo_path}:{line}"),
        remedy: format!(
            "compare the count for \u{201c}{pattern}\u{201d} to zero or make the match \
             command exit non-zero when forbidden text is present"
        ),
        summary: format!(
            "{obligation} claims \u{201c}{}\u{201d}, and {wired_by} wires {repo_path}, but \
             line {line} only counts matches for \u{201c}{pattern}\u{201d} without asserting \
             zero. The gate succeeds when the forbidden text is present; compare the count to \
             zero or make a match exit non-zero.",
            claim.statement
        ),
    }
}

/// The first `# Gate for <OBLIGATION>: <statement>` comment in the file.
///
/// First, not best: a script that makes two claims is reported against the one
/// it states earliest, and if that one is not a negative claim the file is
/// skipped entirely. That is the TypeScript's behaviour and it is preserved.
fn gate_claim(source: &str) -> Option<GateClaim> {
    let captures = CLAIM.captures(source)?;
    Some(GateClaim {
        obligation: ObligationId::new(captures.get(1)?.as_str()),
        statement: captures.get(2)?.as_str().to_owned(),
    })
}

/// Return the searched token when a line merely prints its match count.
fn unasserted_count_pattern(line: &str) -> Option<&str> {
    if CONTROL_FLOW.is_match(line) {
        return None;
    }
    let counter = COUNTER.find(line)?;
    if !MATCHER.is_match(line) {
        return None;
    }
    let after_counter = line.get(counter.end()..)?;
    if CONSUMED.is_match(after_counter) || COMPARED.is_match(after_counter) {
        return None;
    }
    let quoted = QUOTED_PATTERN.captures(line)?.get(1)?.as_str().trim();
    (!quoted.is_empty()).then_some(quoted)
}

/// Does the claim name the thing the line searches for?
///
/// A shared identifier of four characters or more. The floor is what keeps
/// `src`, `for` and `no` from joining every claim to every pattern.
fn claim_mentions(statement: &str, pattern: &str) -> bool {
    let claim_words = identifiers(statement);
    identifiers(pattern)
        .into_iter()
        .any(|word| word.chars().count() >= 4 && claim_words.contains(&word))
}

fn identifiers(value: &str) -> std::collections::BTreeSet<String> {
    let lowered = value.to_lowercase();
    IDENTIFIER
        .find_iter(&lowered)
        .map(|m| m.as_str().to_owned())
        .collect()
}

/// `source.includes(repoPath) || source.includes("./" + repoPath) ||
/// source.includes(basename(repoPath))`.
///
/// The bare-basename arm is why this is a heuristic and not a parse: a wiring
/// file that merely mentions `gate.sh` in a comment counts as wiring it.
fn references_script(body: &str, repo_path: &str) -> bool {
    body.contains(repo_path) || body.contains(RepoPath::new(repo_path).file_name())
}

/// `source.split(/\r?\n/)`.
fn split_lines(source: &str) -> impl Iterator<Item = &str> {
    source
        .split('\n')
        .map(|line| line.strip_suffix('\r').unwrap_or(line))
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "test bodies: a panic here is a failing test, which is the intended signal"
)]
mod tests {
    use super::{
        CLAIM, claim_mentions, gate_claim, references_script, split_lines, unasserted_count_pattern,
    };

    /// The claim regex: keyword, obligation and statement, case-insensitively,
    /// with the first match winning.
    #[test]
    fn tc_377_010_gate_claim_shapes() {
        let claim = gate_claim("#!/bin/sh\n# Gate for FR-001-AC-1: no unwrap in src.\n").unwrap();
        assert_eq!(claim.obligation.as_str(), "FR-001-AC-1");
        assert_eq!(claim.statement, "no unwrap in src.");

        assert_eq!(
            gate_claim("   #   check   for   NFR-2:   never sleep  \n")
                .unwrap()
                .obligation
                .as_str(),
            "NFR-2"
        );
        // `i` applies to the obligation class too, so a lowercase id is kept as
        // written rather than rejected.
        assert_eq!(
            gate_claim("# gate for fr-003: no unwrap\n")
                .unwrap()
                .obligation
                .as_str(),
            "fr-003"
        );
        // First claim wins, even when a later one would have been reportable.
        let two = "# Gate for AA-1: counts every unwrap\n# Gate for BB-2: no unwrap\n";
        assert_eq!(gate_claim(two).unwrap().obligation.as_str(), "AA-1");

        assert!(gate_claim("# gate FR-001: no unwrap\n").is_none());
        assert!(gate_claim("# gate for F: no unwrap\n").is_none());
        assert!(gate_claim("echo '# gate for FR-001: no unwrap'\n").is_none());
        assert!(gate_claim("# Gate for FR-001:\n").is_none());
    }

    /// A statement's trailing whitespace is not part of it.
    #[test]
    fn tc_377_011_statement_is_right_trimmed() {
        assert_eq!(
            gate_claim("# Gate for FR-1: no unwrap   \r\n")
                .unwrap()
                .statement,
            "no unwrap"
        );
        assert!(CLAIM.is_match("# Gate for FR-1: no unwrap"));
    }

    /// The unasserted-count shape and every rejection next to it.
    #[test]
    fn tc_377_012_unasserted_count_pattern() {
        assert_eq!(
            unasserted_count_pattern(r#"grep -rn "unwrap()" src/ | wc -l"#),
            Some("unwrap()")
        );
        assert_eq!(
            unasserted_count_pattern(r"rg -n 'panic!' . | wc -l"),
            Some("panic!")
        );

        // Control flow already provides a failure path.
        assert_eq!(
            unasserted_count_pattern(r#"if grep -rn "unwrap" src/ | wc -l; then"#),
            None
        );
        assert_eq!(
            unasserted_count_pattern(r#"  while grep "x" f | wc -l; do"#),
            None
        );
        assert_eq!(
            unasserted_count_pattern(r#"until grep "x" f | wc -l; do"#),
            None
        );
        // The count is consumed or compared.
        assert_eq!(
            unasserted_count_pattern(r#"grep "unwrap" src | wc -l | grep -q '^0$'"#),
            None
        );
        assert_eq!(
            unasserted_count_pattern(r#"grep "unwrap" src | wc -l | test 0"#),
            None
        );
        assert_eq!(
            unasserted_count_pattern(r#"[ $(grep "unwrap" src | wc -l) -eq 0 ]"#),
            None
        );
        assert_eq!(
            unasserted_count_pattern(r#"n=$(grep "unwrap" s | wc -l); [ "$n" == "0" ]"#),
            None
        );
        // No counter, no matcher, or no quoted pattern.
        assert_eq!(unasserted_count_pattern(r#"grep -rn "unwrap" src/"#), None);
        assert_eq!(unasserted_count_pattern(r"cat f | wc -l"), None);
        assert_eq!(
            unasserted_count_pattern(r"grep -rn unwrap src/ | wc -l"),
            None
        );
        assert_eq!(
            unasserted_count_pattern(r#"grep -rn "" src/ | wc -l"#),
            None
        );
        // `ripgrep` is not `rg`: the word boundary is load-bearing.
        assert_eq!(unasserted_count_pattern(r#"echo "ripgrep" | wc -l"#), None);
        // `-?l\b`: `wc l` counts, `wc -lc` does not.
        assert_eq!(
            unasserted_count_pattern(r#"grep "unwrap" src | wc l"#),
            Some("unwrap")
        );
        assert_eq!(
            unasserted_count_pattern(r#"grep "unwrap" src | wc -lc"#),
            None
        );
    }

    /// The four-character floor on the shared identifier.
    #[test]
    fn tc_377_013_claim_mentions_needs_a_long_shared_word() {
        assert!(claim_mentions("no unwrap in src", "unwrap()"));
        assert!(claim_mentions("NO UNWRAP", "unwrap"));
        assert!(!claim_mentions("no TOD markers", "TOD"));
        assert!(!claim_mentions("no unwrap", "expect"));
        // `src` is three characters, so it cannot join anything.
        assert!(!claim_mentions("nothing in src", "src/"));
    }

    /// Wiring is matched by relative path or by bare basename.
    #[test]
    fn tc_377_014_references_script() {
        assert!(references_script(
            "gate:\n\t./scripts/check.sh\n",
            "scripts/check.sh"
        ));
        assert!(references_script(
            "gate:\n\tbash check.sh\n",
            "scripts/check.sh"
        ));
        assert!(references_script("# see check.sh\n", "scripts/check.sh"));
        assert!(!references_script(
            "gate:\n\t@echo report only\n",
            "scripts/check.sh"
        ));
    }

    /// Lines split on CRLF as well as LF.
    #[test]
    fn tc_377_015_split_lines_handles_crlf() {
        let lines: Vec<&str> = split_lines("a\r\nb\nc").collect();
        assert_eq!(lines, vec!["a", "b", "c"]);
        let trailing: Vec<&str> = split_lines("a\n").collect();
        assert_eq!(trailing, vec!["a", ""]);
    }
}
