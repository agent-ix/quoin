// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! The cutover gate: replay every digest in every reachable store through both
//! implementations and require zero mismatches.
//!
//! Three questions are answered separately, because they fail for different
//! reasons and only one of them blocks cutover:
//!
//! 1. **Agreement.** For every JSON node in every store file, does Rust compute
//!    the same canonical bytes and the same digest as the TypeScript oracle?
//!    Any disagreement here blocks cutover. There is no acceptable non-zero
//!    count.
//! 2. **Self-consistency.** Does every sealed record's stored digest still
//!    equal its recomputed digest, and does every attestation still match its
//!    retained output? A failure here is pre-existing corruption in the store,
//!    not a port defect — it must be reported as what it is.
//! 3. **Round-trip.** Is `read -> re-serialize` byte-identical to what is on
//!    disk? A file that was not written by the canonical writer will differ;
//!    that is a property of the corpus, reported and not silently folded into
//!    the agreement count.
//!
//! # The compared population is part of the gate
//!
//! Question 1 is only answered for store entities the oracle actually has an
//! entry for. "No entry" is therefore not "agreement" — it is *no evidence*,
//! and a gate that cannot tell the two apart passes loudest exactly when it is
//! comparing nothing. So [`ReplayReport`] counts the comparisons it performed
//! and [`ReplayReport::gate_passes`] requires that count to be non-zero *and*
//! to cover every store entity walked, with no entry unmatched on either side.
//! A key that the oracle spells differently from this side — the failure that
//! a resolved-versus-verbatim repository path used to produce — now shows up
//! as two unmatched populations and a failing gate, not as a silent PASS.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use crate::digest::{digest_canonical_value, digest_raw_bytes, verify_record_digest};
use crate::error::StoreError;
use crate::json::jcs::canonicalize_jcs;
use crate::json::order::jcs_order;
use crate::json::parse::parse_strict_json;
use crate::json::pretty::canonical_json;
use crate::json::value::JsonValue;
use crate::store::store_root;

/// Which canonical form a store file is written in.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StoreForm {
    /// `canonicalJson`: two-space indent, trailing newline.
    Pretty,
    /// RFC 8785 JCS: compact, no trailing newline.
    Jcs,
}

impl StoreForm {
    /// The form the evidence store uses for a given path.
    ///
    /// The change-assurance family stores the exact canonical bytes a digest
    /// was taken over; every other family stores the reviewable pretty form.
    #[must_use]
    pub fn for_path(path: &Path) -> Self {
        let is_change_assurance = path
            .components()
            .any(|component| component.as_os_str() == "change-assurance");
        if is_change_assurance {
            Self::Jcs
        } else {
            Self::Pretty
        }
    }

    /// The canonical bytes this form produces for a value.
    ///
    /// # Errors
    ///
    /// [`StoreError::JsonNestingTooDeep`] when the value nests deeper than
    /// [`crate::json::MAX_NESTING_DEPTH`]; both writers are depth-bounded so
    /// that a hostile document cannot exhaust the stack.
    pub fn serialize(self, value: &JsonValue) -> Result<Vec<u8>, StoreError> {
        match self {
            Self::Pretty => canonical_json(value).map(String::into_bytes),
            Self::Jcs => canonicalize_jcs(value).map(String::into_bytes),
        }
    }

    /// The stable label used in reports and in the oracle capture.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Pretty => "pretty",
            Self::Jcs => "jcs",
        }
    }
}

/// Every JSON node of `value`, in the order both implementations walk them:
/// the node itself, then its children — array items by index, object members
/// in RFC 8785 name order.
///
/// Scalars are included deliberately. They are where number formatting and
/// string escaping live, and those are the two places the canonicalizers were
/// most likely to disagree.
pub fn walk_nodes<'a>(value: &'a JsonValue, out: &mut Vec<&'a JsonValue>) {
    out.push(value);
    match value {
        JsonValue::Array(items) => {
            for item in items {
                walk_nodes(item, out);
            }
        }
        JsonValue::Object(object) => {
            for name in jcs_order(object.names()) {
                if let Some(member) = object.get(name) {
                    walk_nodes(member, out);
                }
            }
        }
        _ => {}
    }
}

/// The canonical digest of every node of `value`, in [`walk_nodes`] order.
///
/// # Errors
///
/// [`StoreError::JsonNestingTooDeep`] when a node nests deeper than
/// [`crate::json::MAX_NESTING_DEPTH`].
pub fn node_digests(value: &JsonValue) -> Result<Vec<String>, StoreError> {
    let mut nodes = Vec::new();
    walk_nodes(value, &mut nodes);
    nodes
        .into_iter()
        .map(|node| digest_canonical_value(node).map(|digest| digest.as_hex().to_owned()))
        .collect()
}

/// What the TypeScript oracle reported for one store file.
#[derive(Clone, Debug)]
pub struct OracleFile {
    /// Store-relative path, as captured.
    pub path: String,
    /// `true` when the oracle's strict reader accepted the file.
    pub parsed: bool,
    /// The oracle's error message when it did not.
    pub error: Option<String>,
    /// Digest of the oracle's canonical text for the file's form.
    pub serialization_digest: Option<String>,
    /// Whether the oracle's re-serialization equalled the bytes on disk.
    pub round_trip_identical: Option<bool>,
    /// The oracle's per-node canonical digests.
    pub node_digests: Vec<String>,
}

/// The oracle capture for a set of repositories.
#[derive(Clone, Debug, Default)]
pub struct Oracle {
    /// Keyed by `<repo-label>/<store-relative path>`.
    pub files: BTreeMap<String, OracleFile>,
    /// Raw-domain digests of retained producer outputs, same key shape.
    pub raw_files: BTreeMap<String, String>,
}

impl Oracle {
    /// Read a capture produced by `oracle/capture-store-oracle.mjs`.
    ///
    /// The capture is newline-delimited JSON: one record per line, each tagged
    /// `"file"` or `"raw"`. It is a stream because a single document holding
    /// every node digest of every store in the ecosystem does not fit in a
    /// default V8 heap — and, for the same reason, this reader never holds more
    /// than one line's parse at a time.
    ///
    /// # Errors
    ///
    /// Refuses a line that is not UTF-8 or not a JSON object, and a record that
    /// does not carry the members its `kind` requires.
    pub fn from_ndjson(bytes: &[u8]) -> Result<Self, StoreError> {
        let mut oracle = Self::default();
        for line in bytes.split(|byte| *byte == b'\n') {
            if line.iter().all(u8::is_ascii_whitespace) {
                continue;
            }
            let value = parse_strict_json(line)?;
            let object = value.as_object()?;
            let key = object
                .get("key")
                .and_then(JsonValue::as_str)
                .unwrap_or_default()
                .to_owned();
            if let Some("raw") = object.get("kind").and_then(JsonValue::as_str) {
                if let Some(digest) = object.get("digest").and_then(JsonValue::as_str) {
                    oracle.raw_files.insert(key, digest.to_owned());
                }
            } else {
                let node_digests = match object.get("node_digests") {
                    Some(JsonValue::Array(items)) => items
                        .iter()
                        .filter_map(|item| item.as_str().map(str::to_owned))
                        .collect(),
                    _ => Vec::new(),
                };
                oracle.files.insert(
                    key,
                    OracleFile {
                        path: object
                            .get("path")
                            .and_then(JsonValue::as_str)
                            .unwrap_or_default()
                            .to_owned(),
                        parsed: matches!(object.get("parsed"), Some(JsonValue::Bool(true))),
                        error: object
                            .get("error")
                            .and_then(JsonValue::as_str)
                            .map(str::to_owned),
                        serialization_digest: object
                            .get("serialization_digest")
                            .and_then(JsonValue::as_str)
                            .map(str::to_owned),
                        round_trip_identical: match object.get("round_trip_identical") {
                            Some(JsonValue::Bool(flag)) => Some(*flag),
                            _ => None,
                        },
                        node_digests,
                    },
                );
            }
        }
        Ok(oracle)
    }
}

/// A disagreement between the two implementations. Never "minor".
#[derive(Clone, Debug)]
pub struct Divergence {
    /// The store file involved.
    pub key: String,
    /// What disagreed.
    pub kind: DivergenceKind,
    /// What TypeScript said.
    pub typescript: String,
    /// What Rust said.
    pub rust: String,
}

/// The kind of disagreement observed.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DivergenceKind {
    /// One implementation accepted the file and the other refused it.
    Acceptance,
    /// The canonical serializations differ.
    Serialization,
    /// The number of nodes walked differs.
    NodeCount,
    /// A node's canonical digest differs.
    NodeDigest,
    /// The round-trip verdicts differ.
    RoundTrip,
}

impl DivergenceKind {
    /// Stable label for reports.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Acceptance => "acceptance",
            Self::Serialization => "serialization",
            Self::NodeCount => "node-count",
            Self::NodeDigest => "node-digest",
            Self::RoundTrip => "round-trip",
        }
    }
}

/// A store record whose own integrity check failed.
#[derive(Clone, Debug)]
pub struct IntegrityFinding {
    /// The file involved.
    pub path: PathBuf,
    /// What failed.
    pub detail: String,
}

/// The outcome of a replay over one or more repositories.
#[derive(Clone, Debug, Default)]
pub struct ReplayReport {
    /// Repositories that had a store.
    pub repositories: Vec<PathBuf>,
    /// Store files found.
    pub files_found: usize,
    /// Store files the Rust reader accepted.
    pub files_parsed: usize,
    /// Files the Rust reader refused, with the reason.
    pub parse_refusals: Vec<IntegrityFinding>,
    /// Digests recomputed in Rust and compared against the oracle.
    pub digests_replayed: usize,
    /// Disagreements between the two implementations. Must be empty.
    pub divergences: Vec<Divergence>,
    /// Files whose re-serialization equalled the bytes on disk.
    pub round_trip_identical: usize,
    /// Files whose re-serialization differed from the bytes on disk.
    pub round_trip_divergent: Vec<PathBuf>,
    /// Sealed records whose stored digest was recomputed and agreed.
    pub sealed_records_verified: usize,
    /// Attestations whose retained output digest was recomputed and agreed.
    pub retained_outputs_verified: usize,
    /// Store-integrity failures found while replaying.
    pub integrity_findings: Vec<IntegrityFinding>,
    /// Whether an oracle capture was supplied at all.
    ///
    /// Without one nothing was compared, so there is no gate to pass.
    pub oracle_supplied: bool,
    /// Store entities (files and retained outputs) matched to an oracle entry
    /// and therefore actually compared.
    ///
    /// This is the population behind every "0 mismatches" claim. Zero
    /// mismatches over zero comparisons is not a pass.
    pub oracle_comparisons_performed: usize,
    /// Oracle keys with no matching store entity on disk.
    pub oracle_entries_unmatched: Vec<String>,
    /// Store entities with no matching oracle key.
    pub store_entries_unmatched: Vec<String>,
    /// Every key this side produced, for reconciliation against the oracle.
    pub store_keys: BTreeSet<String>,
    /// Retained producer outputs digested in the raw-bytes domain.
    pub raw_outputs_digested: usize,
    /// Store files carrying a `__proto__` member.
    ///
    /// The TypeScript pretty writer deletes such a member on rewrite (see the
    /// `key-prototype-names-are-data` case). A non-empty list here means that
    /// data loss is reachable in a real store and must be investigated before
    /// cutover.
    pub proto_member_files: Vec<PathBuf>,
    /// `sha256:`-prefixed references found in store files.
    ///
    /// These belong to a different digest domain and a different algorithm
    /// ([`crate::digest::RawFileSha256Digest`]): they name bytes outside the
    /// store and cannot be recomputed from it. Counted so the boundary between
    /// the two algorithms is measured rather than assumed.
    pub foreign_sha256_references: usize,
}

impl ReplayReport {
    /// Store entities walked: files plus retained producer outputs.
    ///
    /// The denominator [`Self::oracle_comparisons_performed`] must reach.
    #[must_use]
    pub const fn store_entities_walked(&self) -> usize {
        self.files_found + self.raw_outputs_digested
    }

    /// Whether the cutover gate passes.
    ///
    /// Every clause is load-bearing:
    ///
    /// * an oracle was supplied — otherwise nothing was compared;
    /// * at least one comparison was performed — zero mismatches over an empty
    ///   population is not evidence of agreement;
    /// * the comparisons cover every store entity walked, and no oracle entry
    ///   was left over — a key-shape mismatch on either side is a failure to
    ///   compare, and a failure to compare is a gate failure;
    /// * the two implementations agreed on everything compared;
    /// * every sealed record verified against itself.
    #[must_use]
    pub fn gate_passes(&self) -> bool {
        self.oracle_supplied
            && self.oracle_comparisons_performed > 0
            && self.oracle_comparisons_performed == self.store_entities_walked()
            && self.oracle_entries_unmatched.is_empty()
            && self.store_entries_unmatched.is_empty()
            && self.divergences.is_empty()
            && self.integrity_findings.is_empty()
    }
}

/// The oracle key label for a repository path.
///
/// The capture script keys its entries on `path.resolve(argument)`, so this
/// side must produce the same string from the same argument or every lookup
/// misses. Resolution is lexical on both sides — absolute against the working
/// directory, `.` dropped, `..` popped, no trailing separator — so a symlinked
/// path keeps the spelling the caller used, exactly as Node leaves it.
#[must_use]
pub fn oracle_label(repo: &Path) -> String {
    let absolute = if repo.is_absolute() {
        repo.to_path_buf()
    } else {
        std::env::current_dir().map_or_else(|_| repo.to_path_buf(), |cwd| cwd.join(repo))
    };
    let mut normalized = PathBuf::new();
    for component in absolute.components() {
        match component {
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                normalized.pop();
            }
            other => normalized.push(other.as_os_str()),
        }
    }
    normalized.to_string_lossy().into_owned()
}

/// Replay every repository and reconcile the compared population.
///
/// This is the whole gate: [`replay_repository`] answers one repository, and
/// this function is what notices that the oracle and this side were talking
/// about different files.
///
/// # Errors
///
/// Propagates any I/O or parse failure from [`replay_repository`].
pub fn replay(
    repositories: &[PathBuf],
    oracle: Option<&Oracle>,
) -> Result<ReplayReport, StoreError> {
    let mut report = ReplayReport {
        oracle_supplied: oracle.is_some(),
        ..ReplayReport::default()
    };
    for repository in repositories {
        let label = oracle_label(repository);
        replay_repository(repository, &label, oracle, &mut report)?;
    }
    if let Some(oracle) = oracle {
        for key in oracle.files.keys().chain(oracle.raw_files.keys()) {
            if !report.store_keys.contains(key) {
                report.oracle_entries_unmatched.push(key.clone());
            }
        }
    }
    Ok(report)
}

/// How many `sha256:<64 hex>` references appear anywhere in `value`.
fn count_sha256_references(value: &JsonValue) -> usize {
    match value {
        JsonValue::String(text) => {
            usize::from(crate::digest::RawFileSha256Digest::parse_stored(text).is_ok())
        }
        JsonValue::Array(items) => items.iter().map(count_sha256_references).sum(),
        JsonValue::Object(object) => object
            .iter()
            .map(|(_, member)| count_sha256_references(member))
            .sum(),
        _ => 0,
    }
}

/// Whether any object anywhere in `value` carries a `__proto__` member.
fn carries_proto_member(value: &JsonValue) -> bool {
    match value {
        JsonValue::Object(object) => {
            object.contains("__proto__")
                || object
                    .iter()
                    .any(|(_, member)| carries_proto_member(member))
        }
        JsonValue::Array(items) => items.iter().any(carries_proto_member),
        _ => false,
    }
}

/// Recursively collect `*.json` files under `root`, plus every `output.bin`.
fn collect_store_files(root: &Path, out: &mut Vec<PathBuf>) -> Result<(), StoreError> {
    let entries = match std::fs::read_dir(root) {
        Ok(entries) => entries,
        Err(source) if source.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(source) => {
            return Err(StoreError::Io {
                operation: "read directory",
                path: root.to_path_buf(),
                source,
            });
        }
    };
    let mut children: Vec<PathBuf> = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|source| StoreError::Io {
            operation: "read directory entry",
            path: root.to_path_buf(),
            source,
        })?;
        children.push(entry.path());
    }
    children.sort();
    for child in children {
        if child.is_dir() {
            collect_store_files(&child, out)?;
        } else if child.extension().is_some_and(|ext| ext == "json")
            || child.file_name().is_some_and(|name| name == "output.bin")
        {
            out.push(child);
        }
    }
    Ok(())
}

/// Replay one repository's store, comparing against `oracle` when supplied.
///
/// `label` must be [`oracle_label`] of `repo`, or every oracle lookup misses.
/// Prefer [`replay`], which supplies the label and reconciles oracle entries
/// this side never saw.
///
/// # Errors
///
/// Any I/O failure reading the store, or a document that nests past
/// [`crate::json::MAX_NESTING_DEPTH`].
pub fn replay_repository(
    repo: &Path,
    label: &str,
    oracle: Option<&Oracle>,
    report: &mut ReplayReport,
) -> Result<(), StoreError> {
    let root = store_root(repo);
    if !root.is_dir() {
        return Ok(());
    }
    report.repositories.push(repo.to_path_buf());
    let mut files = Vec::new();
    collect_store_files(&root, &mut files)?;
    let raw_digests = replay_raw_outputs(&files, &root, label, oracle, report)?;

    for path in files {
        if path.file_name().is_some_and(|name| name == "output.bin") {
            continue;
        }
        report.files_found += 1;
        let key = format!("{label}/{}", relative_key(&path, &root));
        report.store_keys.insert(key.clone());
        let oracle_file = oracle.and_then(|oracle| oracle.files.get(&key));
        match oracle_file {
            Some(_) => report.oracle_comparisons_performed += 1,
            None if oracle.is_some() => report.store_entries_unmatched.push(key.clone()),
            None => {}
        }
        replay_store_file(&path, &key, oracle_file, &raw_digests, report)?;
    }

    Ok(())
}

/// The store-relative, forward-slash spelling a key is built from.
fn relative_key(path: &Path, root: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

/// Digest every retained raw output, comparing each against the oracle.
///
/// Runs before the record pass because an attestation's retained-output check
/// reads the digest of the `output.bin` beside it.
///
/// # Errors
///
/// Refuses on any I/O failure reading a retained output.
fn replay_raw_outputs(
    files: &[PathBuf],
    root: &Path,
    label: &str,
    oracle: Option<&Oracle>,
    report: &mut ReplayReport,
) -> Result<BTreeMap<PathBuf, String>, StoreError> {
    let mut raw_digests: BTreeMap<PathBuf, String> = BTreeMap::new();
    for path in files {
        if path.file_name().is_none_or(|name| name != "output.bin") {
            continue;
        }
        let bytes = crate::store::read_bytes(path)?;
        let digest = digest_raw_bytes(&bytes).as_hex().to_owned();
        report.raw_outputs_digested += 1;
        report.digests_replayed += 1;
        let key = format!("{label}/{}", relative_key(path, root));
        match oracle.and_then(|oracle| oracle.raw_files.get(&key)) {
            Some(expected) => {
                report.oracle_comparisons_performed += 1;
                if expected != &digest {
                    report.divergences.push(Divergence {
                        key: key.clone(),
                        kind: DivergenceKind::NodeDigest,
                        typescript: expected.clone(),
                        rust: digest.clone(),
                    });
                }
            }
            None if oracle.is_some() => report.store_entries_unmatched.push(key.clone()),
            None => {}
        }
        report.store_keys.insert(key);
        raw_digests.insert(path.clone(), digest);
    }
    Ok(raw_digests)
}

/// Replay one stored document: parse it, re-serialize it, digest its nodes, and
/// compare every one of those against the oracle's record for the same key.
///
/// # Errors
///
/// Refuses on any I/O failure reading the document, and on a document that
/// nests past [`crate::json::MAX_NESTING_DEPTH`] when it is re-serialized or
/// walked.
fn replay_store_file(
    path: &Path,
    key: &str,
    oracle_file: Option<&OracleFile>,
    raw_digests: &BTreeMap<PathBuf, String>,
    report: &mut ReplayReport,
) -> Result<(), StoreError> {
    let bytes = crate::store::read_bytes(path)?;
    let parsed = match parse_strict_json(&bytes) {
        Ok(value) => {
            report.files_parsed += 1;
            Some(value)
        }
        Err(error) => {
            report.parse_refusals.push(IntegrityFinding {
                path: path.to_path_buf(),
                detail: format!("{} ({})", error, error.code()),
            });
            None
        }
    };

    if let Some(expected) = oracle_file
        && expected.parsed != parsed.is_some()
    {
        report.divergences.push(Divergence {
            key: key.to_owned(),
            kind: DivergenceKind::Acceptance,
            typescript: if expected.parsed {
                "accepted".to_owned()
            } else {
                expected
                    .error
                    .clone()
                    .unwrap_or_else(|| "refused".to_owned())
            },
            rust: if parsed.is_some() {
                "accepted".to_owned()
            } else {
                "refused".to_owned()
            },
        });
    }

    let Some(value) = parsed else { return Ok(()) };

    let form = StoreForm::for_path(path);
    let serialized = form.serialize(&value)?;
    let identical = serialized == bytes;
    if identical {
        report.round_trip_identical += 1;
    } else {
        report.round_trip_divergent.push(path.to_path_buf());
    }

    if carries_proto_member(&value) {
        report.proto_member_files.push(path.to_path_buf());
    }
    report.foreign_sha256_references += count_sha256_references(&value);

    let serialization_digest = digest_raw_bytes(&serialized).as_hex().to_owned();
    let digests = node_digests(&value)?;
    report.digests_replayed += digests.len();

    if let Some(expected) = oracle_file {
        compare_against_oracle(
            key,
            expected,
            &serialization_digest,
            identical,
            &digests,
            report,
        );
    }

    if let JsonValue::Object(object) = &value {
        verify_self_consistency(path, object, raw_digests, report);
    }
    Ok(())
}

/// Compare one document's replayed values against the oracle's record of them.
fn compare_against_oracle(
    key: &str,
    expected: &OracleFile,
    serialization_digest: &str,
    identical: bool,
    digests: &[String],
    report: &mut ReplayReport,
) {
    if let Some(expected_digest) = &expected.serialization_digest
        && expected_digest != serialization_digest
    {
        report.divergences.push(Divergence {
            key: key.to_owned(),
            kind: DivergenceKind::Serialization,
            typescript: expected_digest.clone(),
            rust: serialization_digest.to_owned(),
        });
    }
    if let Some(expected_round_trip) = expected.round_trip_identical
        && expected_round_trip != identical
    {
        report.divergences.push(Divergence {
            key: key.to_owned(),
            kind: DivergenceKind::RoundTrip,
            typescript: expected_round_trip.to_string(),
            rust: identical.to_string(),
        });
    }
    if expected.node_digests.len() != digests.len() {
        report.divergences.push(Divergence {
            key: key.to_owned(),
            kind: DivergenceKind::NodeCount,
            typescript: expected.node_digests.len().to_string(),
            rust: digests.len().to_string(),
        });
    }
    for (index, (left, right)) in expected.node_digests.iter().zip(digests).enumerate() {
        if left != right {
            report.divergences.push(Divergence {
                key: format!("{key}#node{index}"),
                kind: DivergenceKind::NodeDigest,
                typescript: left.clone(),
                rust: right.clone(),
            });
        }
    }
}

/// Self-consistency: a sealed record must still match its own digest, and an
/// attestation must still match the retained output beside it. Neither check
/// needs an oracle — they are internal to the store.
fn verify_self_consistency(
    path: &Path,
    object: &crate::json::value::JsonObject,
    raw_digests: &BTreeMap<PathBuf, String>,
    report: &mut ReplayReport,
) {
    if object.contains(crate::digest::DIGEST_MEMBER) && is_sealed_record(path) {
        match verify_record_digest(object) {
            Ok(digest) => {
                report.sealed_records_verified += 1;
                report.digests_replayed += 1;
                if let Some(stem) = path.file_stem().and_then(|stem| stem.to_str())
                    && stem != digest.as_hex()
                    && stem.len() == 64
                {
                    report.integrity_findings.push(IntegrityFinding {
                        path: path.to_path_buf(),
                        detail: format!("path names {stem}, record digests to {}", digest.as_hex()),
                    });
                }
            }
            Err(error) => report.integrity_findings.push(IntegrityFinding {
                path: path.to_path_buf(),
                detail: format!("{} ({})", error, error.code()),
            }),
        }
    }
    if path
        .file_name()
        .is_some_and(|name| name == "attestation.json")
    {
        verify_retained_output(path, object, raw_digests, report);
    }
}

/// A sealed record lives under `change-assurance/records` or `attestations`.
fn is_sealed_record(path: &Path) -> bool {
    path.components()
        .any(|component| component.as_os_str() == "change-assurance")
}

/// An attestation names a retained output beside it; check both its raw-domain
/// digest and its size against the bytes actually on disk.
fn verify_retained_output(
    path: &Path,
    attestation: &crate::json::value::JsonObject,
    raw_digests: &BTreeMap<PathBuf, String>,
    report: &mut ReplayReport,
) {
    let Some(JsonValue::Object(retained)) = attestation.get("retained_output") else {
        return;
    };
    let Some(parent) = path.parent() else {
        return;
    };
    let output_path = parent.join("output.bin");
    let Some(actual) = raw_digests.get(&output_path) else {
        report.integrity_findings.push(IntegrityFinding {
            path: output_path,
            detail: "attestation names a retained output that is not on disk".to_owned(),
        });
        return;
    };
    match retained.get("digest").and_then(JsonValue::as_str) {
        Some(stored) if stored == actual => report.retained_outputs_verified += 1,
        Some(stored) => report.integrity_findings.push(IntegrityFinding {
            path: output_path.clone(),
            detail: format!(
                "retained output digest mismatch: attestation says {stored}, bytes digest to {actual}"
            ),
        }),
        None => report.integrity_findings.push(IntegrityFinding {
            path: output_path.clone(),
            detail: "attestation carries no retained_output.digest".to_owned(),
        }),
    }
    if let (Some(size), Ok(bytes)) = (
        retained.get("size_bytes").and_then(JsonValue::as_f64),
        std::fs::metadata(&output_path).map(|meta| meta.len()),
    ) {
        // Sizes persist as doubles. Compare in the double domain rather than
        // casting the double down to an integer, so a value beyond 2^53 is
        // reported as a mismatch instead of silently truncated into agreement.
        #[allow(clippy::cast_precision_loss, reason = "compared, never stored")]
        let actual_as_double = bytes as f64;
        // Exact equality is the intent, not an approximation. A stored size is
        // an integer that happens to persist as a double; "close enough" is
        // precisely the answer a store-integrity check must not give.
        #[allow(
            clippy::float_cmp,
            reason = "an integral size must match exactly; a tolerance would hide corruption"
        )]
        let differs = size != actual_as_double;
        if differs {
            report.integrity_findings.push(IntegrityFinding {
                path: output_path,
                detail: format!(
                    "retained output size mismatch: attestation says {size}, file holds {bytes}"
                ),
            });
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::indexing_slicing,
        clippy::panic,
        reason = "in a test, a panic IS the failure report; the production lints stand"
    )]
    use super::{StoreForm, node_digests, walk_nodes};
    use crate::json::parse::parse_strict_json_str;
    use std::path::Path;

    #[test]
    fn tc_380_store_form_is_selected_by_family_not_by_extension() {
        assert_eq!(
            StoreForm::for_path(Path::new("spec/evidence/bindings.json")),
            StoreForm::Pretty
        );
        assert_eq!(
            StoreForm::for_path(Path::new("spec/evidence/change-assurance/records/a.json")),
            StoreForm::Jcs
        );
    }

    #[test]
    fn tc_380_node_walk_is_preorder_with_members_in_jcs_order() {
        let value = parse_strict_json_str(r#"{"b":[1,2],"a":null}"#).expect("parses");
        let mut nodes = Vec::new();
        walk_nodes(&value, &mut nodes);
        // root, "a" -> null, "b" -> array, 1, 2
        assert_eq!(nodes.len(), 5);
        assert_eq!(nodes[1].type_name(), "null");
        assert_eq!(nodes[2].type_name(), "array");
        assert_eq!(node_digests(&value).expect("shallow").len(), 5);
    }
}
