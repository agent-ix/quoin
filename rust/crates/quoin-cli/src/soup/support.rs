// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Supplied support data for `quoin soup report` (FR-143): the support file,
//! its validation, and the pure evaluation of one record against it.
//!
//! Nothing here reads a file, the network or the wall clock. The caller reads
//! the `--support` file and the records, and passes the `--as-of` text.
//!
//! API, in the order a report uses it:
//!
//! 1. [`parse_support`] validates every entry of the file text before any is
//!    used, and returns a [`SupportFile`] or the [`SupportError`] naming the
//!    entry (FR-143-AC-1).
//! 2. [`as_of_date`] turns the `--as-of` text into its UTC calendar date
//!    (FR-143-AC-9).
//! 3. [`record_support`] reads a record's stated `support_status` and
//!    `end_of_support` from its frontmatter.
//! 4. [`SupportFile::evaluate`] resolves both fields of one record and judges
//!    past end of support.

use std::collections::BTreeMap;
use std::fmt;

use quoin_store::instant::{Instant, InstantGrammar};
use serde_json::Value;
use time::{Date, Month, OffsetDateTime};

use super::plan::{Purl, parse_purl, percent_decode};

/// The `SoupSupportStatus` vocabulary of `spec-objects-security` FR-010.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum SupportStatus {
    Supported,
    CommunitySupported,
    Unsupported,
    /// Nobody has assessed it.
    Unknown,
}

impl SupportStatus {
    /// The spelling the record and the support file use.
    pub(super) fn as_str(self) -> &'static str {
        match self {
            Self::Supported => "supported",
            Self::CommunitySupported => "community_supported",
            Self::Unsupported => "unsupported",
            Self::Unknown => "unknown",
        }
    }

    fn from_text(text: &str) -> Option<Self> {
        match text {
            "supported" => Some(Self::Supported),
            "community_supported" => Some(Self::CommunitySupported),
            "unsupported" => Some(Self::Unsupported),
            "unknown" => Some(Self::Unknown),
            _ => None,
        }
    }
}

/// An end-of-support date, or the statement that it is not known.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum EndOfSupport {
    Unknown,
    On(Date),
}

impl EndOfSupport {
    fn from_text(text: &str) -> Option<Self> {
        if text == "unknown" {
            Some(Self::Unknown)
        } else {
            calendar_date(text).map(Self::On)
        }
    }
}

impl fmt::Display for EndOfSupport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unknown => f.write_str("unknown"),
            Self::On(date) => write!(
                f,
                "{:04}-{:02}-{:02}",
                date.year(),
                u8::from(date.month()),
                date.day()
            ),
        }
    }
}

/// A strict `YYYY-MM-DD` calendar date.
fn calendar_date(text: &str) -> Option<Date> {
    let bytes = text.as_bytes();
    let shape = bytes.len() == 10
        && bytes.get(4) == Some(&b'-')
        && bytes.get(7) == Some(&b'-')
        && bytes
            .iter()
            .enumerate()
            .all(|(i, b)| i == 4 || i == 7 || b.is_ascii_digit());
    if !shape {
        return None;
    }
    let year = text.get(0..4)?.parse::<i32>().ok()?;
    let month = Month::try_from(text.get(5..7)?.parse::<u8>().ok()?).ok()?;
    let day = text.get(8..10)?.parse::<u8>().ok()?;
    Date::from_calendar_date(year, month, day).ok()
}

/// The UTC calendar date of the `--as-of` text: a date, or an RFC 3339 instant
/// whose offset is applied before the date is taken (FR-143-AC-9).
///
/// # Errors
///
/// [`SupportErrorCode::BadAsOf`] when the text is neither.
pub(super) fn as_of_date(text: &str) -> Result<Date, SupportError> {
    if let Some(date) = calendar_date(text) {
        return Ok(date);
    }
    Instant::parse(text, InstantGrammar::Rfc3339)
        .and_then(|instant| instant.epoch_millis())
        .and_then(|millis| OffsetDateTime::from_unix_timestamp(millis.div_euclid(1000)).ok())
        .map(OffsetDateTime::date)
        .ok_or_else(|| {
            SupportError::new(
                SupportErrorCode::BadAsOf,
                format!("--as-of {text:?} is neither a date nor an RFC 3339 instant"),
            )
        })
}

/// Why a support file or an `--as-of` was refused. Codes are the API: never
/// renamed, never reused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum SupportErrorCode {
    /// The file is not a JSON list of entries.
    NotAList,
    /// An entry is not an object, or lacks `purl`, `support_status`,
    /// `end_of_support` or `source`, or a field is not text.
    MissingField,
    /// An entry's `source` is empty.
    EmptySource,
    /// An entry carries a key other than the four fields.
    UnknownField,
    /// An entry's `purl` is not a package URL.
    BadPurl,
    /// An entry's `support_status` is outside `SoupSupportStatus`.
    BadStatus,
    /// An entry's `end_of_support` is neither a calendar date nor `unknown`.
    BadEndOfSupport,
    /// Two entries are equal in type, namespace, name and version.
    Duplicate,
    /// A record states a support field outside its vocabulary.
    BadRecordValue,
    /// The `--as-of` text is not a date or an instant.
    BadAsOf,
}

impl SupportErrorCode {
    /// Every code, so a caller enumerating them cannot miss one.
    pub(super) const ALL: &'static [Self] = &[
        Self::NotAList,
        Self::MissingField,
        Self::EmptySource,
        Self::UnknownField,
        Self::BadPurl,
        Self::BadStatus,
        Self::BadEndOfSupport,
        Self::Duplicate,
        Self::BadRecordValue,
        Self::BadAsOf,
    ];

    /// The stable spelling.
    pub(super) fn as_str(self) -> &'static str {
        match self {
            Self::NotAList => "soup.support_not_a_list",
            Self::MissingField => "soup.support_missing_field",
            Self::EmptySource => "soup.support_empty_source",
            Self::UnknownField => "soup.support_unknown_field",
            Self::BadPurl => "soup.support_bad_purl",
            Self::BadStatus => "soup.support_bad_status",
            Self::BadEndOfSupport => "soup.support_bad_end_of_support",
            Self::Duplicate => "soup.support_duplicate",
            Self::BadRecordValue => "soup.support_bad_record_value",
            Self::BadAsOf => "soup.support_bad_as_of",
        }
    }
}

impl fmt::Display for SupportErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// A refusal. `context` names the entry by its 1-based position (`entry`) and
/// `purl`; a duplicate also carries `other_entry` and `other_purl`, and an
/// unknown field carries `key`. The message repeats them for a reader.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{code}: {message}")]
pub(super) struct SupportError {
    pub(super) code: SupportErrorCode,
    pub(super) message: Box<str>,
    pub(super) context: BTreeMap<String, String>,
}

impl SupportError {
    fn new(code: SupportErrorCode, message: String) -> Self {
        Self {
            code,
            message: message.into_boxed_str(),
            context: BTreeMap::new(),
        }
    }

    fn with_context(mut self, key: &str, value: impl Into<String>) -> Self {
        self.context.insert(key.to_owned(), value.into());
        self
    }
}

/// The keys of a support entry; any other is refused.
const FIELDS: [&str; 4] = ["purl", "support_status", "end_of_support", "source"];

/// One validated support statement.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct SupportEntry {
    /// The entry's purl text, as supplied.
    pub(super) purl: String,
    pub(super) support_status: SupportStatus,
    pub(super) end_of_support: EndOfSupport,
    /// The public URL or citation, non-empty.
    pub(super) source: String,
    base: Vec<String>,
    version: Option<String>,
}

/// A validated support file. Entries are in file order.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(super) struct SupportFile {
    entries: Vec<SupportEntry>,
}

/// The identity of a purl for matching: the type, namespace and name segments,
/// each percent-decoded on its own so an encoded `/` stays inside its segment,
/// and the decoded version. Qualifiers and subpath are not part of it.
fn identity(purl: &Purl) -> (Vec<String>, Option<String>) {
    let segments = purl
        .base
        .strip_prefix("pkg:")
        .unwrap_or(&purl.base)
        .split('/')
        .map(percent_decode)
        .collect();
    (segments, purl.version.clone())
}

/// Validate every entry of the support file text, then return them. Nothing is
/// returned when any entry is refused (FR-143-AC-1).
///
/// # Errors
///
/// The first refused entry in file order; a duplicate names both entries.
pub(super) fn parse_support(text: &str) -> Result<SupportFile, SupportError> {
    let value: Value = serde_json::from_str(text).map_err(|error| {
        SupportError::new(
            SupportErrorCode::NotAList,
            format!("the support file is not JSON: {error}"),
        )
    })?;
    let Value::Array(items) = value else {
        return Err(SupportError::new(
            SupportErrorCode::NotAList,
            "the support file is not a list of entries".to_owned(),
        ));
    };
    let mut entries: Vec<SupportEntry> = Vec::with_capacity(items.len());
    let mut seen: BTreeMap<(Vec<String>, Option<String>), usize> = BTreeMap::new();
    for (index, item) in items.iter().enumerate() {
        let entry = parse_entry(index + 1, item)?;
        if let Some(first) = seen.insert((entry.base.clone(), entry.version.clone()), index + 1) {
            let earlier = entries.get(first - 1).map_or("", |e| e.purl.as_str());
            return Err(SupportError::new(
                SupportErrorCode::Duplicate,
                format!(
                    "entry {first} ({earlier}) and entry {} ({}) are duplicates",
                    index + 1,
                    entry.purl
                ),
            )
            .with_context("entry", (index + 1).to_string())
            .with_context("purl", entry.purl.clone())
            .with_context("other_entry", first.to_string())
            .with_context("other_purl", earlier));
        }
        entries.push(entry);
    }
    Ok(SupportFile { entries })
}

fn parse_entry(position: usize, item: &Value) -> Result<SupportEntry, SupportError> {
    let field = |name: &str| item.get(name).and_then(Value::as_str);
    let label = field("purl").unwrap_or("<no purl>");
    let refuse = |code, what: String| {
        SupportError::new(code, format!("entry {position} ({label}): {what}"))
            .with_context("entry", position.to_string())
            .with_context("purl", label)
    };
    if let Some(key) = item
        .as_object()
        .into_iter()
        .flat_map(|object| object.keys())
        .find(|key| !FIELDS.contains(&key.as_str()))
    {
        return Err(refuse(
            SupportErrorCode::UnknownField,
            format!("`{key}` is not a support entry field"),
        )
        .with_context("key", key.clone()));
    }
    let required = |name: &str| {
        field(name).ok_or_else(|| {
            refuse(
                SupportErrorCode::MissingField,
                format!("`{name}` is missing or not text"),
            )
        })
    };
    let purl = required("purl")?;
    let status = required("support_status")?;
    let end = required("end_of_support")?;
    let source = required("source")?;
    if source.trim().is_empty() {
        return Err(refuse(
            SupportErrorCode::EmptySource,
            "`source` is empty".to_owned(),
        ));
    }
    let parsed = parse_purl(purl).ok_or_else(|| {
        refuse(
            SupportErrorCode::BadPurl,
            "`purl` is not a package URL".to_owned(),
        )
    })?;
    let support_status = SupportStatus::from_text(status).ok_or_else(|| {
        refuse(
            SupportErrorCode::BadStatus,
            format!("`support_status` {status:?} is not a SoupSupportStatus member"),
        )
    })?;
    let end_of_support = EndOfSupport::from_text(end).ok_or_else(|| {
        refuse(
            SupportErrorCode::BadEndOfSupport,
            format!("`end_of_support` {end:?} is neither a calendar date nor `unknown`"),
        )
    })?;
    let (base, version) = identity(&parsed);
    Ok(SupportEntry {
        purl: purl.to_owned(),
        support_status,
        end_of_support,
        source: source.to_owned(),
        base,
        version,
    })
}

/// A record's own support fields: `None` is absent.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(super) struct RecordSupport {
    pub(super) support_status: Option<SupportStatus>,
    pub(super) end_of_support: Option<EndOfSupport>,
}

/// Read `support_status` and `end_of_support` from a record's frontmatter.
///
/// # Errors
///
/// [`SupportErrorCode::BadRecordValue`] when a field is present and is not a
/// member of its vocabulary; the value is not guessed.
pub(super) fn record_support(
    front: &serde_json::Map<String, Value>,
) -> Result<RecordSupport, SupportError> {
    let bad = |name: &str, value: &Value| {
        SupportError::new(
            SupportErrorCode::BadRecordValue,
            format!("the record's `{name}` {value} is not a valid value"),
        )
    };
    let support_status = match front.get("support_status") {
        None => None,
        Some(value) => Some(
            value
                .as_str()
                .and_then(SupportStatus::from_text)
                .ok_or_else(|| bad("support_status", value))?,
        ),
    };
    let end_of_support = match front.get("end_of_support") {
        None => None,
        Some(value) => Some(
            value
                .as_str()
                .and_then(EndOfSupport::from_text)
                .ok_or_else(|| bad("end_of_support", value))?,
        ),
    };
    Ok(RecordSupport {
        support_status,
        end_of_support,
    })
}

/// One field resolved against the support file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum Resolved<T> {
    /// Neither the record nor an entry states it. Never a guess.
    Unknown,
    /// The record states it (and no entry contradicts it).
    Record(T),
    /// The record does not state it; the entry does, from `source`.
    Supplied { value: T, source: String },
    /// Both state it and differ. The record's value is authoritative.
    Disagree {
        record: T,
        supplied: T,
        source: String,
    },
}

impl<T: Copy> Resolved<T> {
    /// The authoritative value: the record's, else the supplied one.
    pub(super) fn authoritative(&self) -> Option<T> {
        match self {
            Self::Unknown => None,
            Self::Record(value) | Self::Supplied { value, .. } => Some(*value),
            Self::Disagree { record, .. } => Some(*record),
        }
    }

    /// Whether the record and the file state different values.
    pub(super) fn disagrees(&self) -> bool {
        matches!(self, Self::Disagree { .. })
    }
}

/// Resolve one field. A value of `unknown` is "not stated" on either side.
fn resolve<T: Copy + PartialEq>(
    record: Option<T>,
    entry: Option<(T, &str)>,
    is_unknown: impl Fn(T) -> bool,
) -> Resolved<T> {
    let record = record.filter(|value| !is_unknown(*value));
    let entry = entry.filter(|(value, _)| !is_unknown(*value));
    match (record, entry) {
        (None, None) => Resolved::Unknown,
        (Some(value), None) => Resolved::Record(value),
        (None, Some((value, source))) => Resolved::Supplied {
            value,
            source: source.to_owned(),
        },
        (Some(record), Some((supplied, _))) if record == supplied => Resolved::Record(record),
        (Some(record), Some((supplied, source))) => Resolved::Disagree {
            record,
            supplied,
            source: source.to_owned(),
        },
    }
}

/// The support facts of one record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Evaluation {
    pub(super) support_status: Resolved<SupportStatus>,
    pub(super) end_of_support: Resolved<EndOfSupport>,
    /// The authoritative end-of-support date is before the `as_of` date.
    /// `false` when the date is `unknown`: neither past nor within support.
    pub(super) past_end_of_support: bool,
}

impl SupportFile {
    /// The entry that applies to `purl`: same type, namespace and name, and
    /// either no version or the record's own. A versioned entry wins over a
    /// version-less one (FR-143-AC-6).
    #[must_use]
    pub(super) fn matching(&self, purl: &str) -> Option<&SupportEntry> {
        let (base, version) = identity(&parse_purl(purl)?);
        let mut general = None;
        for entry in self.entries.iter().filter(|entry| entry.base == base) {
            match (&entry.version, &version) {
                (None, _) => general = Some(entry),
                (Some(wanted), Some(have)) if wanted == have => return Some(entry),
                _ => {}
            }
        }
        general
    }

    /// Resolve a record's support fields against this file and judge past end
    /// of support on `as_of`'s UTC date (FR-143-AC-9). The end day itself is
    /// not past.
    #[must_use]
    pub(super) fn evaluate(&self, purl: &str, record: RecordSupport, as_of: Date) -> Evaluation {
        let entry = self.matching(purl);
        let support_status = resolve(
            record.support_status,
            entry.map(|e| (e.support_status, e.source.as_str())),
            |status| status == SupportStatus::Unknown,
        );
        let end_of_support = resolve(
            record.end_of_support,
            entry.map(|e| (e.end_of_support, e.source.as_str())),
            |end| end == EndOfSupport::Unknown,
        );
        let past_end_of_support = matches!(
            end_of_support.authoritative(),
            Some(EndOfSupport::On(end)) if as_of > end
        );
        Evaluation {
            support_status,
            end_of_support,
            past_end_of_support,
        }
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]
mod tests {
    use super::{
        EndOfSupport, RecordSupport, Resolved, SupportErrorCode, SupportFile, SupportStatus,
        as_of_date, parse_support, record_support,
    };
    use ix_trace_rs::trace;
    use serde_json::{Value, json};

    fn entry(purl: &str, status: &str, end: &str, source: &str) -> Value {
        json!({"purl": purl, "support_status": status, "end_of_support": end, "source": source})
    }

    fn file(entries: &[Value]) -> SupportFile {
        parse_support(&Value::Array(entries.to_vec()).to_string()).unwrap()
    }

    fn refusal(entries: &[Value]) -> super::SupportError {
        parse_support(&Value::Array(entries.to_vec()).to_string()).unwrap_err()
    }

    fn end(text: &str) -> EndOfSupport {
        EndOfSupport::from_text(text).unwrap()
    }

    fn record(status: Option<&str>, end_text: Option<&str>) -> RecordSupport {
        RecordSupport {
            support_status: status.map(|s| SupportStatus::from_text(s).unwrap()),
            end_of_support: end_text.map(end),
        }
    }

    fn day(text: &str) -> time::Date {
        as_of_date(text).unwrap()
    }

    #[trace("FR-143-AC-1")]
    #[test]
    fn tc_1517_001_each_invalid_entry_refuses_the_whole_file_naming_it() {
        let good = entry(
            "pkg:cargo/ok@1",
            "supported",
            "2030-01-01",
            "https://x.test/p",
        );
        let cases = [
            (
                entry("pkg:cargo/a", "supported", "2030-01-01", ""),
                SupportErrorCode::EmptySource,
            ),
            (
                entry("pkg:cargo/a", "supported", "2030-01-01", "  "),
                SupportErrorCode::EmptySource,
            ),
            (
                json!({"purl": "pkg:cargo/a", "support_status": "supported", "end_of_support": "unknown"}),
                SupportErrorCode::MissingField,
            ),
            (
                entry("pkg:cargo/a", "maintained", "2030-01-01", "s"),
                SupportErrorCode::BadStatus,
            ),
            (
                entry("pkg:cargo/a", "supported", "soon", "s"),
                SupportErrorCode::BadEndOfSupport,
            ),
            (
                entry("pkg:cargo/a", "supported", "2026-02-30", "s"),
                SupportErrorCode::BadEndOfSupport,
            ),
            (
                entry("pkg:cargo/a", "supported", "2026-6-1", "s"),
                SupportErrorCode::BadEndOfSupport,
            ),
            (
                entry("cargo/a", "supported", "2030-01-01", "s"),
                SupportErrorCode::BadPurl,
            ),
        ];
        for (bad, code) in cases {
            let error = refusal(&[good.clone(), bad.clone()]);
            assert_eq!(error.code, code, "{bad}");
            assert_eq!(
                error.context.get("entry").map(String::as_str),
                Some("2"),
                "{bad}"
            );
        }
        let dup = refusal(&[
            entry("pkg:cargo/a@1", "supported", "2030-01-01", "s"),
            good,
            entry("pkg:cargo/a@1?arch=x#sub", "unsupported", "unknown", "t"),
        ]);
        assert_eq!(dup.code, SupportErrorCode::Duplicate);
        assert_eq!(
            dup.context.get("other_entry").map(String::as_str),
            Some("1")
        );
        assert_eq!(dup.context.get("entry").map(String::as_str), Some("3"));
        assert_eq!(
            parse_support("{\"purl\": 1}").unwrap_err().code,
            SupportErrorCode::NotAList
        );
    }

    #[trace("FR-143-AC-2")]
    #[test]
    fn tc_1517_002_an_unknown_record_value_takes_the_labelled_entry_value() {
        let f = file(&[entry(
            "pkg:cargo/a",
            "supported",
            "2030-05-01",
            "https://a.test/eol",
        )]);
        let got = f.evaluate(
            "pkg:cargo/a@2.0.0",
            record(Some("unknown"), Some("unknown")),
            day("2026-01-01"),
        );
        assert_eq!(
            got.end_of_support,
            Resolved::Supplied {
                value: end("2030-05-01"),
                source: "https://a.test/eol".to_owned()
            }
        );
        assert_eq!(
            got.support_status,
            Resolved::Supplied {
                value: SupportStatus::Supported,
                source: "https://a.test/eol".to_owned()
            }
        );
        let absent = f.evaluate(
            "pkg:cargo/a@2.0.0",
            RecordSupport::default(),
            day("2026-01-01"),
        );
        assert_eq!(absent, got);
    }

    #[trace("FR-143-AC-3")]
    #[test]
    fn tc_1517_003_a_stated_record_value_wins_and_a_difference_shows_both() {
        let f = file(&[entry(
            "pkg:cargo/a",
            "unsupported",
            "2026-01-01",
            "https://a.test/eol",
        )]);
        let got = f.evaluate(
            "pkg:cargo/a@1",
            record(Some("supported"), Some("2030-01-01")),
            day("2027-01-01"),
        );
        assert_eq!(
            got.end_of_support,
            Resolved::Disagree {
                record: end("2030-01-01"),
                supplied: end("2026-01-01"),
                source: "https://a.test/eol".to_owned(),
            }
        );
        assert!(got.end_of_support.disagrees() && got.support_status.disagrees());
        assert_eq!(got.end_of_support.authoritative(), Some(end("2030-01-01")));
        assert!(
            !got.past_end_of_support,
            "the record's date is authoritative"
        );
        let same = f.evaluate(
            "pkg:cargo/a@1",
            record(Some("unsupported"), Some("2026-01-01")),
            day("2025-01-01"),
        );
        assert!(!same.end_of_support.disagrees());
    }

    #[trace("FR-143-AC-4")]
    #[test]
    fn tc_1517_004_nothing_stated_stays_unknown_and_unflagged() {
        let f = file(&[entry("pkg:cargo/other", "supported", "2020-01-01", "s")]);
        for stated in [
            RecordSupport::default(),
            record(Some("unknown"), Some("unknown")),
        ] {
            let got = f.evaluate("pkg:cargo/a@1", stated, day("2099-01-01"));
            assert_eq!(got.support_status, Resolved::Unknown);
            assert_eq!(got.end_of_support, Resolved::Unknown);
            assert!(!got.past_end_of_support);
        }
        let unknown_entry = file(&[entry("pkg:cargo/a", "unknown", "unknown", "s")]);
        let got =
            unknown_entry.evaluate("pkg:cargo/a@1", RecordSupport::default(), day("2099-01-01"));
        assert_eq!(got.end_of_support, Resolved::Unknown);
    }

    #[trace("FR-143-AC-5")]
    #[test]
    fn tc_1517_005_past_end_of_support_is_judged_against_the_as_of_date() {
        let f = SupportFile::default();
        let as_of = day("2026-06-01");
        let flagged = |date: &str| {
            f.evaluate("pkg:cargo/a", record(None, Some(date)), as_of)
                .past_end_of_support
        };
        assert!(flagged("2026-05-31"));
        assert!(!flagged("2026-06-01"));
        assert!(!flagged("2026-06-02"));
        assert!(!flagged("unknown"));
        let from_entry = file(&[entry("pkg:cargo/a", "unsupported", "2026-05-31", "s")]);
        assert!(
            from_entry
                .evaluate("pkg:cargo/a", RecordSupport::default(), as_of)
                .past_end_of_support
        );
    }

    #[trace("FR-143-AC-6")]
    #[test]
    fn tc_1517_006_matching_ignores_qualifiers_and_prefers_the_versioned_entry() {
        let f = file(&[
            entry("pkg:cargo/a", "supported", "2030-01-01", "general"),
            entry("pkg:cargo/a@1", "unsupported", "2020-01-01", "versioned"),
            entry("pkg:cargo/b@2", "supported", "2030-01-01", "b2"),
        ]);
        let source = |purl: &str| f.matching(purl).map(|e| e.source.as_str());
        assert_eq!(source("pkg:cargo/a@1?arch=x#sub"), Some("versioned"));
        assert_eq!(source("pkg:cargo/a@7"), Some("general"));
        assert_eq!(source("pkg:cargo/a"), Some("general"));
        assert_eq!(
            source("pkg:cargo/b@3"),
            None,
            "another version matches nothing"
        );
        assert_eq!(source("pkg:cargo/b"), None);
        assert_eq!(source("pkg:npm/a@1"), None, "type differs");
        assert_eq!(source("pkg:cargo/ns/a@1"), None, "namespace differs");
        assert_eq!(source("pkg:cargo/c"), None);
        assert_eq!(source("pkg:cargo/b@2"), Some("b2"));
    }

    #[trace("FR-143-AC-9")]
    #[test]
    fn tc_1517_007_as_of_is_compared_by_its_utc_date() {
        let f = file(&[entry("pkg:cargo/a", "unsupported", "2026-05-31", "s")]);
        let past = |as_of: &str| {
            f.evaluate(
                "pkg:cargo/a",
                RecordSupport::default(),
                as_of_date(as_of).unwrap(),
            )
            .past_end_of_support
        };
        assert!(!past("2026-05-31T12:00:00Z"));
        assert!(!past("2026-05-31T23:59:59Z"));
        assert!(past("2026-06-01T00:00:00Z"));
        // 00:30 on 06-01 at +02:00 is still 05-31 in UTC.
        assert!(!past("2026-06-01T00:30:00+02:00"));
        // 23:30 on 05-31 at -02:00 is already 06-01 in UTC.
        assert!(past("2026-05-31T23:30:00-02:00"));
        assert_eq!(
            as_of_date("tomorrow").unwrap_err().code,
            SupportErrorCode::BadAsOf
        );
    }

    #[test]
    fn tc_1517_008_a_record_frontmatter_is_read_without_defaults() {
        let front = |value: Value| value.as_object().unwrap().clone();
        let stated = record_support(&front(
            json!({"support_status": "supported", "end_of_support": "2027-03-04"}),
        ))
        .unwrap();
        assert_eq!(stated, record(Some("supported"), Some("2027-03-04")));
        assert_eq!(
            record_support(&front(json!({}))).unwrap(),
            RecordSupport::default()
        );
        let bad = record_support(&front(json!({"support_status": "bogus"}))).unwrap_err();
        assert_eq!(bad.code, SupportErrorCode::BadRecordValue);
        let md =
            "---\ntype: soup_record\npurl: pkg:cargo/a@1\nend_of_support: 2027-03-04\n---\n# a\n";
        let parsed = quoin_quire::validate::frontmatter(md).unwrap();
        assert_eq!(
            record_support(&parsed).unwrap().end_of_support,
            Some(end("2027-03-04"))
        );
    }

    #[test]
    fn tc_1517_009_an_entry_with_another_key_is_refused_naming_it() {
        let mut stray = entry("pkg:cargo/a", "supported", "2030-01-01", "s");
        stray["sorce"] = json!("typo");
        let error = refusal(&[stray]);
        assert_eq!(error.code, SupportErrorCode::UnknownField);
        assert_eq!(error.context.get("key").map(String::as_str), Some("sorce"));
        assert_eq!(error.context.get("entry").map(String::as_str), Some("1"));
    }

    #[trace("FR-143-AC-6")]
    #[test]
    fn tc_1517_010_an_encoded_slash_stays_inside_its_segment() {
        let f = file(&[
            entry("pkg:x/a%2Fb", "supported", "2030-01-01", "name-with-slash"),
            entry("pkg:x/a/b", "unsupported", "2020-01-01", "namespaced"),
        ]);
        let source = |purl: &str| f.matching(purl).map(|e| e.source.as_str());
        assert_eq!(source("pkg:x/a%2Fb@1"), Some("name-with-slash"));
        assert_eq!(source("pkg:x/a/b@1"), Some("namespaced"));
        let same = file(&[entry("pkg:x/%40s/a", "supported", "2030-01-01", "enc")]);
        assert!(
            same.matching("pkg:x/@s/a").is_some(),
            "per-segment decoding still equates spellings"
        );
    }

    #[test]
    fn tc_1517_011_error_codes_are_unique_and_namespaced() {
        let spellings: std::collections::BTreeSet<&str> =
            SupportErrorCode::ALL.iter().map(|c| c.as_str()).collect();
        assert_eq!(spellings.len(), SupportErrorCode::ALL.len());
        assert!(spellings.iter().all(|s| s.starts_with("soup.support_")));
        assert_eq!(SupportErrorCode::ALL.len(), 10);
    }
}
