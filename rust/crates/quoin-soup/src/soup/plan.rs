// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The pure half of `quoin soup promote` (FR-141): purl segments, record file
//! names, the accounting of components, and the rendered record. Nothing here
//! reads a file; `super` owns the disk and the module lookup.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::path::PathBuf;

use quoin_evidence::adapters::{DeclaredLicence, SbomComponent};

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

pub(super) use quoin_evidence::purl::percent_decode;

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
    /// The record's `disposition_clause`, when it names one (FR-142).
    pub(super) disposition_clause: Option<String>,
    /// The record's whole frontmatter, as parsed once.
    pub(super) front: serde_json::Map<String, serde_json::Value>,
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

/// The supplier and licence the SBOM states for one purl, after the omissions
/// FR-141 requires. A created record carries exactly these.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(super) struct Stated {
    pub(super) supplier: Option<String>,
    pub(super) license: Option<String>,
}

/// A record to write.
#[derive(Debug, PartialEq, Eq)]
pub(super) struct NewRecord {
    pub(super) purl: String,
    pub(super) name: String,
    pub(super) version: Option<String>,
    pub(super) file: String,
    pub(super) stated: Stated,
}

/// An existing record whose purl differs from a new one only in version.
#[derive(Debug, PartialEq, Eq)]
pub(super) struct Superseded {
    pub(super) path: PathBuf,
    pub(super) purl: String,
    pub(super) by: String,
}

/// A field of an existing record that differs from what the SBOM states.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct Difference {
    pub(super) purl: String,
    pub(super) path: PathBuf,
    pub(super) field: &'static str,
}

/// A component whose licences do not become the record's single `license`:
/// the declared texts, with `unreadable` for an entry that has none.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct Unrecorded {
    pub(super) symbol: String,
    pub(super) texts: Vec<String>,
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
    /// Fields of existing records that differ from the SBOM.
    pub(super) differences: Vec<Difference>,
    /// Components declaring several licences or only an unreadable one.
    pub(super) unrecorded: Vec<Unrecorded>,
    /// Purl and field where a later occurrence states other than the first.
    pub(super) conflicts: Vec<(String, &'static str)>,
    /// Symbol and field left off a record because its text holds a control character.
    pub(super) omitted: Vec<(String, &'static str)>,
}

/// An unchanged record: the purl and the file that already holds it.
#[derive(Debug, PartialEq, Eq)]
pub(super) struct Unchanged {
    pub(super) purl: String,
    pub(super) path: PathBuf,
}

/// What `component` states for its record, with the fields it omits and the
/// licence texts to list when no single licence can be written.
fn state(component: &SbomComponent) -> (Stated, Vec<&'static str>, Option<Vec<String>>) {
    let mut omitted = Vec::new();
    let supplier = component.supplier.clone().filter(|text| {
        let clean = !text.chars().any(char::is_control);
        if !clean {
            omitted.push("supplier");
        }
        clean
    });
    let (license, unrecorded) = match component.licences.as_slice() {
        [] => (None, None),
        [
            DeclaredLicence {
                text: Some(text), ..
            },
        ] => {
            if text.chars().any(char::is_control) {
                omitted.push("license");
                (None, None)
            } else {
                (Some(text.clone()), None)
            }
        }
        licences => (
            None,
            Some(
                licences
                    .iter()
                    .map(|licence| {
                        licence
                            .text
                            .clone()
                            .unwrap_or_else(|| "unreadable".to_owned())
                    })
                    .collect(),
            ),
        ),
    };
    (Stated { supplier, license }, omitted, unrecorded)
}

/// The fields of `existing` that differ from `stated`: set where the SBOM
/// states none, unset where it states one, or set to another value. A record
/// carrying `license_fact_ref` states its licence another way and is not
/// compared on `license`.
pub(super) fn differences(stated: &Stated, existing: &Existing) -> Vec<Difference> {
    let mut found = Vec::new();
    let mut compare = |field: &'static str, stated: Option<&str>| {
        let record = existing.front.get(field).filter(|value| !value.is_null());
        let same = match (record, stated) {
            (None, None) => true,
            (Some(value), Some(text)) => value.as_str() == Some(text),
            _ => false,
        };
        if !same {
            found.push(Difference {
                purl: existing.purl.clone(),
                path: existing.path.clone(),
                field,
            });
        }
    };
    compare("supplier", stated.supplier.as_deref());
    if !existing.front.contains_key("license_fact_ref") {
        compare("license", stated.license.as_deref());
    }
    found
}

/// Classify every component. Each one lands in exactly one of `created`,
/// `unchanged`, `merged` or `skipped`, so the four counts sum to `read`. A
/// purl's supplier and licence are its first occurrence's; a later occurrence
/// that states other than that is listed as a conflict. Every list is sorted,
/// so the report is deterministic.
pub(super) fn plan(components: &[SbomComponent], existing: &[Existing]) -> Plan {
    let by_purl: BTreeMap<&str, &Existing> = existing
        .iter()
        .map(|record| (record.purl.as_str(), record))
        .collect();
    let mut result = Plan {
        read: components.len(),
        ..Plan::default()
    };
    let mut seen: BTreeMap<&str, &SbomComponent> = BTreeMap::new();
    let mut conflicts = BTreeSet::new();
    for component in components {
        let symbol = &component.symbol;
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
        } else if let Some(first) = seen.get(symbol.as_str()) {
            result.merged.push(symbol.clone());
            if first.supplier != component.supplier {
                conflicts.insert((symbol.clone(), "supplier"));
            }
            if first.licences != component.licences {
                conflicts.insert((symbol.clone(), "license"));
            }
        } else {
            seen.insert(symbol.as_str(), component);
            let (stated, omitted, unrecorded) = state(component);
            result
                .omitted
                .extend(omitted.into_iter().map(|field| (symbol.clone(), field)));
            if let Some(texts) = unrecorded {
                result.unrecorded.push(Unrecorded {
                    symbol: symbol.clone(),
                    texts,
                });
            }
            if let Some(record) = by_purl.get(symbol.as_str()) {
                result.differences.extend(differences(&stated, record));
                result.unchanged.push(Unchanged {
                    purl: symbol.clone(),
                    path: record.path.clone(),
                });
                continue;
            }
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
                stated,
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
    result.differences.sort();
    result.unrecorded.sort();
    result.omitted.sort();
    result.conflicts = conflicts.into_iter().collect();
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
/// only what the SBOM states (and, for `supplier` and `license`, only where it
/// states one usable value), and the body is the skeleton's with its
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
    if let Some(supplier) = &record.stated.supplier {
        let _ = writeln!(text, "supplier: {}", quote(supplier));
    }
    if let Some(license) = &record.stated.license {
        let _ = writeln!(text, "license: {}", quote(license));
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
    use quoin_evidence::adapters::{DeclaredLicence, LicenceKind, SbomComponent};
    use std::path::PathBuf;

    fn component(symbol: &str, supplier: Option<&str>, licences: &[Option<&str>]) -> SbomComponent {
        SbomComponent {
            symbol: symbol.to_owned(),
            purl: Some(symbol.to_owned()),
            licences: licences
                .iter()
                .map(|text| DeclaredLicence {
                    kind: if text.is_some() {
                        LicenceKind::Id
                    } else {
                        LicenceKind::Unreadable
                    },
                    text: text.map(str::to_owned),
                })
                .collect(),
            supplier: supplier.map(str::to_owned),
        }
    }

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
                disposition_clause: None,
                front: serde_json::Map::new(),
            },
            Existing {
                path: PathBuf::from("b.md"),
                purl: "pkg:npm/b@1".to_owned(),
                id: None,
                disposition_clause: None,
                front: serde_json::Map::new(),
            },
        ];
        let symbols: Vec<SbomComponent> = ["pkg:npm/a@1", "pkg:npm/b@2", "pkg:npm/b@2", "c@3"]
            .map(|symbol| component(symbol, None, &[]))
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

    #[trace("FR-141-AC-25")]
    #[test]
    fn tc_1812_001_the_first_occurrence_supplies_the_record_and_a_later_one_conflicts() {
        let components = [
            component("pkg:npm/a@1", Some("First"), &[Some("MIT")]),
            component("pkg:npm/a@1", Some("Second"), &[Some("MIT")]),
        ];
        let planned = plan(&components, &[]);
        assert_eq!(planned.created.len(), 1);
        assert_eq!(planned.created[0].stated.supplier.as_deref(), Some("First"));
        assert_eq!(planned.conflicts, [("pkg:npm/a@1".to_owned(), "supplier")]);
    }

    #[trace("FR-141-AC-20", "FR-141-AC-21", "FR-141-AC-24")]
    #[test]
    fn tc_1812_002_only_one_readable_clean_licence_is_written() {
        let components = [
            component("pkg:npm/one@1", None, &[Some("MIT")]),
            component("pkg:npm/two@1", None, &[Some("MIT"), None]),
            component("pkg:npm/none@1", None, &[None]),
            component("pkg:npm/ctrl@1", Some("A\u{7}"), &[Some("M\nT")]),
        ];
        let planned = plan(&components, &[]);
        let license = |purl: &str| {
            planned
                .created
                .iter()
                .find(|r| r.purl == purl)
                .unwrap()
                .stated
                .license
                .clone()
        };
        assert_eq!(license("pkg:npm/one@1").as_deref(), Some("MIT"));
        assert_eq!(license("pkg:npm/two@1"), None);
        assert_eq!(license("pkg:npm/none@1"), None);
        assert_eq!(license("pkg:npm/ctrl@1"), None);
        let listed: Vec<_> = planned
            .unrecorded
            .iter()
            .map(|u| (u.symbol.as_str(), u.texts.join("|")))
            .collect();
        assert_eq!(
            listed,
            [
                ("pkg:npm/none@1", "unreadable".to_owned()),
                ("pkg:npm/two@1", "MIT|unreadable".to_owned())
            ]
        );
        assert_eq!(
            planned.omitted,
            [
                ("pkg:npm/ctrl@1".to_owned(), "license"),
                ("pkg:npm/ctrl@1".to_owned(), "supplier")
            ]
        );
    }
}
