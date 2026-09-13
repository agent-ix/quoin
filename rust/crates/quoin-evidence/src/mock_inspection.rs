// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Source-level inspection for explicit stand-in substitutions (FR-032).
//!
//! Deliberately narrow. It recognizes names that declare their intent —
//! `Mock*`, `Fake*`, `Stub*`, `Spy*` — and explicit permissive factories such as
//! `Confirmation::allow()`. It does not decide whether that identifier overlaps
//! the behaviour an obligation claims to verify; the auditor does, from the
//! record this writes.
//!
//! # Offsets are bytes here and UTF-16 units in the oracle
//!
//! The retained TypeScript indexes with JavaScript string offsets. Every
//! comparison below is between offsets from the same scan, so the two agree
//! wherever the source is ASCII — which is every marker pattern and every
//! identifier the scan can match. The two bounded windows (`240` characters
//! between `#[test]` and its `fn`, `500` before an opening brace) are the one
//! place a multi-byte character shortens the Rust window, and both are
//! heuristics rather than contracts.

use std::collections::BTreeMap;

use regex::Regex;

use crate::error::EvidenceError;
use crate::ids::{Commit, SuiteId, SymbolId};
use crate::source::EvidenceSource;
use crate::store::{list_recorded_suites, read_mock_inspection};
use crate::types::MockInjection;

/// One test function's span in a source file.
#[derive(Debug, Clone)]
struct TestMarker {
    at: usize,
    end: usize,
    symbol: String,
}

/// Exact-commit inspection input for the pure auditor.
///
/// Only records **at that commit** count. An inspection of an older tree says
/// nothing about the source as it stands, and accepting one would let a stale
/// record answer for code it never read.
///
/// # Errors
///
/// [`EvidenceError::StoreIo`] or [`EvidenceError::StoreRead`] from the store.
pub fn mock_inspection_input<S: EvidenceSource + ?Sized>(
    source: &S,
    commit: Option<&Commit>,
) -> Result<(Vec<SuiteId>, Vec<MockInjection>), EvidenceError> {
    let Some(commit) = commit else {
        return Ok((Vec::new(), Vec::new()));
    };
    let mut suites = Vec::new();
    let mut injections = Vec::new();
    for suite in list_recorded_suites(source)? {
        let Some(record) = read_mock_inspection(source, &suite, commit)? else {
            continue;
        };
        if &record.commit != commit {
            continue;
        }
        suites.push(record.suite.clone());
        injections.extend(record.injections);
    }
    suites.sort();
    injections.sort_by(|a, b| {
        a.suite
            .as_str()
            .cmp(b.suite.as_str())
            .then_with(|| {
                a.path
                    .as_deref()
                    .unwrap_or("")
                    .cmp(b.path.as_deref().unwrap_or(""))
            })
            .then_with(|| a.line.unwrap_or(0).cmp(&b.line.unwrap_or(0)))
    });
    Ok((suites, injections))
}

/// Inspect the repository's test source for explicit stand-in constructors.
///
/// # Errors
///
/// [`EvidenceError::StoreIo`] when the walk or a read fails.
pub fn inspect_mock_injections<S: EvidenceSource + ?Sized>(
    source: &S,
    suite: &SuiteId,
) -> Result<Vec<MockInjection>, EvidenceError> {
    let call = call_regex();
    let mut found: Vec<MockInjection> = Vec::new();
    for path in source.source_files()? {
        let text = source.source_text(&path)?;
        let markers = test_markers(&text, extension_of(&path));
        if markers.is_empty() {
            continue;
        }
        for capture in call.captures_iter(&text) {
            let Some(identifier) = capture.get(1) else {
                continue;
            };
            if !looks_like_stand_in(identifier.as_str()) {
                continue;
            }
            let Some(whole) = capture.get(0) else {
                continue;
            };
            let Some(marker) = nearest_marker(&markers, whole.start()) else {
                continue;
            };
            found.push(MockInjection {
                suite: suite.clone(),
                symbol: SymbolId::new(marker.symbol.clone()),
                injects: vec![identifier.as_str().to_owned()],
                path: Some(path.clone()),
                line: Some(line_of(&text, whole.start())),
            });
        }
    }

    // Merged on `path:line:symbol`: one call site naming two stand-ins is one
    // injection carrying both, not two injections a reader has to reassemble.
    let mut merged: BTreeMap<String, MockInjection> = BTreeMap::new();
    for item in found {
        let key = format!(
            "{}:{}:{}",
            item.path.as_deref().unwrap_or(""),
            item.line.unwrap_or(0),
            item.symbol
        );
        merged
            .entry(key)
            .and_modify(|prior| {
                prior.injects.extend(item.injects.iter().cloned());
                prior.injects.sort();
                prior.injects.dedup();
            })
            .or_insert_with(|| {
                let mut fresh = item;
                fresh.injects.sort();
                fresh.injects.dedup();
                fresh
            });
    }
    let mut out: Vec<MockInjection> = merged.into_values().collect();
    out.sort_by(|a, b| {
        a.path
            .as_deref()
            .unwrap_or("")
            .cmp(b.path.as_deref().unwrap_or(""))
            .then_with(|| a.line.unwrap_or(0).cmp(&b.line.unwrap_or(0)))
            .then_with(|| a.symbol.as_str().cmp(b.symbol.as_str()))
    });
    Ok(out)
}

/// A qualified call: a capitalized type, one or more `::`/`.` segments, `(`.
fn call_regex() -> Regex {
    #[expect(
        clippy::expect_used,
        reason = "a literal pattern that fails to compile is a build-time defect, not a runtime condition, and the test below exercises it"
    )]
    Regex::new(r"\b([A-Z][A-Za-z0-9_]*(?:(?:::|\.)[A-Za-z_][A-Za-z0-9_]*)+)\s*\(")
        .expect("the call pattern is a literal")
}

fn extension_of(path: &str) -> &str {
    path.rsplit_once('.').map_or("", |(_, extension)| extension)
}

/// The 1-based line an offset falls on.
fn line_of(text: &str, at: usize) -> u64 {
    let prefix = text.get(..at).unwrap_or(text);
    u64::try_from(prefix.matches('\n').count() + 1).unwrap_or(u64::MAX)
}

/// Every test function in one file, ordered by position.
fn test_markers(text: &str, extension: &str) -> Vec<TestMarker> {
    #[expect(
        clippy::expect_used,
        reason = "literal patterns that fail to compile are a build-time defect, not a runtime condition"
    )]
    let patterns: Vec<Regex> = match extension {
        "rs" => vec![
            Regex::new(r"(?s)#\s*\[\s*test\s*\].{0,240}?\bfn\s+([A-Za-z_][A-Za-z0-9_]*)\s*\(")
                .expect("the rust test pattern is a literal"),
        ],
        "py" => vec![
            Regex::new(r"(?:^|\n)\s*(?:async\s+)?def\s+(test_[A-Za-z0-9_]*)\s*\(")
                .expect("the python test pattern is a literal"),
        ],
        _ => vec![
            Regex::new("\\b(?:test|it)\\s*\\(\\s*[\"'`]([^\"'`]+)[\"'`]")
                .expect("the javascript test pattern is a literal"),
            Regex::new(r"\b(?:async\s+)?function\s+(test[A-Za-z0-9_]*)\s*\(")
                .expect("the javascript function pattern is a literal"),
        ],
    };

    let mut markers = Vec::new();
    for pattern in &patterns {
        for capture in pattern.captures_iter(text) {
            let (Some(whole), Some(name)) = (capture.get(0), capture.get(1)) else {
                continue;
            };
            // The Python pattern consumes the preceding newline. Starting the
            // span there makes `python_function_end` see that same newline as
            // the end of the signature and terminate before the body, so the
            // span is anchored on the captured name instead.
            let at = if extension == "py" {
                name.start()
            } else {
                whole.start()
            };
            let end = if extension == "py" {
                python_function_end(text, at)
            } else {
                braced_body_end(text, whole.end())
            };
            markers.push(TestMarker {
                at,
                end,
                symbol: name.as_str().to_owned(),
            });
        }
    }
    markers.sort_by_key(|marker| marker.at);
    markers
}

/// The innermost test span containing an offset, or `None` outside every test.
fn nearest_marker(markers: &[TestMarker], at: usize) -> Option<&TestMarker> {
    let mut found = None;
    for marker in markers {
        if marker.at > at {
            break;
        }
        if at <= marker.end {
            found = Some(marker);
        }
    }
    found
}

/// Where a Python function body ends: the first later line indented no further.
fn python_function_end(text: &str, at: usize) -> usize {
    let line_start = text
        .get(..at)
        .and_then(|prefix| prefix.rfind('\n').map(|index| index + 1))
        .unwrap_or(0);
    let indentation = text
        .get(line_start..)
        .map_or(0, |rest| rest.len() - rest.trim_start().len());
    let Some(newline) = text.get(at..).and_then(|rest| rest.find('\n')) else {
        return text.len();
    };
    let mut next = at + newline + 1;
    while next < text.len() {
        let rest = text.get(next..).unwrap_or("");
        let line_end = rest.find('\n').map_or(text.len(), |index| next + index);
        let line = text.get(next..line_end).unwrap_or("");
        let trimmed = line.trim_start();
        if !trimmed.is_empty() && !trimmed.starts_with('#') {
            let current = line.len() - trimmed.len();
            if current <= indentation {
                return next.saturating_sub(1);
            }
        }
        next = line_end + 1;
    }
    text.len()
}

/// The closing brace of a braced body, ignoring braces in strings and comments.
fn braced_body_end(text: &str, from: usize) -> usize {
    let Some(relative) = text.get(from..).and_then(|rest| rest.find('{')) else {
        return from;
    };
    let open = from + relative;
    if open - from > 500 {
        return from;
    }
    let bytes = text.as_bytes();
    let mut depth = 0_i32;
    let mut quote: Option<u8> = None;
    let mut escaped = false;
    let mut line_comment = false;
    let mut block_comment = false;
    let mut index = open;
    while index < bytes.len() {
        let Some(&character) = bytes.get(index) else {
            break;
        };
        let next = bytes.get(index + 1).copied().unwrap_or(0);
        if line_comment {
            if character == b'\n' {
                line_comment = false;
            }
        } else if block_comment {
            if character == b'*' && next == b'/' {
                block_comment = false;
                index += 1;
            }
        } else if let Some(open_quote) = quote {
            if escaped {
                escaped = false;
            } else if character == b'\\' {
                escaped = true;
            } else if character == open_quote {
                quote = None;
            }
        } else if character == b'/' && next == b'/' {
            line_comment = true;
            index += 1;
        } else if character == b'/' && next == b'*' {
            block_comment = true;
            index += 1;
        } else if character == b'"' || character == b'\'' || character == b'`' {
            quote = Some(character);
        } else if character == b'{' {
            depth += 1;
        } else if character == b'}' {
            depth -= 1;
            if depth == 0 {
                return index;
            }
        }
        index += 1;
    }
    text.len()
}

/// Whether an identifier declares itself a stand-in.
///
/// Two independent rules: the *type* announces it (`MockClock`), or the *method*
/// is an explicit permissive factory (`Confirmation::allow`). Either is enough;
/// neither is a judgement about whether the substitution matters.
fn looks_like_stand_in(identifier: &str) -> bool {
    let parts: Vec<&str> = identifier
        .split("::")
        .flat_map(|part| part.split('.'))
        .collect();
    let type_name = parts.first().copied().unwrap_or("");
    let method = parts.last().copied().unwrap_or("");
    let lowered_type = type_name.to_ascii_lowercase();
    let lowered_method = method.to_ascii_lowercase();
    ["mock", "fake", "stub", "spy"]
        .iter()
        .any(|prefix| lowered_type.starts_with(prefix))
        || matches!(
            lowered_method.as_str(),
            "allow"
                | "approve"
                | "approved"
                | "bypass"
                | "fake"
                | "mock"
                | "stub"
                | "succeed"
                | "success"
        )
}
