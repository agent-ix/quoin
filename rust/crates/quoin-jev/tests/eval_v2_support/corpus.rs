// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Corpus v2: the schema, the loaders, validation, by-reference
//! materialization and the held-out seal (PLAT-1027).
//!
//! # The schema is shared
//!
//! PLAT-1025 writes `fixtures/eval-v2/corpus.json`; PLAT-1026 writes the
//! external corpus. Both read these types. Every struct denies unknown fields,
//! so a field one side adds without the other is a parse error, not a silent
//! drop.
//!
//! # Two sources
//!
//! - **In-repo**, [`in_repo_corpus_path`]. Absent until PLAT-1025 lands;
//!   callers treat `Ok(None)` as "nothing to measure", never as a pass.
//! - **External**, the path in `QUOIN_JEV_EXTERNAL_CORPUS`, optional. Its rows
//!   may set `ref` instead of embedding bodies: the harness reads the named
//!   files under `QUOIN_JEV_EXTERNAL_ROOT/<repo name>`, checks each file's
//!   sha256 against the row, applies the mutation patch, and cuts the named
//!   `fn` out. `ref.commit` is informational: the digest over the bytes is
//!   what is checked, so a checkout at another commit with identical bytes is
//!   accepted, and one with different bytes is refused.
//!
//! # The held-out seal
//!
//! A corpus's held-out rows are sealed by `heldout.sha256` in the corpus
//! file's own directory: the sha256 (hex, optionally `sha256:`-prefixed) of
//! the RFC 8785 canonical JSON of the array of held-out row objects, in file
//! order, exactly as they appear in the corpus file (before any
//! materialization). [`heldout_digest`] computes it.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::path::{Component, Path, PathBuf};

use quoin_store::{JsonValue, canonical_bytes, digest_bytes_sha256, parse_strict_json_str};
use serde::{Deserialize, Serialize};

use super::keys::{Mode, NO, YES, spec};
use super::patch::apply_unified_patch;
use super::units::extract_rust_fn;

/// The only schema id this harness reads.
pub(crate) const SCHEMA: &str = "quoin-jev.eval-corpus/v2";
/// The env var naming an external corpus file.
pub(crate) const EXTERNAL_CORPUS_ENV: &str = "QUOIN_JEV_EXTERNAL_CORPUS";
/// The env var naming the directory holding the sibling checkouts.
pub(crate) const EXTERNAL_ROOT_ENV: &str = "QUOIN_JEV_EXTERNAL_ROOT";
/// The env var that must be `1` for a held-out run.
pub(crate) const HELDOUT_ENV: &str = "QUOIN_JEV_HELDOUT";
/// The seal's file name, beside its corpus.
pub(crate) const SEAL_FILE: &str = "heldout.sha256";

/// The whole corpus file.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct CorpusFile {
    /// Must be [`SCHEMA`].
    pub(crate) schema: String,
    /// The written random sampling rule.
    pub(crate) sampling_rule: String,
    /// The sampling seed.
    pub(crate) seed: u64,
    /// The commit the population was drawn at. Informational.
    pub(crate) source_commit: String,
    /// Every row.
    pub(crate) rows: Vec<Row>,
}

/// Which split a row belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum Split {
    /// Variants are tuned here.
    Dev,
    /// Sealed; run once, behind [`HELDOUT_ENV`].
    Heldout,
}

impl Split {
    /// The corpus spelling.
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::Dev => "dev",
            Self::Heldout => "heldout",
        }
    }
}

/// One corpus row.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Row {
    /// e.g. `EV2-0001`.
    pub(crate) id: String,
    /// Which artifacts the row carries.
    pub(crate) mode: Mode,
    /// Dev or held-out.
    pub(crate) split: Split,
    /// The sampling strata.
    pub(crate) strata: Strata,
    /// The requirement, always present.
    pub(crate) requirement: Requirement,
    /// The test, in `RT` and `RTC` rows.
    pub(crate) test: Option<TestArtifact>,
    /// The code, in `RC` and `RTC` rows.
    pub(crate) code: Option<CodeArtifact>,
    /// Where an external row's files live, instead of embedded bodies.
    #[serde(rename = "ref")]
    pub(crate) reference: Option<Reference>,
    /// The scratch edit that made this row, when it is a mutant.
    pub(crate) mutation: Option<Mutation>,
    /// Ground truth, by question key.
    pub(crate) truth: BTreeMap<String, Truth>,
}

/// The strata a row was sampled under.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Strata {
    /// The requirement id.
    pub(crate) fr_id: String,
    /// The requirement's kind.
    pub(crate) req_kind: String,
    /// The test's kind, when there is a test.
    pub(crate) test_kind: Option<String>,
    /// The crate (or repo) the row came from.
    #[serde(rename = "crate")]
    pub(crate) crate_name: String,
    /// The requirement's EARS pattern, when known.
    pub(crate) ears_pattern: Option<String>,
}

/// The requirement side of a row.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Requirement {
    /// e.g. `FR-068`.
    pub(crate) fr_id: String,
    /// The acceptance criterion, when the row is about one.
    pub(crate) ac_id: Option<String>,
    /// The requirement's normative statement.
    pub(crate) statement: String,
    /// The acceptance criterion's text.
    pub(crate) ac_text: Option<String>,
    /// Further requirement prose (description, behaviour).
    pub(crate) context: Option<String>,
}

/// The test side of a row.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct TestArtifact {
    /// The test file.
    pub(crate) path: String,
    /// The test function.
    pub(crate) fn_name: String,
    /// The test's source. Empty on an external row until materialized.
    #[serde(default)]
    pub(crate) body: String,
}

/// The code side of a row.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct CodeArtifact {
    /// The source file.
    pub(crate) path: String,
    /// The covered symbol; `Type::method` is matched on its last segment.
    pub(crate) symbol: String,
    /// The symbol's source. Empty on an external row until materialized.
    #[serde(default)]
    pub(crate) body: String,
}

/// Where an external row's files live.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Reference {
    /// e.g. `agent-ix/quire-rs`; the checkout is `<root>/<last segment>`.
    pub(crate) repo: String,
    /// Informational: the digests are what is checked.
    pub(crate) commit: String,
    /// Role (`requirement` / `test` / `code`) to a repo-relative path.
    pub(crate) paths: BTreeMap<String, String>,
    /// Role to the sha256 of that file's pristine bytes.
    pub(crate) sha256: BTreeMap<String, String>,
}

/// Which artifact a mutation edited.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum Target {
    /// The requirement text.
    Requirement,
    /// The test.
    Test,
    /// The code.
    Code,
}

impl Target {
    const fn role(self) -> &'static str {
        match self {
            Self::Requirement => "requirement",
            Self::Test => "test",
            Self::Code => "code",
        }
    }
}

/// A scratch edit, stored as a patch, never committed to source.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Mutation {
    /// The mutation's id.
    pub(crate) id: String,
    /// What it edited.
    pub(crate) target: Target,
    /// Its kind, e.g. `drop_assertion`.
    pub(crate) kind: String,
    /// What it does.
    pub(crate) description: String,
    /// The unified diff. Provenance on an in-repo row; applied on an
    /// external one.
    pub(crate) patch: String,
}

/// A truth answer: a label, or a bool read as `yes`/`no`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub(crate) enum TruthAnswer {
    /// `true` / `false`.
    Bool(bool),
    /// A label from the key's answer space.
    Label(String),
}

impl TruthAnswer {
    /// The label this answer grades as.
    pub(crate) fn label(&self) -> String {
        match self {
            Self::Bool(true) => YES.to_owned(),
            Self::Bool(false) => NO.to_owned(),
            Self::Label(label) => label.clone(),
        }
    }
}

/// How a truth label was established.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum TruthKind {
    /// Verified by running the owning test.
    Mechanical,
    /// True because of the change that made the row.
    ByConstruction,
    /// Two independent agent labels that agree.
    AgentDual,
    /// Two agent labels that disagree; both readings kept.
    AgentContested,
}

/// The coarse truth-kind groups every report is broken down by.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum KindGroup {
    /// Mechanical truth.
    Mechanical,
    /// By-construction truth.
    ByConstruction,
    /// Agent-labelled truth, dual or contested.
    Agent,
}

impl KindGroup {
    /// Every group, in report order.
    pub(crate) const ALL: [Self; 3] = [Self::Mechanical, Self::ByConstruction, Self::Agent];

    /// The report's name for the group. The agent group always says so.
    pub(crate) const fn label(self) -> &'static str {
        match self {
            Self::Mechanical => "mechanical",
            Self::ByConstruction => "by-construction",
            Self::Agent => "AGENT-LABELLED",
        }
    }
}

impl TruthKind {
    /// This kind's report group.
    pub(crate) const fn group(self) -> KindGroup {
        match self {
            Self::Mechanical => KindGroup::Mechanical,
            Self::ByConstruction => KindGroup::ByConstruction,
            Self::AgentDual | Self::AgentContested => KindGroup::Agent,
        }
    }
}

/// One key's ground truth on one row.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Truth {
    /// The primary answer.
    pub(crate) answer: TruthAnswer,
    /// How it was established.
    pub(crate) kind: TruthKind,
    /// Other defensible answers. A prediction matching one counts as
    /// agreement, as in every earlier Jev corpus.
    #[serde(default)]
    pub(crate) alternatives: Vec<TruthAnswer>,
    /// Why, citing the row's own text.
    pub(crate) rationale: String,
}

/// Where a corpus came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Origin {
    /// `fixtures/eval-v2/corpus.json`.
    InRepo,
    /// `QUOIN_JEV_EXTERNAL_CORPUS`.
    External,
}

/// One loaded corpus: its path, its raw text (the seal is over the text, not
/// over a re-serialization) and the parsed file.
#[derive(Debug, Clone)]
pub(crate) struct Source {
    /// Where it came from.
    pub(crate) origin: Origin,
    /// Its file.
    pub(crate) path: PathBuf,
    /// The file's text, as read.
    pub(crate) text: String,
    /// The parsed file. External rows are materialized in place.
    pub(crate) file: CorpusFile,
}

/// `fixtures/eval-v2/corpus.json`.
pub(crate) fn in_repo_corpus_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/eval-v2/corpus.json")
}

/// The held-out run log, one JSON line per held-out run.
pub(crate) fn heldout_log_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/eval-v2/heldout-runs.jsonl")
}

/// Parses a corpus from its text.
///
/// # Errors
/// When the text is not the schema (unknown fields included).
pub(crate) fn parse(text: &str) -> Result<CorpusFile, String> {
    serde_json::from_str(text).map_err(|error| format!("corpus does not parse: {error}"))
}

/// Reads and parses one corpus file.
///
/// # Errors
/// When it cannot be read or does not parse.
pub(crate) fn read_source(path: &Path, origin: Origin) -> Result<Source, String> {
    let text =
        std::fs::read_to_string(path).map_err(|error| format!("{}: {error}", path.display()))?;
    let file = parse(&text).map_err(|error| format!("{}: {error}", path.display()))?;
    Ok(Source {
        origin,
        path: path.to_owned(),
        text,
        file,
    })
}

/// The in-repo corpus, or `Ok(None)` when PLAT-1025 has not landed it yet.
///
/// # Errors
/// When the file exists but cannot be read or parsed.
pub(crate) fn load_in_repo() -> Result<Option<Source>, String> {
    let path = in_repo_corpus_path();
    if !path.exists() {
        return Ok(None);
    }
    read_source(&path, Origin::InRepo).map(Some)
}

/// The external corpus named by `corpus`, with every by-reference row
/// materialized from `root`. `Ok(None)` when `corpus` is `None`.
///
/// # Errors
/// When the file cannot be read or parsed, or any by-reference row cannot be
/// materialized (no root, missing file, digest mismatch, patch or symbol
/// failure). One bad row fails the whole load: a corpus that silently loses
/// rows changes its own denominator.
pub(crate) fn load_external(
    corpus: Option<&Path>,
    root: Option<&Path>,
) -> Result<Option<Source>, String> {
    let Some(corpus) = corpus else {
        return Ok(None);
    };
    let mut source = read_source(corpus, Origin::External)?;
    for row in &mut source.file.rows {
        if row.reference.is_some() {
            let root = root.ok_or_else(|| {
                format!(
                    "{}: row {} is by reference but {EXTERNAL_ROOT_ENV} is unset",
                    corpus.display(),
                    row.id
                )
            })?;
            materialize(row, root).map_err(|error| format!("row {}: {error}", row.id))?;
        }
    }
    Ok(Some(source))
}

/// [`load_external`] from the two env vars.
///
/// # Errors
/// As [`load_external`].
pub(crate) fn load_external_from_env() -> Result<Option<Source>, String> {
    let corpus = std::env::var_os(EXTERNAL_CORPUS_ENV).map(PathBuf::from);
    let root = std::env::var_os(EXTERNAL_ROOT_ENV).map(PathBuf::from);
    load_external(corpus.as_deref(), root.as_deref())
}

/// A repo-relative path that stays inside the repo: relative, and no `..`.
fn contained(relative: &str) -> Result<&Path, String> {
    let path = Path::new(relative);
    let escapes = path
        .components()
        .any(|part| !matches!(part, Component::Normal(_) | Component::CurDir));
    if relative.is_empty() || escapes {
        return Err(format!(
            "path {relative:?} is not a plain repo-relative path"
        ));
    }
    Ok(path)
}

/// Fills an external row's bodies from its checkout.
///
/// # Errors
/// As [`load_external`].
pub(crate) fn materialize(row: &mut Row, root: &Path) -> Result<(), String> {
    let Some(reference) = row.reference.clone() else {
        return Ok(());
    };
    let repo = reference
        .repo
        .rsplit('/')
        .next()
        .filter(|name| !name.is_empty())
        .ok_or_else(|| format!("ref.repo {:?} names no repository", reference.repo))?;
    let checkout = root.join(repo);
    for (role, relative) in &reference.paths {
        if !matches!(role.as_str(), "requirement" | "test" | "code") {
            return Err(format!("ref.paths has unknown role {role:?}"));
        }
        let path = checkout.join(contained(relative)?);
        let bytes = std::fs::read(&path).map_err(|error| format!("{}: {error}", path.display()))?;
        let expected = reference
            .sha256
            .get(role)
            .ok_or_else(|| format!("ref.sha256 has no digest for role {role:?}"))?;
        let expected = expected.strip_prefix("sha256:").unwrap_or(expected);
        let actual = digest_bytes_sha256(&bytes);
        if !actual.as_hex().eq_ignore_ascii_case(expected) {
            return Err(format!(
                "{}: content digest {} does not match the row's {expected}",
                path.display(),
                actual.as_hex()
            ));
        }
        let mut text = String::from_utf8(bytes)
            .map_err(|error| format!("{}: not UTF-8: {error}", path.display()))?;
        if let Some(mutation) = row.mutation.as_ref().filter(|m| m.target.role() == role) {
            text = apply_unified_patch(&text, &mutation.patch)
                .map_err(|error| format!("mutation {}: {error}", mutation.id))?;
        }
        match role.as_str() {
            "test" => {
                let artifact = row
                    .test
                    .as_mut()
                    .ok_or("ref.paths names a test but the row has none")?;
                artifact.body = extract_rust_fn(&text, &artifact.fn_name)?;
            }
            "code" => {
                let code = row
                    .code
                    .as_mut()
                    .ok_or("ref.paths names code but the row has none")?;
                code.body = extract_rust_fn(&text, &code.symbol)?;
            }
            // The requirement's text is carried in the row; its file is only
            // digest-checked, so a spec that moved on is noticed.
            _ => {}
        }
    }
    if let Some(mutation) = &row.mutation
        && mutation.target != Target::Requirement
        && !reference.paths.contains_key(mutation.target.role())
    {
        return Err(format!(
            "mutation {} targets {} but ref.paths has no such file to patch",
            mutation.id,
            mutation.target.role()
        ));
    }
    Ok(())
}

/// Every problem with one corpus, as sentences naming the row. Empty = valid.
///
/// Checks: the schema id; unique, non-empty ids; each row's artifacts match
/// its mode; bodies are non-empty (an in-repo row must embed, not reference);
/// every truth key is known, its answer and alternatives lie in the key's
/// answer space, the row carries what the key needs, and the truth kind
/// agrees with the alternatives; a mutation targets an artifact the row has;
/// `strata.fr_id` agrees with `requirement.fr_id`.
pub(crate) fn validate(file: &CorpusFile, origin: Origin) -> Vec<String> {
    let mut problems = Vec::new();
    if file.schema != SCHEMA {
        problems.push(format!("schema is {:?}, expected {SCHEMA:?}", file.schema));
    }
    let mut seen = BTreeSet::new();
    for row in &file.rows {
        let mut say = |problem: String| problems.push(format!("{}: {problem}", row.id));
        if row.id.trim().is_empty() {
            say("empty id".to_owned());
        }
        if !seen.insert(row.id.clone()) {
            say("duplicate id".to_owned());
        }
        if row.mode.has_test() != row.test.is_some() {
            say(format!(
                "mode {} but test is {}",
                row.mode.as_str(),
                if row.test.is_some() {
                    "present"
                } else {
                    "null"
                }
            ));
        }
        if row.mode.has_code() != row.code.is_some() {
            say(format!(
                "mode {} but code is {}",
                row.mode.as_str(),
                if row.code.is_some() {
                    "present"
                } else {
                    "null"
                }
            ));
        }
        if origin == Origin::InRepo && row.reference.is_some() {
            say("an in-repo row embeds its bodies; `ref` is for external rows".to_owned());
        }
        if row
            .test
            .as_ref()
            .is_some_and(|test| test.body.trim().is_empty())
        {
            say("test body is empty".to_owned());
        }
        if row
            .code
            .as_ref()
            .is_some_and(|code| code.body.trim().is_empty())
        {
            say("code body is empty".to_owned());
        }
        if row.requirement.statement.trim().is_empty() {
            say("requirement statement is empty".to_owned());
        }
        if row.strata.fr_id != row.requirement.fr_id {
            say(format!(
                "strata.fr_id {} disagrees with requirement.fr_id {}",
                row.strata.fr_id, row.requirement.fr_id
            ));
        }
        if let Some(mutation) = &row.mutation {
            let present = match mutation.target {
                Target::Requirement => true,
                Target::Test => row.test.is_some(),
                Target::Code => row.code.is_some(),
            };
            if !present {
                say(format!(
                    "mutation {} targets {}, which the row does not carry",
                    mutation.id,
                    mutation.target.role()
                ));
            }
        }
        if row.truth.is_empty() {
            say("no truth labels".to_owned());
        }
        for (key, truth) in &row.truth {
            for problem in truth_problems(row.mode, key, truth) {
                say(problem);
            }
        }
    }
    problems
}

fn truth_problems(mode: Mode, key: &str, truth: &Truth) -> Vec<String> {
    let Some(spec) = spec(key) else {
        return vec![format!("unknown truth key {key:?}")];
    };
    let mut problems = Vec::new();
    if !mode.satisfies(spec.needs) {
        problems.push(format!(
            "{key} needs {:?}, which a mode {} row lacks",
            spec.needs,
            mode.as_str()
        ));
    }
    let answer = truth.answer.label();
    for label in std::iter::once(&truth.answer).chain(&truth.alternatives) {
        let label = label.label();
        if !spec.space.contains(&label.as_str()) {
            problems.push(format!(
                "{key}: {label:?} is outside the answer space {:?}",
                spec.space
            ));
        }
    }
    if truth.alternatives.iter().any(|alt| alt.label() == answer) {
        problems.push(format!("{key}: an alternative repeats the answer"));
    }
    match truth.kind {
        TruthKind::AgentContested if truth.alternatives.is_empty() => problems.push(format!(
            "{key}: agent_contested truth records no alternative reading"
        )),
        TruthKind::Mechanical | TruthKind::ByConstruction if !truth.alternatives.is_empty() => {
            problems.push(format!(
                "{key}: {:?} truth carries alternatives; only agent truth is contested",
                truth.kind
            ));
        }
        _ => {}
    }
    if truth.rationale.trim().is_empty() {
        problems.push(format!("{key}: empty rationale"));
    }
    problems
}

/// Row counts by mode, split, truth kind and crate, for printing beside a
/// validation or a run.
pub(crate) fn census(file: &CorpusFile) -> String {
    let mut counts: BTreeMap<String, usize> = BTreeMap::new();
    for row in &file.rows {
        *counts
            .entry(format!("mode {}", row.mode.as_str()))
            .or_default() += 1;
        *counts
            .entry(format!("split {}", row.split.as_str()))
            .or_default() += 1;
        *counts
            .entry(format!("crate {}", row.strata.crate_name))
            .or_default() += 1;
        for (key, truth) in &row.truth {
            *counts
                .entry(format!("truth {key} / {}", truth.kind.group().label()))
                .or_default() += 1;
        }
    }
    let mut out = format!("{} rows\n", file.rows.len());
    for (name, count) in counts {
        let _ = writeln!(out, "  {name}: {count}");
    }
    out
}

/// The held-out seal of a corpus text: see this module's doc.
///
/// # Errors
/// When the text is not strict JSON or has no `rows` array.
pub(crate) fn heldout_digest(text: &str) -> Result<String, String> {
    let value = parse_strict_json_str(text).map_err(|error| format!("strict parse: {error}"))?;
    let JsonValue::Object(object) = value else {
        return Err("the corpus is not a JSON object".to_owned());
    };
    let Some(JsonValue::Array(rows)) = object.get("rows") else {
        return Err("the corpus has no `rows` array".to_owned());
    };
    let heldout: Vec<JsonValue> = rows
        .iter()
        .filter(|row| {
            matches!(row, JsonValue::Object(fields)
                if matches!(fields.get("split"), Some(JsonValue::String(split)) if split == "heldout"))
        })
        .cloned()
        .collect();
    let bytes = canonical_bytes(&JsonValue::Array(heldout))
        .map_err(|error| format!("canonicalize: {error}"))?;
    Ok(digest_bytes_sha256(&bytes).as_hex().to_owned())
}

/// Checks `source` against the seal beside it; returns the digest.
///
/// # Errors
/// When the seal file is missing or does not match. The mismatch message
/// prints both digests; resealing is a committed, reviewable diff.
pub(crate) fn verify_seal(source: &Source) -> Result<String, String> {
    let seal_path = source
        .path
        .parent()
        .map_or_else(|| PathBuf::from(SEAL_FILE), |dir| dir.join(SEAL_FILE));
    let committed = std::fs::read_to_string(&seal_path).map_err(|error| {
        format!(
            "{}: the held-out split is not sealed ({error})",
            seal_path.display()
        )
    })?;
    let committed = committed.trim();
    let committed = committed.strip_prefix("sha256:").unwrap_or(committed);
    let actual = heldout_digest(&source.text)?;
    if !actual.eq_ignore_ascii_case(committed) {
        return Err(format!(
            "{}: held-out rows digest to {actual}, the committed seal is {committed}; \
             the held-out split changed after it was sealed",
            source.path.display()
        ));
    }
    Ok(actual)
}

/// Gate for a held-out run: `flag` (the value of [`HELDOUT_ENV`]) must be
/// `1`, and every source's seal must hold. Returns each source's digest.
///
/// # Errors
/// When the flag is not `1` or any seal fails.
pub(crate) fn authorize_heldout(
    flag: Option<&str>,
    sources: &[&Source],
) -> Result<Vec<String>, String> {
    if flag != Some("1") {
        return Err(format!(
            "the held-out split runs only with {HELDOUT_ENV}=1; winners are chosen on dev \
             and reported once on held-out (PLAT-1024)"
        ));
    }
    sources.iter().map(|source| verify_seal(source)).collect()
}

/// One held-out run, as appended to [`heldout_log_path`].
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct HeldoutRun {
    /// Seconds since the Unix epoch when the run finished.
    pub(crate) unix_seconds: u64,
    /// Every variant label run, in run order.
    pub(crate) variants: Vec<String>,
    /// Each sealed source's held-out digest.
    pub(crate) seals: Vec<String>,
    /// Held-out rows run.
    pub(crate) rows: usize,
    /// Answering model, by count of requests sent.
    pub(crate) models: BTreeMap<String, usize>,
}

/// Appends `run` as one JSON line to `log`, creating it if absent. The log is
/// committed with the report, so a reader can see how many times, and with
/// which variants, the held-out split was spent.
///
/// # Errors
/// When the line cannot be written.
pub(crate) fn record_heldout_run(log: &Path, run: &HeldoutRun) -> Result<(), String> {
    use std::io::Write as _;
    let line = serde_json::to_string(run).map_err(|error| error.to_string())?;
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(log)
        .map_err(|error| format!("{}: {error}", log.display()))?;
    writeln!(file, "{line}").map_err(|error| format!("{}: {error}", log.display()))
}
