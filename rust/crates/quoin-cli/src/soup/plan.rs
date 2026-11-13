// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The pure half of `quoin soup promote` (FR-141): purl segments, record file
//! names, the accounting of components, and the rendered record. Nothing here
//! reads a file; `super` owns the disk and the module lookup.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::path::PathBuf;

/// The segments of a purl that a record states (FR-141).
#[derive(Debug, PartialEq, Eq)]
pub(super) struct Purl {
    /// The purl up to and including the name segment: type, namespace and name,
    /// with no version, qualifiers or subpath.
    pub(super) base: String,
    /// The percent-decoded name segment.
    pub(super) name: String,
    /// The percent-decoded version segment, when the purl carries one.
    pub(super) version: Option<String>,
    /// The raw version segment, so two spellings of one version compare equal
    /// only when they are one spelling.
    raw_version: Option<String>,
}

/// Parse `pkg:<type>/[<namespace>/]<name>[@<version>][?<qualifiers>][#<subpath>]`.
///
/// `None` for anything else, which the command reports as skipped rather than
/// guessing an identity.
pub(super) fn parse_purl(symbol: &str) -> Option<Purl> {
    let rest = symbol.strip_prefix("pkg:")?;
    let core = rest.split(['#', '?']).next().unwrap_or_default();
    let slash = core.rfind('/')?;
    let (head, tail) = core.split_at(slash + 1);
    let (name_raw, version_raw) = match tail.split_once('@') {
        Some((name, version)) => (name, Some(version).filter(|v| !v.is_empty())),
        None => (tail, None),
    };
    if name_raw.is_empty() || head.len() < 2 || head.starts_with('/') {
        return None;
    }
    Some(Purl {
        base: format!("pkg:{head}{name_raw}"),
        name: percent_decode(name_raw),
        version: version_raw.map(percent_decode),
        raw_version: version_raw.map(str::to_owned),
    })
}

pub(super) fn percent_decode(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while let Some(&byte) = bytes.get(index) {
        let decoded = (byte == b'%')
            .then(|| bytes.get(index + 1..index + 3))
            .flatten()
            .and_then(|pair| std::str::from_utf8(pair).ok())
            .and_then(|pair| u8::from_str_radix(pair, 16).ok());
        if let Some(value) = decoded {
            out.push(value);
            index += 3;
        } else {
            out.push(byte);
            index += 1;
        }
    }
    String::from_utf8(out).unwrap_or_else(|_| text.to_owned())
}

/// The record file name of a purl: its identity without qualifiers and subpath,
/// every byte outside `A-Za-z0-9._-` as `%` and its two uppercase hex digits,
/// then `.md`. Qualifiers such as `download_url` would push a name past the
/// filesystem limit.
pub(super) fn file_name(purl: &str) -> String {
    let mut name = String::with_capacity(purl.len() + 3);
    for byte in purl.split(['?', '#']).next().unwrap_or_default().bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-') {
            name.push(char::from(byte));
        } else {
            let _ = write!(name, "%{byte:02X}");
        }
    }
    name.push_str(".md");
    name
}

/// A `soup_record` already on disk.
#[derive(Debug, Clone)]
pub(super) struct Existing {
    pub(super) path: PathBuf,
    pub(super) purl: String,
    /// The record's `id`, when it states one.
    pub(super) id: Option<String>,
}

/// The first two records that carry one purl.
pub(super) fn duplicate_records(existing: &[Existing]) -> Option<(&Existing, &Existing)> {
    let mut seen: BTreeMap<&str, &Existing> = BTreeMap::new();
    for record in existing {
        if let Some(first) = seen.insert(record.purl.as_str(), record) {
            return Some((first, record));
        }
    }
    None
}

/// A record to write.
#[derive(Debug, PartialEq, Eq)]
pub(super) struct NewRecord {
    pub(super) purl: String,
    pub(super) name: String,
    pub(super) version: Option<String>,
    pub(super) file: String,
}

/// An existing record whose purl differs from a new one only in version.
#[derive(Debug, PartialEq, Eq)]
pub(super) struct Superseded {
    pub(super) path: PathBuf,
    pub(super) purl: String,
    pub(super) by: String,
}

/// What promoting one document does, before any file is touched.
#[derive(Debug, Default, PartialEq, Eq)]
pub(super) struct Plan {
    pub(super) read: usize,
    pub(super) created: Vec<NewRecord>,
    pub(super) unchanged: Vec<Unchanged>,
    pub(super) superseded: Vec<Superseded>,
    pub(super) merged: Vec<String>,
    pub(super) skipped: Vec<String>,
}

/// An unchanged record: the purl and the file that already holds it.
#[derive(Debug, PartialEq, Eq)]
pub(super) struct Unchanged {
    pub(super) purl: String,
    pub(super) path: PathBuf,
}

/// Classify every component symbol. Each one lands in exactly one of
/// `created`, `unchanged`, `merged` or `skipped`, so the four counts sum to
/// `read`. Every list is sorted, so the report is deterministic.
pub(super) fn plan(symbols: &[String], existing: &[Existing]) -> Plan {
    let by_purl: BTreeMap<&str, &Existing> = existing
        .iter()
        .map(|record| (record.purl.as_str(), record))
        .collect();
    let mut result = Plan {
        read: symbols.len(),
        ..Plan::default()
    };
    let mut seen = BTreeSet::new();
    for symbol in symbols {
        let Some(purl) = parse_purl(symbol) else {
            result.skipped.push(symbol.clone());
            continue;
        };
        if purl
            .name
            .chars()
            .chain(purl.version.iter().flat_map(|v| v.chars()))
            .any(char::is_control)
        {
            result.skipped.push(symbol.clone());
        } else if !seen.insert(symbol.as_str()) {
            result.merged.push(symbol.clone());
        } else if let Some(record) = by_purl.get(symbol.as_str()) {
            result.unchanged.push(Unchanged {
                purl: symbol.clone(),
                path: record.path.clone(),
            });
        } else {
            for old in existing {
                let same_component = parse_purl(&old.purl).is_some_and(|other| {
                    other.base == purl.base && other.raw_version != purl.raw_version
                });
                if same_component {
                    result.superseded.push(Superseded {
                        path: old.path.clone(),
                        purl: old.purl.clone(),
                        by: symbol.clone(),
                    });
                }
            }
            result.created.push(NewRecord {
                purl: symbol.clone(),
                name: purl.name,
                version: purl.version,
                file: file_name(symbol),
            });
        }
    }
    result.created.sort_by(|a, b| a.purl.cmp(&b.purl));
    result.unchanged.sort_by(|a, b| a.purl.cmp(&b.purl));
    result
        .superseded
        .sort_by(|a, b| (&a.purl, &a.by).cmp(&(&b.purl, &b.by)));
    result.merged.sort();
    result.skipped.sort();
    result
}

/// The next unused `SOUP-NNN` number after every id in `ids`.
pub(super) fn next_id_number<'a>(ids: impl IntoIterator<Item = &'a str>) -> u32 {
    ids.into_iter()
        .filter_map(|id| id.strip_prefix("SOUP-")?.parse::<u32>().ok())
        .max()
        .map_or(1, |max| max.saturating_add(1))
}

/// Render one record from the module's own skeleton: the frontmatter states
/// only what the SBOM states, and the body is the skeleton's with its
/// authoring comment removed and its heading naming the record.
pub(super) fn render(
    skeleton: &str,
    id: &str,
    record: &NewRecord,
    disposition_clause: Option<&str>,
) -> String {
    let quote = |text: &str| serde_json::Value::String(text.to_owned()).to_string();
    let title = if record.name.is_empty() {
        record.purl.as_str()
    } else {
        record.name.as_str()
    };
    let mut text = format!(
        "---\nid: {id}\ntitle: {}\ntype: soup_record\nobject: soup_record\npurl: {}\nname: {}\n",
        quote(title),
        quote(&record.purl),
        quote(&record.name),
    );
    if let Some(version) = &record.version {
        let _ = writeln!(text, "version: {}", quote(version));
    }
    if let Some(clause) = disposition_clause {
        let _ = writeln!(text, "disposition_clause: {}", quote(clause));
    }
    text.push_str("---\n");
    let mut in_comment = false;
    for line in body(skeleton).lines() {
        if line.trim_start().starts_with("<!--") {
            in_comment = true;
        }
        if in_comment {
            in_comment = !line.contains("-->");
            continue;
        }
        if line.starts_with("# [") {
            let _ = writeln!(text, "# [{id}] {title}");
        } else {
            text.push_str(line);
            text.push('\n');
        }
    }
    text
}

/// The skeleton without its frontmatter.
fn body(skeleton: &str) -> &str {
    skeleton
        .strip_prefix("---\n")
        .and_then(|rest| rest.split_once("\n---\n"))
        .map_or(skeleton, |(_, body)| body)
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]
mod tests {
    use super::{Existing, file_name, next_id_number, parse_purl, plan};
    use ix_trace_rs::trace;
    use std::path::PathBuf;

    #[trace("FR-141-AC-1")]
    #[test]
    fn tc_1515_001_name_and_version_are_the_purl_segments() {
        let purl = parse_purl("pkg:npm/%40scope/pkg@%5E1.0.0?download_url=x#sub").unwrap();
        assert_eq!(purl.name, "pkg");
        assert_eq!(purl.version.as_deref(), Some("^1.0.0"));
        assert_eq!(purl.base, "pkg:npm/%40scope/pkg");
        let bare = parse_purl("pkg:cargo/serde").unwrap();
        assert_eq!(bare.version, None);
        assert!(parse_purl("serde@1.0.0").is_none());
        assert!(parse_purl("pkg:serde").is_none());
    }

    #[trace("FR-141-AC-13", "FR-141-AC-17")]
    #[test]
    fn tc_1515_002_file_names_hold_no_separator_and_stay_distinct() {
        assert_eq!(file_name("pkg:npm/a@1"), "pkg%3Anpm%2Fa%401.md",);
        assert_ne!(file_name("pkg:npm/a%2Fb"), file_name("pkg:npm/a/b"));
        assert_eq!(file_name("pkg:npm/a@1?x=y#z"), "pkg%3Anpm%2Fa%401.md");
    }

    #[trace("FR-141-AC-2", "FR-141-AC-6")]
    #[test]
    fn tc_1515_003_every_component_is_counted_once() {
        let existing = vec![
            Existing {
                path: PathBuf::from("a.md"),
                purl: "pkg:npm/a@1".to_owned(),
                id: None,
            },
            Existing {
                path: PathBuf::from("b.md"),
                purl: "pkg:npm/b@1".to_owned(),
                id: None,
            },
        ];
        let symbols: Vec<String> = ["pkg:npm/a@1", "pkg:npm/b@2", "pkg:npm/b@2", "c@3"]
            .map(str::to_owned)
            .to_vec();
        let planned = plan(&symbols, &existing);
        assert_eq!(planned.read, 4);
        assert_eq!(planned.unchanged.len(), 1);
        assert_eq!(planned.created.len(), 1);
        assert_eq!(planned.merged, ["pkg:npm/b@2"]);
        assert_eq!(planned.skipped, ["c@3"]);
        assert_eq!(planned.superseded.len(), 1);
        assert_eq!(planned.superseded[0].purl, "pkg:npm/b@1");
    }

    #[trace("FR-141-AC-14")]
    #[test]
    fn tc_1515_004_ids_continue_after_the_highest_existing() {
        assert_eq!(next_id_number(["SOUP-001", "SOUP-007", "other"]), 8);
        assert_eq!(next_id_number([]), 1);
    }
}
