// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! What the runner checks before it spends a single call (PR #620 review and
//! re-review, findings 3a, 4 and 5). Both the live runner and the offline
//! tests call [`authorize_run`], so deleting a check from it fails an
//! offline test, not only a live run nobody can afford to repeat.
//!
//! - Every `variant@version` must carry a request-digest pin that matches
//!   its wording ([`check_request_pins`]): a wording change without a version
//!   bump, or a bumped version nobody pinned, is refused.
//! - No variant may be past the dev cap of [`MAX_DEV_VERSIONS`] versions.
//! - On the held-out split: the flag and the seal
//!   ([`corpus::authorize_heldout`]), a committed selection entry covering
//!   every variant ([`corpus::check_heldout_selected`]), and no unexplained
//!   rerun ([`corpus::check_heldout_rerun`]).

use std::path::Path;

use serde_json::Value;

use super::corpus::{self, Row, Source, Split};
use super::fixtures;
use super::variant::Variant;

/// The most versions a variant family may have on dev (PLAT-1024 rule 5,
/// MP-240). A version past it is refused, not merely reported.
pub(crate) const MAX_DEV_VERSIONS: u32 = 5;

/// The pinned digest of every registered variant's question text (the
/// questions of every ask, instructions and labels, never the state) on the
/// canonical row of each mode it runs in: [`fixtures::four_modes`]'s row for
/// that mode. A changed wording with an unchanged `version`, or a bumped
/// version with no pin, is refused by [`check_request_pins`], which the
/// runner calls before any request; the offline gate
/// `tc_1027_request_digest_is_pinned_per_variant_version` also holds this
/// table equal to the registry's digests, and prints the digests to pin.
pub(crate) const REQUEST_DIGEST_PINS: &[(&str, &str, &str)] = &[
    (
        "B0@v1",
        "RTC",
        "sha256:06b0a47799e9fa6c7059d91db3b0457eb5ed578337128b9a619d41883fe6ce72",
    ),
    (
        "S0@v1",
        "RTC",
        "sha256:06b0a47799e9fa6c7059d91db3b0457eb5ed578337128b9a619d41883fe6ce72",
    ),
    (
        "E0@v1",
        "RTC",
        "sha256:06b0a47799e9fa6c7059d91db3b0457eb5ed578337128b9a619d41883fe6ce72",
    ),
    (
        "T0@v1",
        "RTC",
        "sha256:06b0a47799e9fa6c7059d91db3b0457eb5ed578337128b9a619d41883fe6ce72",
    ),
    (
        "C0@v1",
        "R",
        "sha256:61a9cd3492f4f9655ea3422da31e30bf0df0e89f7416f0fb326edaa1fd8d6ea4",
    ),
    (
        "C0@v1",
        "RT",
        "sha256:cb993257058030361d99de6e5fe955fff5b6891a0f0bc73660d64bdef421e668",
    ),
    (
        "C0@v1",
        "RC",
        "sha256:9c06b2a363798254e4c0890cb10c9eb8e1e8ebdf817c06dbc299defffa9a0458",
    ),
    (
        "C0@v1",
        "RTC",
        "sha256:b99912ed3c09d7f9094e23ffa5f3618c328eccd439a9574eb0ac589e3a6a9feb",
    ),
    (
        "S1@v1",
        "RTC",
        "sha256:af4001a1b2a994250dd42fffc002d81ff3ef0359b2ad0a715cb8c625017047fa",
    ),
    (
        "S1-RT@v1",
        "RT",
        "sha256:a3b3c8666144b550a8bfd64d403edc05ef6b39316509c95778149eef8f4e3219",
    ),
    (
        "S1-RC@v1",
        "RC",
        "sha256:b54bb4cdaffaab9f1da35b135aacaccc6d5f97621ab56349ee9f546bfbc9c7d4",
    ),
    (
        "S2@v1",
        "RTC",
        "sha256:087dae08a2e4946f8eaf2c7b694566ad2aa3e22ad9cce8f4b316870b9051b7c3",
    ),
    (
        "S2-RT@v1",
        "RT",
        "sha256:35dcda595da52ba10aa576fe22a91025d5d85274b63d834d04fee7fa8e20597c",
    ),
    (
        "S2-RC@v1",
        "RC",
        "sha256:92b7246971106f34f99681e33a9423175612e3de377bce3ed428397ce7e25640",
    ),
    (
        "S2M@v1",
        "RTC",
        "sha256:087dae08a2e4946f8eaf2c7b694566ad2aa3e22ad9cce8f4b316870b9051b7c3",
    ),
    (
        "S2M-RT@v1",
        "RT",
        "sha256:35dcda595da52ba10aa576fe22a91025d5d85274b63d834d04fee7fa8e20597c",
    ),
    (
        "S2M-RC@v1",
        "RC",
        "sha256:92b7246971106f34f99681e33a9423175612e3de377bce3ed428397ce7e25640",
    ),
    (
        "S3@v1",
        "RT",
        "sha256:2690765d45c33f7d8b3f49e8c475221a84c75936171288b0b070251fe15db896",
    ),
    (
        "S3@v1",
        "RC",
        "sha256:2690765d45c33f7d8b3f49e8c475221a84c75936171288b0b070251fe15db896",
    ),
    (
        "S3@v1",
        "RTC",
        "sha256:2690765d45c33f7d8b3f49e8c475221a84c75936171288b0b070251fe15db896",
    ),
    (
        "E0-RC@v1",
        "RC",
        "sha256:9e632aa6cc9f0ea6cd547ccacb845dbed0f2e81c6cf2304e57d0ebaf6ed2543d",
    ),
    (
        "E1@v1",
        "RC",
        "sha256:8a1fffbde7e0b0619a6ac4b1f46fff30d1e42653f8b29c4e277e671909798b94",
    ),
    (
        "E1@v1",
        "RTC",
        "sha256:8a1fffbde7e0b0619a6ac4b1f46fff30d1e42653f8b29c4e277e671909798b94",
    ),
    (
        "E2@v1",
        "RC",
        "sha256:452778542aa7637309a22a7cc416725db23a3e640b58608c1b131cdd4e23f469",
    ),
    (
        "E2@v1",
        "RTC",
        "sha256:452778542aa7637309a22a7cc416725db23a3e640b58608c1b131cdd4e23f469",
    ),
    (
        "E4@v1",
        "RC",
        "sha256:8a1fffbde7e0b0619a6ac4b1f46fff30d1e42653f8b29c4e277e671909798b94",
    ),
    (
        "E4@v1",
        "RTC",
        "sha256:8a1fffbde7e0b0619a6ac4b1f46fff30d1e42653f8b29c4e277e671909798b94",
    ),
];

/// The digest of `variant`'s questions on `row`.
///
/// # Errors
/// When the questions do not serialize.
pub(crate) fn request_digest(variant: &Variant, row: &Row) -> Result<String, String> {
    let questions = (variant.asks)(row)
        .iter()
        .map(|ask| serde_json::to_value(&ask.request.questions))
        .collect::<Result<Vec<Value>, _>>()
        .map_err(|error| format!("{}: {error}", variant.label()))?;
    let text = serde_json::to_string(&questions)
        .map_err(|error| format!("{}: {error}", variant.label()))?;
    Ok(quoin_store::digest_bytes_sha256(text.as_bytes()).to_stored())
}

/// `(label, mode, digest)` for every variant on the canonical row of each
/// mode it runs in, in variant then mode order.
///
/// # Errors
/// When the canonical rows or a variant's questions do not serialize.
pub(crate) fn request_digests(
    variants: &[&Variant],
) -> Result<Vec<(String, &'static str, String)>, String> {
    let canonical = fixtures::four_modes()?;
    let mut digests = Vec::new();
    for variant in variants {
        for row in canonical.rows.iter().filter(|row| variant.applies_to(row)) {
            digests.push((
                variant.label(),
                row.mode.as_str(),
                request_digest(variant, row)?,
            ));
        }
    }
    Ok(digests)
}

/// Refuses any `variant@version` whose wording has no pin in
/// [`REQUEST_DIGEST_PINS`], or differs from its pin.
///
/// # Errors
/// Naming every unpinned or mismatched `(label, mode)`.
pub(crate) fn check_request_pins(variants: &[&Variant]) -> Result<(), String> {
    let mut problems = Vec::new();
    for (label, mode, digest) in request_digests(variants)? {
        match REQUEST_DIGEST_PINS
            .iter()
            .find(|(pinned, pinned_mode, _)| *pinned == label && *pinned_mode == mode)
        {
            None => problems.push(format!(
                "{label} in {mode} has no request-digest pin; pin {digest} before it runs"
            )),
            Some((_, _, pinned)) if *pinned != digest => problems.push(format!(
                "{label} in {mode} asks wording {digest}, but its pin is {pinned}; a wording \
                 change needs a version bump"
            )),
            Some(_) => {}
        }
    }
    if problems.is_empty() {
        Ok(())
    } else {
        Err(problems.join("\n"))
    }
}

/// Refuses any variant past [`MAX_DEV_VERSIONS`].
///
/// # Errors
/// Naming every such variant.
pub(crate) fn check_version_cap(variants: &[&Variant]) -> Result<(), String> {
    let over: Vec<String> = variants
        .iter()
        .filter(|variant| variant.version > MAX_DEV_VERSIONS)
        .map(|variant| variant.label())
        .collect();
    if over.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "{over:?} are past the dev cap of {MAX_DEV_VERSIONS} versions per family; the \
             family's last capped version is its final one"
        ))
    }
}

/// The run's settings the preflight reads.
#[derive(Debug, Clone, Copy)]
pub(crate) struct RunGate<'a> {
    /// The split to run.
    pub(crate) split: Split,
    /// The value of [`corpus::HELDOUT_ENV`].
    pub(crate) heldout_flag: Option<&'a str>,
    /// The value of [`corpus::HELDOUT_RERUN_ENV`].
    pub(crate) rerun_reason: Option<&'a str>,
    /// The held-out selection file ([`corpus::heldout_selection_path`] in a
    /// real run).
    pub(crate) selection: &'a Path,
    /// The held-out run log ([`corpus::heldout_log_path`] in a real run).
    pub(crate) log: &'a Path,
}

/// Every check a run must pass before its first request. Returns each
/// sealed source's held-out digest (empty on dev).
///
/// # Errors
/// The first check that fails, naming what it refused.
pub(crate) fn authorize_run(
    gate: &RunGate<'_>,
    variants: &[&Variant],
    sources: &[&Source],
) -> Result<Vec<String>, String> {
    check_version_cap(variants)?;
    check_request_pins(variants)?;
    if gate.split != Split::Heldout {
        return Ok(Vec::new());
    }
    let seals = corpus::authorize_heldout(gate.heldout_flag, sources)?;
    let text = std::fs::read_to_string(gate.selection)
        .map_err(|error| format!("{}: {error}", gate.selection.display()))?;
    let selection = corpus::parse_selection(&text)?;
    let chosen: Vec<(&str, u32)> = variants
        .iter()
        .map(|variant| (variant.id, variant.version))
        .collect();
    corpus::check_heldout_selected(&selection, &chosen)?;
    let labels: Vec<String> = variants.iter().map(|variant| variant.label()).collect();
    corpus::check_heldout_rerun(gate.log, &labels, gate.rerun_reason)?;
    Ok(seals)
}
