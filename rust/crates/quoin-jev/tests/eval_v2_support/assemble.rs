// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Corpus v2's assembly (PLAT-1025): the rules that turn the committed
//! inputs into `corpus.json`.
//!
//! The inputs are `sample.json` (the seeded draw), `mutations.json` (the
//! mutation log, outcomes recorded by `run-mutants.sh`), the two independent
//! label passes, and the repository's own files at [`CONTENT_COMMIT`], read
//! through git. Nothing here reads the working tree, so the assembly is an
//! offline, deterministic function of committed bytes: the builder
//! (`eval_v2_build.rs`) writes its output, and `eval_v2_corpus.rs` rebuilds it
//! in the default gate and compares.
//!
//! [`mutation_truth`] and [`settlement`] are the whole rule set that turns a
//! mutation log entry into ground truth.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "a fixture builder over committed inputs: a malformed input IS the failure report"
)]
#![allow(
    clippy::too_many_lines,
    reason = "the assembly is one linear pass over the rows; splitting it scatters the rules"
)]

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;

use regex::Regex;
use serde_json::{Value, json};

use super::corpus::{MAX_NATURAL_ROWS_PER_FR, SCHEMA};
use super::criterion_defects::CRITERION_DEFECTS;

/// The sampling seed (restated in [`SAMPLING_RULE`]).
pub(crate) const SEED: u64 = 20_260_923;

/// The commit whose tree every body and requirement text is read from. It is
/// on `main` and equals, for every sampled file, the tree the mutation runner
/// ran against. The population was enumerated at `sample.json`'s
/// `source_commit` (2c81f1c8); between the two, the only sampled content that
/// changed is FR-107-AC-8's text and a test appended to
/// `tc_958_stage_verdicts.rs` below the sampled one.
pub(crate) const CONTENT_COMMIT: &str = "ef152795c0758835682e365f5f8e683d032ffd49";

/// The written sampling rule, copied verbatim into the corpus header.
pub(crate) const SAMPLING_RULE: &str = "\
POPULATION. The deterministic trace scan gap-analysis runs (skills/gap-analysis step 3 and \
step 4), over quoin at source_commit. (a) Test-to-requirement: `quire symbols --module \
spec-artifacts-process --json`; every test symbol whose extracted trace_ids name a requirement \
(FR-/NFR-/StR-/US-N) contributes one (test, requirement) pair per distinct requirement, with the \
first criterion id it cites for that requirement (-AC-/-CON-/-VC-/-M-N) as the AC, else none. \
Requirement ids defined by more than one spec document (FR-046, FR-047, FR-048 at \
source_commit) are ambiguous and excluded. (b) Test-to-code: the covered symbol is the non-test \
Rust function, defined in a file that `quire trace --id <req> --prefix` cites for the \
requirement (the code-to-requirement trace), whose name is called in the test body; ties go to \
call count, then name length, then earliest call. Trait boilerplate (fmt, from, new, as_str, \
...) and bodies under 3 lines are not candidates. A pair with no candidate has no code. (c) \
Requirement-to-code with no test: for a requirement no test cites, each `quire trace` citation \
that sits inside a non-test Rust function. (d) Criteria: every criterion row (AC/CON/VC/M) of \
every requirement document. \
CANDIDATES per mode: RTC = pairs with code; RT = every pair (code dropped, or genuinely \
absent); RC = pairs with code (test dropped) plus (c); R = (d). \
STRATA: mode x req_kind (fr_behaviour | constraint | nfr | stakeholder, from the criterion id) \
x test_kind (unit = #[cfg(test)] in src; integration = tests/ dir; cli = the test file spawns \
a built binary via CARGO_BIN_EXE; property = the file uses proptest) x crate; for R, mode x \
req_kind x ears_pattern (event_driven When / state_driven While / unwanted If..then / optional \
Where / ubiquitous otherwise, read from the criterion text by keyword). \
DRAW. One SplitMix64 stream, seed 20260923. For each mode in the order R, RT, RC, RTC: \
candidates grouped by stratum in lexical key order, each group Fisher-Yates shuffled, then \
the stratum order shuffled. Rounds then visit the modes in the order RTC, RC, RT, R; a mode \
under quota takes the next stratum in its rotation and pops candidates until one passes: its \
requirement has fewer than 3 natural rows in any mode (the per-FR cap), its test and its code \
symbol are not already in any natural row, and its criterion is not already an R row. A \
failing candidate is discarded (every constraint is monotone). Quotas: RTC 64, RC 28, RT 48, \
R 58; a mode whose strata run dry stops short of its quota (RTC does, at 46). \
FIRST MUTATION TARGETS, from the same stream after the draw: natural RTC rows shuffled, first \
30 are code-mutant sources (every source a violating code mutant; even positions an additive \
code mutant, odd positions a test-weakening mutant; the first half of each list also shown in \
the pair mode, RC for code mutants and RT for test-weakening); natural RT rows shuffled, first \
16 get a requirement-text mutant; natural R rows shuffled, first 30 get a criterion mutant, the \
defect cycling vague_term, no_measurable_threshold, compound, missing_trigger, untestable; 6 \
RTC, 5 RT and 5 RC natural rows (shuffled, not already a source) get a trace swap to a \
requirement drawn from those whose tests live in a different crate. \
SPLIT. A family is a natural row plus every mutation row derived from it (its source group); \
per natural mode, families are shuffled and the first round(0.35 n) are dev, the rest heldout. \
A mutant always sits in its source's split and names it in mutation.source_id. \
AFTER THE DRAW (recorded in mutations.json, not re-derived from the seed). (1) A code-mutant \
source whose shown code its mutation author found does not implement the shown requirement was \
replaced by the next row of code_source_order (slots VIOL-31.., ADD-16.., WEAK-16..). (2) \
Supplementary dev-side mutants (slots S*) were added so every mutant kind had ten dev pairs. \
(3) TOP-UP, applied to every natural row of BOTH splits, so dev and held-out get mutants in \
the same proportion (slots T*): every RTC or RC row whose code_implements_intent label (pass \
A) is yes has two violating code mutants; every RTC or RC row whose code_exceeds_requirement \
label is no has two additive code mutants; every RTC row whose trace_correct and \
test_asserts_intent labels are yes has two test-weakening mutants, each paired with its own \
violating mutant; every RT row with those two labels yes has one test-weakening mutant, paired \
with a pair-only violating mutant of the code its test exercises (no corpus row of its own); \
every RT, RC or RTC row whose trace_correct label is yes has one trace swap, its partner drawn \
with Python random.Random(20260925) from the requirements whose tests live in another crate \
and that carry fewer than 8 rows, so the swaps land on thinly covered requirements. A top-up \
mutant is shown in its source's mode only. A slot with no sensible mutant is recorded as \
skipped. The code top-up was planned as 13 packets of sources; 9 were authored and run \
(sources in packets 1, 7, 8 and 10 got no top-up mutants), so the top-up covers most, not \
every, eligible row. New criterion mutants were authored but their two blind audit passes \
were not run, so none is in the corpus: the criterion mutants are the first draw's and the \
supplementary S* ones, each confirmed by both label passes to meet its defect's definition. \
(4) Redone: WEAK-14/16, whose weakened tests still caught their pairs, are replaced by \
RWEAK-001..002 where authored, and the size-cap additive mutants SADD-01/03/05 (size \
ceilings are excluded by the code_exceeds_requirement rule) are dropped. Every \
missing_trigger mutant leaves a dangling reference; 7 of them share the prefix \"In that \
case,\", a known surface cue. A superseded mutant stays in mutations.json as dropped, with the reason. \
A mutant that fails to compile, an additive mutant its owning test catches, a weakened test \
that fails against the shipped code, and a criterion mutant either label pass finds does not \
meet its defect's definition are dropped, with the reason. \
SAME-MODE SOURCES. A mutant shown in a mode other than its natural source's (the first \
draw's RC copies of code mutants and RT copies of test-weakening mutants) is paired, through \
mutation.source_id, with that source's projection into the mode: the same unit, split and \
source group, carrying only the labels whose question reads what the mode still shows \
(divergence_kind and severity are not carried). A projection is a natural row and counts \
toward the per-FR cap of 3; where the cap is full, the mutant is not shown in that mode. \
RESEAL. The held-out split was re-drawn into and resealed on 2026-09-24, before any variant \
had made a live call on either split (no request had been sent, so no held-out row had been \
seen by a variant): the PR #628 review changed the truth rules and the per-row mutant rule, \
and the soundness labels were redone under the shared criterion_defects definitions.";

/// The criterion-soundness checklist keys, `criterion_sound` first.
pub(crate) const CRITERION_KEYS: [&str; 6] = [
    "criterion_sound",
    "vague_term",
    "no_measurable_threshold",
    "untestable",
    "compound",
    "missing_trigger",
];

/// The question keys this corpus labels on a row of `mode`. Narrower than
/// what the harness admits (`keys.rs` allows `severity` and the checklist on
/// any row): the checklist is labelled on `R` rows, the gap-analysis battery
/// on rows that carry a test or code.
pub(crate) fn keys_for_mode(mode: &str) -> &'static [&'static str] {
    match mode {
        "R" => &CRITERION_KEYS,
        "RT" => &[
            "trace_correct",
            "assertion_vacuous",
            "test_asserts_intent",
            "tests_only_its_own_mock",
            "divergence_kind",
            "severity",
        ],
        "RC" => &[
            "trace_correct",
            "code_implements_intent",
            "code_exceeds_requirement",
            "divergence_kind",
            "severity",
        ],
        "RTC" => &[
            "trace_correct",
            "assertion_vacuous",
            "test_asserts_intent",
            "tests_only_its_own_mock",
            "code_implements_intent",
            "code_exceeds_requirement",
            "divergence_kind",
            "severity",
        ],
        _ => &[],
    }
}

// ---------------------------------------------------------------------------
// Content: where file bytes come from
// ---------------------------------------------------------------------------

/// The repository files the corpus quotes.
pub(crate) trait Content {
    /// The text of a repo-relative file.
    fn read(&self, path: &str) -> String;
    /// The repo-relative paths of the `.md` files directly in `dir`, sorted.
    fn list_md(&self, dir: &str) -> Vec<String>;
}

/// Files as committed at one commit, through `git show`: offline and fixed.
pub(crate) struct GitContent {
    root: PathBuf,
    commit: String,
    cache: RefCell<BTreeMap<String, String>>,
}

impl GitContent {
    /// The tree of `commit` in the repository at `root`.
    pub(crate) fn new(root: &Path, commit: &str) -> Self {
        Self {
            root: root.to_owned(),
            commit: commit.to_owned(),
            cache: RefCell::new(BTreeMap::new()),
        }
    }

    fn git(&self, args: &[&str]) -> String {
        let out = Command::new("git")
            .arg("-C")
            .arg(&self.root)
            .args(args)
            .output()
            .expect("spawn git");
        assert!(
            out.status.success(),
            "git {args:?} failed: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        String::from_utf8(out.stdout).expect("utf-8 from git")
    }
}

impl Content for GitContent {
    fn read(&self, path: &str) -> String {
        if let Some(text) = self.cache.borrow().get(path) {
            return text.clone();
        }
        let text = self.git(&["show", &format!("{}:{path}", self.commit)]);
        self.cache
            .borrow_mut()
            .insert(path.to_owned(), text.clone());
        text
    }

    fn list_md(&self, dir: &str) -> Vec<String> {
        let mut paths: Vec<String> = self
            .git(&["ls-tree", "--name-only", &self.commit, &format!("{dir}/")])
            .lines()
            .filter(|p| Path::new(p).extension().is_some_and(|x| x == "md"))
            .map(str::to_owned)
            .collect();
        paths.sort();
        paths
    }
}

/// Files as they are in a working tree (the enumerate and sample stages).
pub(crate) struct TreeContent(pub(crate) PathBuf);

impl Content for TreeContent {
    fn read(&self, path: &str) -> String {
        let full = self.0.join(path);
        std::fs::read_to_string(&full).unwrap_or_else(|e| panic!("{}: {e}", full.display()))
    }

    fn list_md(&self, dir: &str) -> Vec<String> {
        let mut paths: Vec<String> = std::fs::read_dir(self.0.join(dir))
            .unwrap()
            .map(|e| e.unwrap().path())
            .filter(|p| p.extension().is_some_and(|x| x == "md"))
            .map(|p| {
                format!(
                    "{dir}/{}",
                    p.file_name().unwrap().to_string_lossy().into_owned()
                )
            })
            .collect();
        paths.sort();
        paths
    }
}

/// This worktree's root.
pub(crate) fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .canonicalize()
        .expect("repo root")
}

/// `tests/fixtures/eval-v2/`.
pub(crate) fn fixture_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/eval-v2")
}

// ---------------------------------------------------------------------------
// Spec documents
// ---------------------------------------------------------------------------

/// One requirement document, as far as the corpus needs it.
#[derive(Debug, Clone)]
pub(crate) struct ReqDoc {
    /// e.g. `FR-068`.
    pub(crate) id: String,
    /// Repo-relative path.
    pub(crate) path: String,
    /// Frontmatter title.
    pub(crate) title: String,
    /// The normative statement.
    pub(crate) statement: String,
    /// criterion id -> criterion text (the second table cell).
    pub(crate) criteria: BTreeMap<String, String>,
    /// criterion ids in document order.
    pub(crate) order: Vec<String>,
}

fn frontmatter_field(text: &str, key: &str) -> Option<String> {
    let body = text.strip_prefix("---\n")?;
    let end = body.find("\n---")?;
    body[..end].lines().find_map(|line| {
        line.strip_prefix(&format!("{key}:"))
            .map(|v| v.trim().trim_matches('"').to_owned())
    })
}

fn section(text: &str, headings: &[&str]) -> String {
    for heading in headings {
        let marker = format!("\n## {heading}\n");
        if let Some(start) = text.find(&marker) {
            let rest = &text[start + marker.len()..];
            let end = rest.find("\n## ").unwrap_or(rest.len());
            return rest[..end].trim().to_owned();
        }
    }
    String::new()
}

/// The normative statement: the Description / Statement / Stakeholder Need
/// section, or, for a document with none of them (a withdrawn requirement
/// carries only a notice and a Disposition), everything between the title
/// and the Dependencies section.
fn statement_of(text: &str) -> String {
    let found = section(text, &["Description", "Statement", "Stakeholder Need"]);
    if !found.is_empty() {
        return found;
    }
    let Some(title) = text.find("\n# ") else {
        return String::new();
    };
    let rest = &text[title + 1..];
    let rest = &rest[rest.find('\n').map_or(rest.len(), |i| i + 1)..];
    let end = rest.find("\n## Dependencies").unwrap_or(rest.len());
    rest[..end].trim().to_owned()
}

/// Every requirement document with a unique id, and the ids defined twice.
pub(crate) fn load_docs(content: &dyn Content) -> (BTreeMap<String, ReqDoc>, Vec<String>) {
    let crit =
        Regex::new(r"^\|\s*((?:FR|NFR|StR|US)-\d+-(?:AC|CON|VC|M)-\d+)\s*\|([^|]*)\|").unwrap();
    let mut by_id: BTreeMap<String, Vec<ReqDoc>> = BTreeMap::new();
    for dir in ["spec/functional", "spec/non-functional", "spec/stakeholder"] {
        for path in content.list_md(dir) {
            let text = content.read(&path);
            let Some(id) = frontmatter_field(&text, "id") else {
                continue;
            };
            let mut criteria = BTreeMap::new();
            let mut order = Vec::new();
            for line in text.lines() {
                if let Some(c) = crit.captures(line) {
                    let cid = c[1].to_owned();
                    if cid.starts_with(&format!("{id}-")) && !criteria.contains_key(&cid) {
                        criteria.insert(cid.clone(), c[2].trim().to_owned());
                        order.push(cid);
                    }
                }
            }
            by_id.entry(id.clone()).or_default().push(ReqDoc {
                title: frontmatter_field(&text, "title").unwrap_or_default(),
                statement: statement_of(&text),
                id,
                path,
                criteria,
                order,
            });
        }
    }
    let ambiguous: Vec<String> = by_id
        .iter()
        .filter(|(_, docs)| docs.len() > 1)
        .map(|(id, _)| id.clone())
        .collect();
    let docs = by_id
        .into_iter()
        .filter(|(_, docs)| docs.len() == 1)
        .map(|(id, mut docs)| (id, docs.remove(0)))
        .collect();
    (docs, ambiguous)
}

/// The requirement's kind, from its id and criterion id.
pub(crate) fn req_kind(req_id: &str, crit_id: Option<&str>) -> &'static str {
    if req_id.starts_with("NFR-") {
        "nfr"
    } else if req_id.starts_with("StR-") || req_id.starts_with("US-") {
        "stakeholder"
    } else if crit_id.is_some_and(|c| c.contains("-CON-")) {
        "constraint"
    } else {
        "fr_behaviour"
    }
}

/// The EARS pattern of a criterion, by keyword.
pub(crate) fn ears_pattern(text: &str) -> &'static str {
    let t = text.trim_start().to_lowercase();
    if t.starts_with("when ") || t.contains(" when ") {
        "event_driven"
    } else if t.starts_with("while ") || t.contains(" while ") {
        "state_driven"
    } else if (t.starts_with("if ") || t.contains(" if ")) && t.contains(" then ") {
        "unwanted"
    } else if t.starts_with("where ") {
        "optional"
    } else {
        "ubiquitous"
    }
}

/// Lines `from..=to` (1-based) of `text`.
pub(crate) fn span(text: &str, from: usize, to: usize) -> String {
    let lines: Vec<&str> = text.lines().collect();
    let from = from.max(1) - 1;
    let to = to.min(lines.len());
    lines[from..to].join("\n")
}

fn s(value: &Value, key: &str) -> String {
    value[key]
        .as_str()
        .unwrap_or_else(|| panic!("{key} missing in {value}"))
        .to_owned()
}

fn u(value: &Value, key: &str) -> usize {
    usize::try_from(
        value[key]
            .as_u64()
            .unwrap_or_else(|| panic!("{key} in {value}")),
    )
    .unwrap()
}

// ---------------------------------------------------------------------------
// Labelling rules
// ---------------------------------------------------------------------------

/// Per-question labelling rules, copied into the corpus header. The five
/// soundness checks come from [`CRITERION_DEFECTS`].
pub(crate) fn labelling_rules() -> Value {
    let mut rules = json!({
        "_provenance": "Natural rows are AGENT-LABELLED, not human ground truth: two independent \
            Claude subagent passes (A and B) each saw only the row and these rules, never each \
            other's output. Agreement is kind agent_dual; disagreement is agent_contested with \
            both answers in alternatives (answer = pass A, alternatives = [pass B]); nothing was \
            resolved. On R rows every checklist key was relabelled by two further independent \
            passes under the criterion_defects definitions below (PR #624 and #628 reviews); \
            those answers replace the earlier ones. Mutation rows carry mechanical truth (the \
            owning test was rerun) or by_construction truth (true because of the edit made), per \
            mutation.kind and the rules in eval_v2_support/assemble.rs. A key a mutation does not \
            determine is inherited from the source row's AGENT labels only where the edit leaves \
            everything that key reads unchanged, and its rationale says INHERITED; a key the edit \
            may change is left unlabelled on the mutant. A natural row's test_asserts_intent is \
            mechanical only where a violating mutant of its code settles it: both passes say \
            trace_correct is yes, the mutant breaks the behaviour the shown requirement states, \
            and the owning test either failed on its own assertion or still passed.",
        "trace_correct": "yes when the shown requirement (statement + criterion) is about the \
            behaviour the shown test and/or code exercise: a reader would expect this test or \
            code to be cited for it. no when the requirement describes a different subsystem, \
            a process step, or behaviour the test/code never touches.",
        "assertion_vacuous": "yes when the test would still pass if the implementation under \
            test were replaced by a stub returning a default (empty, zero, None, Ok(default)): \
            it asserts nothing, or only asserts things a default satisfies. no otherwise.",
        "test_asserts_intent": "yes when the test would fail if the code stopped doing the \
            specific thing the shown requirement/criterion states (while still compiling and \
            returning a well-formed result); judge against the requirement's own stated \
            behaviour, not other things the code does. no when it checks only an incidental \
            aspect, a weaker property, or something unrelated to the criterion.",
        "tests_only_its_own_mock": "yes when every assertion compares only against values the \
            test itself supplied (as input, or to a fake/mock/fixture it built), so no \
            assertion states an independent expected value; also yes when the test makes no \
            assertions. no when at least one assertion checks an independent expected value.",
        "code_implements_intent": "yes when the shown code does what the shown requirement \
            states, for the part a single symbol can be expected to carry. no when it \
            contradicts it, omits a stated behaviour it is the natural owner of, or is about \
            something else entirely (trace mismatch).",
        "code_exceeds_requirement": "yes when the code makes a substantive decision (a branch, \
            a refusal, an extra output, a side effect) that the shown requirement does not \
            state or imply. Boundary hygiene every operation carries (size ceilings, request \
            parsing, error-context enrichment) does not count. Judged against the requirement \
            SHOWN only.",
        "divergence_kind": "the dominant divergence among aligned, \
            test_weaker_than_requirement, test_stronger_than_requirement, \
            code_short_of_requirement, code_exceeds_requirement, requirement_ambiguous (the \
            FullBattery option set). In RT rows only test_* / aligned / requirement_ambiguous \
            are available evidence; in RC rows only code_* / aligned / requirement_ambiguous.",
        "severity": "the rubric in skills/gap-analysis/references/step-5-semantic-review.md: \
            high = the test does not validate the requirement's intent (a test tagged to a \
            criterion that asserts something unrelated fails here) or does not exercise code, \
            or the code contradicts the requirement; medium = partial validation, meaningful \
            edge cases unchecked, minor drift; low = trivial drift; none = no finding.",
        "criterion_sound": "yes when the criterion, read alone, is a single, testable, \
            unambiguous statement with a clear trigger/condition where one is needed and a \
            measurable pass/fail. no when any of the five checks below says yes.",
    });
    for check in &CRITERION_DEFECTS {
        rules[check.key] = Value::String(check.rule());
    }
    rules
}

// ---------------------------------------------------------------------------
// Truth
// ---------------------------------------------------------------------------

fn truth(answer: &str, kind: &str, alternatives: &[String], rationale: &str) -> Value {
    json!({"answer": answer, "kind": kind, "alternatives": alternatives, "rationale": rationale})
}

/// The label for `key` on natural row `id`, merged from the two passes.
fn natural_truth(a: &Value, b: &Value, id: &str, key: &str) -> Option<Value> {
    let la = a[id][key]["answer"].as_str()?;
    let lb = b[id][key]["answer"].as_str()?;
    let ra = a[id][key]["why"].as_str().unwrap_or("");
    let rb = b[id][key]["why"].as_str().unwrap_or("");
    Some(if la == lb {
        truth(
            la,
            "agent_dual",
            &[],
            &format!("AGENT-LABELLED, two independent passes agree. A: {ra} | B: {rb}"),
        )
    } else {
        truth(
            la,
            "agent_contested",
            &[lb.to_owned()],
            &format!(
                "AGENT-LABELLED, passes disagree (A={la}, B={lb}); not resolved. A: {ra} | B: {rb}"
            ),
        )
    })
}

/// Replace the single occurrence of `find` in `text`, or `None` when it does
/// not occur exactly once.
fn replace_once(text: &str, find: &str, replace: &str) -> Option<String> {
    (text.matches(find).count() == 1).then(|| text.replacen(find, replace, 1))
}

/// A truth a mutation determines.
pub(crate) struct Determined {
    /// The key.
    pub(crate) key: &'static str,
    /// The answer.
    pub(crate) answer: String,
    /// `mechanical` or `by_construction`.
    pub(crate) kind: &'static str,
    /// Why, naming the mutation.
    pub(crate) rationale: String,
}

fn det(key: &'static str, answer: &str, kind: &'static str, rationale: String) -> Determined {
    Determined {
        key,
        answer: answer.to_owned(),
        kind,
        rationale,
    }
}

/// Whether a violating mutant's run is evidence about what the test checks:
/// it breaks the stated behaviour, and the owning test either failed on its
/// own assertion or still passed.
fn qualifies(m: &Value) -> bool {
    m["breaks_stated_behaviour"] == true
        && (m["outcome"] == "survived" || (m["outcome"] == "caught" && m["failure"] == "assertion"))
}

fn outcome_phrase(m: &Value) -> String {
    let failure = m["failure"]
        .as_str()
        .map_or_else(String::new, |f| format!(" ({f})"));
    format!(
        "{} {}{failure}",
        s(m, "id"),
        m["outcome"].as_str().unwrap_or("?")
    )
}

/// How the violating mutants of a natural row settle its
/// `test_asserts_intent`: `Some((answer, rationale))` when they do, else
/// `None` with the evidence sentence to append to the agent label.
///
/// They settle it only when both label passes say `trace_correct` is yes and
/// at least one mutant [`qualifies`]. Then any qualifying survivor makes it
/// `no` (the test passes while the stated behaviour is broken); otherwise it
/// is `yes` (every qualifying mutant failed the test on its own assertion).
pub(crate) fn settlement(
    trace_dual_yes: bool,
    violators: &[&Value],
) -> Result<(&'static str, String), Option<String>> {
    if violators.is_empty() {
        return Err(None);
    }
    let list = violators
        .iter()
        .map(|m| outcome_phrase(m))
        .collect::<Vec<_>>()
        .join(", ");
    let qualifying: Vec<&&Value> = violators.iter().filter(|m| qualifies(m)).collect();
    if !trace_dual_yes || qualifying.is_empty() {
        let why = if trace_dual_yes {
            "no violating mutant both breaks the stated behaviour and fails the test on its own \
             assertion or passes it"
        } else {
            "the two label passes do not both say trace_correct is yes"
        };
        return Err(Some(format!(
            " Mutation evidence, NOT used as truth because {why}: {list}."
        )));
    }
    let survivors: Vec<String> = qualifying
        .iter()
        .filter(|m| m["outcome"] == "survived")
        .map(|m| s(m, "id"))
        .collect();
    Ok(if survivors.is_empty() {
        (
            "yes",
            format!(
                "Every violating mutant that breaks the stated behaviour made the owning test fail \
                 on its own assertion ({list}); both label passes say the trace is correct."
            ),
        )
    } else {
        (
            "no",
            format!(
                "Violating mutant(s) {} break the stated behaviour and the owning test still \
                 PASSES ({list}); both label passes say the trace is correct.",
                survivors.join(", ")
            ),
        )
    })
}

/// The truths `m` determines, and the keys it leaves to the source row's
/// agent labels (inherited unchanged, because the edit leaves everything the
/// key reads as it was). Every other key is unlabelled on the mutant.
///
/// `settled` is the source's own settlement of `test_asserts_intent`, when
/// its violating mutants settle it.
pub(crate) fn mutation_truth(
    m: &Value,
    source_agent: &BTreeMap<String, Value>,
    settled: Option<&(&'static str, String)>,
    by_id: &BTreeMap<String, &Value>,
) -> (Vec<Determined>, Vec<&'static str>) {
    let id = s(m, "id");
    let what = s(m, "description");
    let test_name = m["owning_test"]["filter"]
        .as_str()
        .unwrap_or("the owning test")
        .to_owned();
    match s(m, "kind").as_str() {
        "violating_code" => {
            assert_eq!(
                m["breaks_stated_behaviour"], true,
                "{id}: a violating mutant in the corpus must break the stated behaviour"
            );
            let confirmed = m["outcome"] == "caught" && m["failure"] == "assertion";
            let mut d = vec![
                det(
                    "code_implements_intent",
                    "no",
                    if confirmed {
                        "mechanical"
                    } else {
                        "by_construction"
                    },
                    if confirmed {
                        format!(
                            "{id} contradicts the stated behaviour ({what}); {test_name} FAILED \
                             on its own assertion against it."
                        )
                    } else {
                        format!("{id} contradicts the stated behaviour ({what}) by construction.")
                    },
                ),
                det(
                    "divergence_kind",
                    "code_short_of_requirement",
                    "by_construction",
                    format!("{id} makes the code contradict the stated behaviour."),
                ),
                det(
                    "severity",
                    "high",
                    "by_construction",
                    "step-5 rubric: the code contradicts the requirement -> high.".to_owned(),
                ),
            ];
            let mut inherit = vec![
                "trace_correct",
                "tests_only_its_own_mock",
                "assertion_vacuous",
            ];
            match settled {
                Some((answer, why)) => d.push(det(
                    "test_asserts_intent",
                    answer,
                    "mechanical",
                    format!("Settled on the source row by its violating mutants: {why}"),
                )),
                None => inherit.push("test_asserts_intent"),
            }
            (d, inherit)
        }
        "additive_code" => {
            let mut d = vec![det(
                "code_exceeds_requirement",
                "yes",
                "by_construction",
                format!(
                    "{id} adds behaviour the requirement does not state ({what}); {test_name} \
                     still PASSED."
                ),
            )];
            if source_agent
                .get("divergence_kind")
                .is_some_and(|v| v["answer"] == "aligned" && v["kind"] == "agent_dual")
            {
                d.push(det(
                    "divergence_kind",
                    "code_exceeds_requirement",
                    "by_construction",
                    format!(
                        "The source row is labelled aligned by both passes; {id} adds unrequired \
                         behaviour."
                    ),
                ));
            }
            (
                d,
                vec![
                    "trace_correct",
                    "test_asserts_intent",
                    "tests_only_its_own_mock",
                    "assertion_vacuous",
                ],
            )
        }
        "test_weakening" => {
            let pair = by_id.get(&s(m, "pair")).copied();
            let mechanical = m["pair_outcome_original_test"] == "caught"
                && m["pair_outcome_weakened_test"] == "survived"
                && pair.is_some_and(|p| p["failure"] == "assertion");
            let removed = m["weakening"] == "removed";
            let mut d = vec![
                det(
                    "divergence_kind",
                    "test_weaker_than_requirement",
                    "by_construction",
                    format!("{id} removes or loosens the assertion of the stated behaviour."),
                ),
                det(
                    "severity",
                    if removed { "high" } else { "medium" },
                    "by_construction",
                    if removed {
                        "step-5 rubric: the test no longer validates the requirement's intent -> \
                         high."
                            .to_owned()
                    } else {
                        "step-5 rubric: partial validation, the stated behaviour is only loosely \
                         checked -> medium."
                            .to_owned()
                    },
                ),
            ];
            if mechanical {
                d.push(det(
                    "test_asserts_intent",
                    "no",
                    "mechanical",
                    format!(
                        "{id} weakens the test ({what}). Violating mutant {} failed the original \
                         test on its own assertion and PASSES the weakened one.",
                        s(m, "pair")
                    ),
                ));
                if m["all_assertions_removed"] == true {
                    d.push(det(
                        "assertion_vacuous",
                        "yes",
                        "by_construction",
                        format!("{id} leaves the test with no assertion on the code's result."),
                    ));
                }
            }
            (
                d,
                vec![
                    "trace_correct",
                    "code_implements_intent",
                    "code_exceeds_requirement",
                ],
            )
        }
        "requirement_text" => {
            let mut d = vec![det(
                "trace_correct",
                "yes",
                "by_construction",
                format!(
                    "{id} changes a value or condition, not the subject: the test is still about \
                     this requirement."
                ),
            )];
            let mut inherit = vec!["tests_only_its_own_mock", "assertion_vacuous"];
            match m["requirement_effect"].as_str() {
                Some("demands_unchecked") => {
                    d.push(det(
                        "test_asserts_intent",
                        "no",
                        "by_construction",
                        format!(
                            "{id} makes the requirement demand something the unchanged test \
                             does not check ({what})."
                        ),
                    ));
                    d.push(det(
                        "severity",
                        "high",
                        "by_construction",
                        "step-5 rubric: the test does not validate the requirement's (changed) \
                         intent -> high."
                            .to_owned(),
                    ));
                    d.push(det(
                        "divergence_kind",
                        "test_weaker_than_requirement",
                        "by_construction",
                        format!("{id}: {what}."),
                    ));
                }
                Some("narrowed") => {
                    // The unchanged test checks a superset of the narrowed
                    // requirement: whatever it asserted of the original it
                    // asserts of the narrower one, so its answer is the
                    // source's, and nothing makes the finding high.
                    if source_agent
                        .get("test_asserts_intent")
                        .is_some_and(|v| v["answer"] == "yes")
                    {
                        inherit.push("test_asserts_intent");
                    }
                    d.push(det(
                        "divergence_kind",
                        "test_stronger_than_requirement",
                        "by_construction",
                        format!("{id}: {what}."),
                    ));
                }
                other => panic!("{id}: requirement_effect {other:?}"),
            }
            (d, inherit)
        }
        "criterion" => {
            let defect = CRITERION_DEFECTS
                .iter()
                .find(|c| m["defect"] == c.key)
                .unwrap_or_else(|| panic!("{id}: unknown defect {}", m["defect"]))
                .key;
            for pass in ["pass_a", "pass_b"] {
                assert!(
                    m["meets_definition"][pass]["answer"] == "yes",
                    "{id}: label {pass} does not find the {defect} definition met"
                );
            }
            (
                vec![
                    det(
                        defect,
                        "yes",
                        "by_construction",
                        format!(
                            "{id} injects it: {what}. Both independent label passes confirm it \
                             meets the {defect} definition."
                        ),
                    ),
                    det(
                        "criterion_sound",
                        "no",
                        "by_construction",
                        format!("{id} injects a {defect} defect."),
                    ),
                ],
                Vec::new(),
            )
        }
        "trace_swap" => (
            vec![
                det(
                    "trace_correct",
                    "no",
                    "by_construction",
                    format!(
                        "{id} pairs this test/code with {}, a requirement whose tests live in \
                         another crate.",
                        s(m, "partner_req_id")
                    ),
                ),
                det(
                    "test_asserts_intent",
                    "no",
                    "by_construction",
                    "The test was written for a different requirement.".to_owned(),
                ),
                det(
                    "code_implements_intent",
                    "no",
                    "by_construction",
                    "The code implements a different requirement (MP-234's trace-aware reading)."
                        .to_owned(),
                ),
                det(
                    "severity",
                    "high",
                    "by_construction",
                    "step-5 rubric: a test tagged to a criterion that asserts something unrelated \
                     -> high."
                        .to_owned(),
                ),
            ],
            vec!["tests_only_its_own_mock", "assertion_vacuous"],
        ),
        other => panic!("unknown mutation kind {other}"),
    }
}

// ---------------------------------------------------------------------------
// Assembly
// ---------------------------------------------------------------------------

/// The committed inputs.
pub(crate) struct Inputs {
    /// `sample.json`.
    pub(crate) sample: Value,
    /// `mutations.json`.
    pub(crate) mutations: Value,
    /// `labels-pass-a.json`'s `labels`.
    pub(crate) labels_a: Value,
    /// `labels-pass-b.json`'s `labels`.
    pub(crate) labels_b: Value,
}

fn load_json(path: &Path) -> Value {
    let text = std::fs::read_to_string(path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    serde_json::from_str(&text).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

impl Inputs {
    /// The inputs committed in `dir`.
    pub(crate) fn load(dir: &Path) -> Self {
        Self {
            sample: load_json(&dir.join("sample.json")),
            mutations: load_json(&dir.join("mutations.json")),
            labels_a: load_json(&dir.join("labels-pass-a.json"))["labels"].clone(),
            labels_b: load_json(&dir.join("labels-pass-b.json"))["labels"].clone(),
        }
    }
}

fn requirement_block(docs: &BTreeMap<String, ReqDoc>, req: &str, crit: Option<&str>) -> Value {
    let doc = docs
        .get(req)
        .unwrap_or_else(|| panic!("no document for {req}"));
    json!({
        "fr_id": req,
        "ac_id": crit,
        "statement": doc.statement,
        "ac_text": crit.and_then(|c| doc.criteria.get(c)).cloned(),
        "context": format!("{}: {} ({})", doc.id, doc.title, doc.path),
    })
}

fn body_block(content: &dyn Content, r: &Value, name_key: &str) -> Value {
    let path = s(r, "path");
    let text = content.read(&path);
    let symbol = s(r, "symbol");
    let mut out = json!({
        "path": path,
        "body": span(&text, u(r, "leading_line"), u(r, "end_line")),
    });
    out[name_key] = Value::String(if name_key == "fn_name" {
        symbol.rsplit("::").next().unwrap().to_owned()
    } else {
        symbol
    });
    out
}

/// The corpus rows before final ids; mutation rows name their source (a
/// natural row or its projection into the mutant's mode) in
/// `mutation.source_id`.
pub(crate) fn draft_rows(inputs: &Inputs, content: &dyn Content) -> Draft {
    let (docs, _) = load_docs(content);
    let natural = inputs.sample["natural"].as_array().unwrap();
    let split = &inputs.sample["split"];
    let all: Vec<&Value> = inputs.mutations["mutations"]
        .as_array()
        .unwrap()
        .iter()
        .collect();
    let by_id: BTreeMap<String, &Value> = all.iter().map(|m| (s(m, "id"), *m)).collect();
    let muts: Vec<&Value> = all
        .iter()
        .copied()
        .filter(|m| m["outcome"] != "dropped" && m["skip"].is_null())
        .collect();
    let mut violators: BTreeMap<String, Vec<&Value>> = BTreeMap::new();
    for m in &muts {
        if m["kind"] == "violating_code" {
            violators.entry(s(m, "source")).or_default().push(m);
        }
    }

    let mut rows: Vec<Value> = Vec::new();
    let mut agent_truths: BTreeMap<String, BTreeMap<String, Value>> = BTreeMap::new();
    let mut settlements: BTreeMap<String, (&'static str, String)> = BTreeMap::new();
    let mut natural_rows: BTreeMap<String, Value> = BTreeMap::new();
    for n in natural {
        let id = s(n, "id");
        let mode = s(n, "mode");
        let src = &n["source"];
        let req = s(n, "req_id");
        let crit = n["crit_id"].as_str();
        let test = mode
            .contains('T')
            .then(|| body_block(content, &src["test"], "fn_name"));
        let code = mode
            .contains('C')
            .then(|| body_block(content, &src["code"], "symbol"));
        let mut agent = BTreeMap::new();
        for key in keys_for_mode(&mode) {
            if let Some(v) = natural_truth(&inputs.labels_a, &inputs.labels_b, &id, key) {
                agent.insert((*key).to_owned(), v);
            }
        }
        let mut t = agent.clone();
        if mode.contains('T') {
            let trace_dual_yes = agent
                .get("trace_correct")
                .is_some_and(|v| v["answer"] == "yes" && v["kind"] == "agent_dual");
            let own = violators.get(&id).map_or(&[][..], Vec::as_slice);
            match settlement(trace_dual_yes, own) {
                Ok((answer, why)) => {
                    let prior = agent
                        .get("test_asserts_intent")
                        .map_or_else(String::new, |v| {
                            format!(
                                " (Agent labels, superseded: {})",
                                v["rationale"].as_str().unwrap_or("")
                            )
                        });
                    t.insert(
                        "test_asserts_intent".to_owned(),
                        truth(answer, "mechanical", &[], &format!("{why}{prior}")),
                    );
                    settlements.insert(id.clone(), (answer, why));
                }
                Err(Some(evidence)) => {
                    if let Some(v) = t.get_mut("test_asserts_intent") {
                        let r = format!("{}{evidence}", v["rationale"].as_str().unwrap_or(""));
                        v["rationale"] = json!(r);
                    }
                }
                Err(None) => {}
            }
        }
        let crate_name = src["crate"]
            .as_str()
            .map_or_else(|| "spec".to_owned(), str::to_owned);
        let strata = json!({
            "fr_id": req,
            "req_kind": req_kind(&req, crit),
            "test_kind": if mode.contains('T') { src["test_kind"].clone() } else { Value::Null },
            "crate": crate_name,
            "ears_pattern": crit
                .and_then(|c| docs[&req].criteria.get(c))
                .map(|text| ears_pattern(text)),
        });
        let row = json!({
            "id": id,
            "mode": mode,
            "split": split[&id],
            "strata": strata,
            "requirement": requirement_block(&docs, &req, crit),
            "test": test,
            "code": code,
            "ref": Value::Null,
            "mutation": Value::Null,
            "truth": t,
        });
        agent_truths.insert(id.clone(), agent);
        natural_rows.insert(id.clone(), row.clone());
        rows.push(row);
    }

    // A mutant shown in a mode other than its natural source's is paired with
    // that source's projection into the mode: the same unit and split, only
    // the id and mode differ (PR #625 review: bar D pairs within one mode).
    // A projection is a natural row and counts toward the per-FR cap; where
    // the cap is full the mutant is not shown in that mode.
    let mut per_fr: BTreeMap<String, usize> = BTreeMap::new();
    for n in natural {
        *per_fr.entry(s(n, "req_id")).or_default() += 1;
    }
    let natural_mode: BTreeMap<String, String> =
        natural.iter().map(|n| (s(n, "id"), s(n, "mode"))).collect();
    let mut projected: BTreeMap<(String, String), String> = BTreeMap::new();
    let mut dropped_modes: Vec<String> = Vec::new();
    for m in &muts {
        let source_id = s(m, "source");
        for mode in m["modes"].as_array().unwrap() {
            let mode = mode.as_str().unwrap().to_owned();
            let key = (source_id.clone(), mode.clone());
            if mode == natural_mode[&source_id]
                || projected.contains_key(&key)
                || dropped_modes.contains(&format!("{source_id} {mode}"))
            {
                continue;
            }
            let req = s(&natural_rows[&source_id]["requirement"], "fr_id");
            let count = per_fr.entry(req).or_default();
            if *count >= MAX_NATURAL_ROWS_PER_FR {
                dropped_modes.push(format!("{source_id} {mode}"));
                continue;
            }
            *count += 1;
            let id = format!("{source_id}@{mode}");
            rows.push(projection(
                &natural_rows[&source_id],
                &id,
                &mode,
                &agent_truths[&source_id],
                settlements.get(&source_id),
            ));
            projected.insert(key, id);
        }
    }

    for m in &muts {
        let source_id = s(m, "source");
        let base = &natural_rows[&source_id];
        let source_agent = &agent_truths[&source_id];
        let (determined, inherit) =
            mutation_truth(m, source_agent, settlements.get(&source_id), &by_id);
        // A trace swap shown without a test edits no file: no find/replace.
        let find = m["find"].as_str().unwrap_or_default().to_owned();
        let replace = m["replace"].as_str().unwrap_or_default().to_owned();
        for mode in m["modes"].as_array().unwrap() {
            let mode = mode.as_str().unwrap();
            let pair_source = if mode == natural_mode[&source_id] {
                source_id.clone()
            } else if let Some(id) = projected.get(&(source_id.clone(), mode.to_owned())) {
                id.clone()
            } else {
                continue;
            };
            let mut row = base.clone();
            row["mode"] = json!(mode);
            if !mode.contains('T') {
                row["test"] = Value::Null;
                row["strata"]["test_kind"] = Value::Null;
            }
            if !mode.contains('C') {
                row["code"] = Value::Null;
            }
            let edit = |body: &str, what: &str| {
                replace_once(body, &find, &replace)
                    .unwrap_or_else(|| panic!("{}: find is not once in the {what}", m["id"]))
            };
            match s(m, "target").as_str() {
                "code" => {
                    let body = s(&row["code"], "body");
                    row["code"]["body"] = json!(edit(&body, "code body"));
                }
                "test" => {
                    if mode.contains('T') {
                        let body = s(&row["test"], "body");
                        row["test"]["body"] = json!(edit(&body, "test body"));
                    }
                }
                "requirement" => {
                    let ac = row["requirement"]["ac_text"]
                        .as_str()
                        .unwrap_or("")
                        .to_owned();
                    if let Some(new) = replace_once(&ac, &find, &replace) {
                        row["requirement"]["ac_text"] = json!(new);
                    } else {
                        let statement = s(&row["requirement"], "statement");
                        row["requirement"]["statement"] = json!(edit(&statement, "requirement"));
                    }
                }
                "trace" => {
                    let partner = s(m, "partner_req_id");
                    let partner_crit = m["partner_crit_id"].as_str();
                    row["requirement"] = requirement_block(&docs, &partner, partner_crit);
                    row["strata"]["fr_id"] = json!(partner);
                    row["strata"]["req_kind"] = json!(req_kind(&partner, partner_crit));
                    row["strata"]["ears_pattern"] = json!(
                        partner_crit
                            .and_then(|c| docs[&partner].criteria.get(c))
                            .map(|text| ears_pattern(text))
                    );
                    if mode.contains('T') {
                        let body = s(&row["test"], "body");
                        row["test"]["body"] = json!(edit(&body, "test body (trace line)"));
                    }
                }
                other => panic!("unknown target {other}"),
            }
            let mut t = BTreeMap::new();
            for d in &determined {
                if keys_for_mode(mode).contains(&d.key) {
                    t.insert(
                        d.key.to_owned(),
                        truth(&d.answer, d.kind, &[], &d.rationale),
                    );
                }
            }
            for key in &inherit {
                if t.contains_key(*key) || !keys_for_mode(mode).contains(key) {
                    continue;
                }
                if let Some(v) = source_agent.get(*key) {
                    let mut v = v.clone();
                    v["rationale"] = json!(format!(
                        "INHERITED from natural row {source_id}; this mutation does not change \
                         what the question reads. {}",
                        v["rationale"].as_str().unwrap_or("")
                    ));
                    t.insert((*key).to_owned(), v);
                }
            }
            row["truth"] = serde_json::to_value(&t).unwrap();
            // The harness's targets are requirement / test / code; a trace swap
            // replaces the requirement (and rewrites the test's Trace line to match).
            let target = match s(m, "target").as_str() {
                "trace" => "requirement".to_owned(),
                other => other.to_owned(),
            };
            row["mutation"] = json!({
                "id": s(m, "id"),
                "target": target,
                "kind": s(m, "kind"),
                "description": s(m, "description"),
                "patch": s(m, "patch"),
                "source_id": pair_source,
            });
            rows.push(row);
        }
    }
    Draft {
        rows,
        dropped_modes,
    }
}

/// The keys a projection of a natural row into `mode` carries: those whose
/// question reads only artifacts the projection still shows. Divergence and
/// severity weigh the test against the code, so they are not carried.
fn projected_keys(mode: &str) -> &'static [&'static str] {
    match mode {
        "RC" => &[
            "trace_correct",
            "code_implements_intent",
            "code_exceeds_requirement",
        ],
        "RT" => &[
            "trace_correct",
            "assertion_vacuous",
            "test_asserts_intent",
            "tests_only_its_own_mock",
        ],
        other => panic!("no projection into mode {other}"),
    }
}

/// `base` (a natural row) shown in `mode` under `id`, with the labels that
/// still apply.
fn projection(
    base: &Value,
    id: &str,
    mode: &str,
    agent: &BTreeMap<String, Value>,
    settled: Option<&(&'static str, String)>,
) -> Value {
    let source = s(base, "id");
    let mut row = base.clone();
    row["id"] = json!(id);
    row["mode"] = json!(mode);
    if !mode.contains('T') {
        row["test"] = Value::Null;
        row["strata"]["test_kind"] = Value::Null;
    }
    if !mode.contains('C') {
        row["code"] = Value::Null;
    }
    let mut t = BTreeMap::new();
    for key in projected_keys(mode) {
        if *key == "test_asserts_intent"
            && let Some((answer, why)) = settled
        {
            t.insert(
                (*key).to_owned(),
                truth(
                    answer,
                    "mechanical",
                    &[],
                    &format!("Settled on natural row {source}; by its violating mutants: {why}"),
                ),
            );
            continue;
        }
        if let Some(v) = agent.get(*key) {
            let mut v = v.clone();
            v["rationale"] = json!(format!(
                "Labelled on natural row {source}; (the same unit, shown in mode {}), and \
                 carried because this question reads only what mode {mode} still shows. {}",
                s(base, "mode"),
                v["rationale"].as_str().unwrap_or("")
            ));
            t.insert((*key).to_owned(), v);
        }
    }
    row["truth"] = serde_json::to_value(&t).unwrap();
    row
}

/// The rows before final ids, and the (source, mode) pairs a mutant was not
/// shown in because the per-FR natural-row cap was full.
pub(crate) struct Draft {
    /// Natural rows in draw order, then projections, then mutation rows in
    /// log order. Ids are still `N-…` (`N-…@<mode>` for a projection).
    pub(crate) rows: Vec<Value>,
    /// `"<source> <mode>"` for every mode a mutant lost to the cap.
    pub(crate) dropped_modes: Vec<String>,
}

/// The whole corpus file, final ids assigned: `EV2-NNNN`, natural rows first.
pub(crate) fn assemble(inputs: &Inputs, content: &dyn Content) -> Value {
    let mut rows = draft_rows(inputs, content).rows;
    let mut id_map: Vec<(String, String)> = Vec::new();
    for (i, row) in rows.iter_mut().enumerate() {
        let new_id = format!("EV2-{:04}", i + 1);
        if row["mutation"].is_null() {
            id_map.push((s(row, "id"), new_id.clone()));
        }
        row["id"] = json!(new_id);
    }
    let final_id: BTreeMap<&str, &str> = id_map
        .iter()
        .map(|(old, new)| (old.as_str(), new.as_str()))
        .collect();
    for row in &mut rows {
        if let Some(source) = row["mutation"]["source_id"].as_str() {
            row["mutation"]["source_id"] = json!(final_id[source]);
        }
        if let Some(t) = row["truth"].as_object_mut() {
            for v in t.values_mut() {
                let mut r = s(v, "rationale");
                for (old, new) in &id_map {
                    r = r.replace(
                        &format!("natural row {old};"),
                        &format!("natural row {new};"),
                    );
                }
                v["rationale"] = json!(r);
            }
        }
    }
    for row in &rows {
        let mode = s(row, "mode");
        for key in row["truth"].as_object().unwrap().keys() {
            assert!(
                keys_for_mode(&mode).contains(&key.as_str()),
                "{}: {key}",
                row["id"]
            );
        }
    }
    json!({
        "schema": SCHEMA,
        "sampling_rule": SAMPLING_RULE,
        "seed": SEED,
        "source_commit": inputs.sample["source_commit"],
        "labelling_rules": labelling_rules(),
        "rows": rows,
    })
}

/// The canonical bytes `corpus.json` holds for `corpus`.
pub(crate) fn corpus_bytes(corpus: &Value) -> String {
    let text = serde_json::to_string(corpus).unwrap();
    let parsed = quoin_store::parse_strict_json_str(&text).unwrap();
    quoin_store::canonical_json(&parsed).unwrap()
}
