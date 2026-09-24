// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The builder for Jev evaluation corpus v2 (PLAT-1025). Every test here is
//! `#[ignore]`d: they write fixtures, shell out to `quire`, and take minutes.
//! Run them by hand, in order, from `rust/`:
//!
//! ```text
//! cargo test -p quoin-jev --test eval_v2_build -- --ignored --exact eval_v2_1_enumerate
//! cargo test -p quoin-jev --test eval_v2_build -- --ignored --exact eval_v2_2_sample
//! # author mutations.json; run run-mutants.sh; collect the two label passes
//! cargo test -p quoin-jev --test eval_v2_build -- --ignored --exact eval_v2_3_assemble
//! ```
//!
//! 1. **enumerate** runs the deterministic trace scan gap-analysis step 3
//!    and step 4 run (`quire symbols` for test-to-requirement tags, `quire
//!    trace` for code citations of each requirement), resolves each tagged
//!    test to the code it calls, and writes `population.json`: references
//!    only, no bodies.
//! 2. **sample** reads `population.json` and draws the natural rows and the
//!    mutation targets by the seeded, stratified rule in
//!    [`assemble::SAMPLING_RULE`], writing `sample.json`.
//! 3. **assemble** runs [`assemble::assemble`] over `sample.json`,
//!    `mutations.json` (the mutation log, outcomes recorded by
//!    `run-mutants.sh`) and the two independent label passes, reading every
//!    body through git at [`assemble::CONTENT_COMMIT`], and writes
//!    `corpus.json` and `heldout.sha256`. `eval_v2_corpus.rs` reruns the same
//!    function in the default gate and requires the committed bytes.
//!
//! Only the agent labels and the mutation log are data rather than code: they
//! are recorded, not recomputed. Everything else is reproducible from the
//! seed and the committed inputs.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]
#![allow(
    clippy::too_many_lines,
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "a one-shot fixture builder: long linear stages read better unsplit, and every \
              count here is at most a few thousand"
)]

mod eval_v2_support;
mod gap_semantic_support;

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::process::Command;

use eval_v2_support::assemble::{
    self, CONTENT_COMMIT, GitContent, Inputs, ReqDoc, SAMPLING_RULE, SEED, TreeContent,
    corpus_bytes, ears_pattern, fixture_dir, load_docs, repo_root, req_kind,
};
use eval_v2_support::corpus::{Origin, heldout_digest, parse, validate};
use regex::Regex;
use serde_json::{Value, json};

/// At most this many natural rows cite any one requirement.
const NATURAL_ROWS_PER_REQUIREMENT: usize = 3;

/// `SplitMix64` (Steele, Lea and Flood, 2014): small, dependency-free and
/// fully specified, so the sampling rule can be restated in prose and
/// re-implemented anywhere.
#[derive(Debug, Clone)]
struct SplitMix64(u64);

impl SplitMix64 {
    const fn new(seed: u64) -> Self {
        Self(seed)
    }

    fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// A draw in `0..n` by modulo (`n` is at most a few thousand here, so the
    /// bias is below 1e-15 and is accepted rather than rejected away).
    fn below(&mut self, n: usize) -> usize {
        let n64 = u64::try_from(n).expect("n fits u64");
        usize::try_from(self.next_u64() % n64).expect("a value below n fits usize")
    }

    /// Fisher-Yates, from the back.
    fn shuffle<T>(&mut self, items: &mut [T]) {
        for i in (1..items.len()).rev() {
            let j = self.below(i + 1);
            items.swap(i, j);
        }
    }
}

/// The seeded-draw quotas for natural rows.
const QUOTAS: [(&str, usize); 4] = [("RTC", 64), ("RC", 28), ("RT", 48), ("R", 58)];

/// Methods too generic to name the behaviour a test covers.
const BOILERPLATE: [&str; 22] = [
    "fmt",
    "deserialize",
    "serialize",
    "from",
    "try_from",
    "default",
    "clone",
    "eq",
    "hash",
    "into",
    "new",
    "as_str",
    "all",
    "from_str",
    "as_ref",
    "deref",
    "drop",
    "len",
    "is_empty",
    "main",
    "run",
    "to_string",
];

fn module_dir() -> PathBuf {
    std::env::var_os("QUOIN_EVAL_V2_MODULE").map_or_else(
        || {
            PathBuf::from(std::env::var_os("HOME").expect("HOME"))
                .join(".ix/filament/modules/spec-artifacts-process")
        },
        PathBuf::from,
    )
}

fn run(cmd: &mut Command) -> String {
    let out = cmd.output().expect("spawn");
    assert!(
        out.status.success(),
        "{cmd:?} failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stdout).expect("utf-8 stdout")
}

fn read(path: &Path) -> String {
    std::fs::read_to_string(path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

fn write_json(path: &Path, value: &Value) {
    let text = serde_json::to_string(value).unwrap();
    let parsed = quoin_store::parse_strict_json_str(&text).unwrap();
    std::fs::write(path, quoin_store::canonical_json(&parsed).unwrap()).unwrap();
}

fn load_json(path: &Path) -> Value {
    serde_json::from_str(&read(path)).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
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
// Stage 1: enumerate
// ---------------------------------------------------------------------------

/// A code candidate's rank: call count, name length, -first call.
type Score = (usize, usize, i64);

fn crate_of(path: &str) -> String {
    let parts: Vec<&str> = path.split('/').collect();
    if path.starts_with("rust/crates/") {
        parts[2].to_owned()
    } else {
        parts[0].to_owned()
    }
}

fn is_test_path(path: &str) -> bool {
    path.contains("/tests/") || path.ends_with("/tests.rs") || path.contains("/benches/")
}

fn in_scope(path: &str) -> bool {
    (path.starts_with("rust/crates/") || path.starts_with("templates/"))
        && !path.contains("node_modules")
        && !path.contains("/oracle/")
}

fn lines_of(root: &Path, path: &str, cache: &mut BTreeMap<String, Vec<String>>) -> Vec<String> {
    cache
        .entry(path.to_owned())
        .or_insert_with(|| read(&root.join(path)).lines().map(str::to_owned).collect())
        .clone()
}

fn span(lines: &[String], from: usize, to: usize) -> String {
    let from = from.max(1) - 1;
    let to = to.min(lines.len());
    lines[from..to].join("\n")
}

fn test_kind(language: &str, path: &str, file_text: &str) -> &'static str {
    if file_text.contains("proptest!") || file_text.contains("use proptest") {
        "property"
    } else if file_text.contains("CARGO_BIN_EXE") {
        "cli"
    } else if language == "rust" && !is_test_path(path) {
        "unit"
    } else {
        "integration"
    }
}

#[test]
#[ignore = "fixture builder: shells out to quire and rewrites population.json"]
fn eval_v2_1_enumerate() {
    let root = repo_root();
    let module = module_dir();
    let quire_version = run(Command::new("quire").arg("--version"))
        .trim()
        .to_owned();
    let commit = run(Command::new("git")
        .arg("-C")
        .arg(&root)
        .args(["rev-parse", "HEAD"]))
    .trim()
    .to_owned();
    let symbols: Value = serde_json::from_str(&run(Command::new("quire")
        .arg("symbols")
        .arg("--scope")
        .arg(&root)
        .arg("--module")
        .arg(&module)
        .arg("--json")))
    .unwrap();
    let symbols: Vec<Value> = symbols["symbols"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|r| in_scope(r["path"].as_str().unwrap()))
        .cloned()
        .collect();
    let (docs, ambiguous) = load_docs(&TreeContent(root.clone()));

    // `quire trace` per requirement, 8 at a time: one call is ~30 s.
    let ids: Vec<String> = docs.keys().cloned().collect();
    let mut citations: BTreeMap<String, Vec<Value>> = BTreeMap::new();
    for chunk in ids.chunks(8) {
        let results: Vec<(String, Vec<Value>)> = std::thread::scope(|scope| {
            let handles: Vec<_> = chunk
                .iter()
                .map(|id| {
                    let root = &root;
                    let module = &module;
                    scope.spawn(move || {
                        let out = run(Command::new("quire")
                            .arg("trace")
                            .arg("--scope")
                            .arg(root)
                            .arg("--module")
                            .arg(module)
                            .args(["--id", id, "--prefix", "--json"]));
                        let report: Value = serde_json::from_str(&out).unwrap();
                        (
                            id.clone(),
                            report["citations"]
                                .as_array()
                                .cloned()
                                .unwrap_or_default()
                                .into_iter()
                                .filter(|c| c["path"].as_str().is_some_and(in_scope))
                                .collect(),
                        )
                    })
                })
                .collect();
            handles.into_iter().map(|h| h.join().unwrap()).collect()
        });
        citations.extend(results);
    }

    // Non-test Rust functions, per crate: the code a test can resolve to.
    let mut functions: BTreeMap<String, Vec<&Value>> = BTreeMap::new();
    for r in &symbols {
        let path = r["path"].as_str().unwrap();
        let symbol = r["symbol"].as_str().unwrap();
        let name = symbol.rsplit("::").next().unwrap();
        if r["kind"] != "function"
            || r["language"] != "rust"
            || is_test_path(path)
            || symbol.starts_with("tests::")
            || r["container"]
                .as_str()
                .is_some_and(|c| c.starts_with("tests"))
            || BOILERPLATE.contains(&name)
            || u(r, "end_line") < u(r, "line") + 2
        {
            continue;
        }
        functions.entry(crate_of(path)).or_default().push(r);
    }

    let criterion = Regex::new(r"^((?:FR|NFR|StR|US)-\d+)(?:-(?:AC|CON|VC|M)-\d+)?$").unwrap();
    let mut cache = BTreeMap::new();
    let mut pairs = Vec::new();
    let mut tested: BTreeSet<String> = BTreeSet::new();
    for t in symbols.iter().filter(|r| r["kind"] == "test_function") {
        let path = s(t, "path");
        let mut roots: Vec<(String, Option<String>)> = Vec::new();
        for id in t["trace_ids"].as_array().unwrap() {
            let id = id.as_str().unwrap();
            let Some(c) = criterion.captures(id) else {
                continue;
            };
            let req = c[1].to_owned();
            let crit = (id != req).then(|| id.to_owned());
            match roots.iter_mut().find(|(r, _)| *r == req) {
                Some(entry) => {
                    if entry.1.is_none() {
                        entry.1 = crit;
                    }
                }
                None => roots.push((req, crit)),
            }
        }
        if roots.is_empty() {
            continue;
        }
        let lines = lines_of(&root, &path, &mut cache);
        let body = span(&lines, u(t, "line"), u(t, "end_line"));
        let file_text = lines.join("\n");
        let kind = test_kind(t["language"].as_str().unwrap(), &path, &file_text);
        for (req, crit) in roots {
            if !docs.contains_key(&req) {
                continue;
            }
            tested.insert(req.clone());
            let cited: BTreeSet<&str> = citations
                .get(&req)
                .map(|cs| cs.iter().filter_map(|c| c["path"].as_str()).collect())
                .unwrap_or_default();
            let mut best: Option<(Score, &Value)> = None;
            if t["language"] == "rust" {
                for f in functions
                    .get(&crate_of(&path))
                    .map_or(&[][..], Vec::as_slice)
                {
                    // Step 4's code-to-requirement trace: only a function in a
                    // file `quire trace` cites for this requirement is its code.
                    if !cited.contains(f["path"].as_str().unwrap()) {
                        continue;
                    }
                    let name = f["symbol"].as_str().unwrap().rsplit("::").next().unwrap();
                    let call = Regex::new(&format!(
                        r"(?:^|[^A-Za-z0-9_]){}\s*(?:::<[^>]*>)?\(",
                        regex::escape(name)
                    ))
                    .unwrap();
                    let hits: Vec<usize> = call.find_iter(&body).map(|m| m.start()).collect();
                    let Some(first) = hits.first() else { continue };
                    let score = (hits.len(), name.len(), -i64::try_from(*first).unwrap());
                    if best.as_ref().is_none_or(|(b, _)| score > *b) {
                        best = Some((score, f));
                    }
                }
            }
            pairs.push(json!({
                "test": {"path": path, "symbol": s(t, "symbol"), "line": u(t, "line"),
                         "leading_line": u(t, "leading_line"), "end_line": u(t, "end_line"),
                         "language": s(t, "language")},
                "test_kind": kind,
                "crate": crate_of(&path),
                "req_id": req,
                "crit_id": crit,
                "code": best.map(|(_, f)| json!({"path": s(f, "path"), "symbol": s(f, "symbol"),
                    "line": u(f, "line"), "leading_line": u(f, "leading_line"),
                    "end_line": u(f, "end_line")})),
            }));
        }
    }

    // Requirements no test cites, with code that cites them.
    let mut testless = Vec::new();
    for (req, cs) in &citations {
        if tested.contains(req) {
            continue;
        }
        let mut seen = BTreeSet::new();
        for c in cs {
            let path = c["path"].as_str().unwrap();
            if is_test_path(path)
                || !Path::new(path)
                    .extension()
                    .is_some_and(|x| x.eq_ignore_ascii_case("rs"))
            {
                continue;
            }
            let line = u(c, "line");
            let enclosing = functions.get(&crate_of(path)).and_then(|fs| {
                fs.iter().find(|f| {
                    f["path"] == path && u(f, "leading_line") <= line && line <= u(f, "end_line")
                })
            });
            if let Some(f) = enclosing
                && seen.insert(s(f, "symbol"))
            {
                testless.push(json!({
                    "req_id": req, "crate": crate_of(path),
                    "code": {"path": s(f, "path"), "symbol": s(f, "symbol"), "line": u(f, "line"),
                             "leading_line": u(f, "leading_line"), "end_line": u(f, "end_line")},
                }));
            }
        }
    }

    let criteria: Vec<Value> = docs
        .values()
        .flat_map(|d| {
            d.order
                .iter()
                .map(|c| json!({"req_id": d.id, "crit_id": c, "doc": d.path}))
        })
        .collect();

    let population = json!({
        "generated_by": "eval_v2_build.rs::eval_v2_1_enumerate",
        "quire_version": quire_version,
        "source_commit": commit,
        "module": "spec-artifacts-process",
        "ambiguous_requirement_ids": ambiguous,
        "pairs": pairs,
        "testless_code": testless,
        "criteria": criteria,
    });
    std::fs::create_dir_all(fixture_dir()).unwrap();
    write_json(&fixture_dir().join("population.json"), &population);
    println!(
        "pairs {} (with code {}), testless code {}, criteria {}",
        pairs.len(),
        pairs.iter().filter(|p| !p["code"].is_null()).count(),
        testless.len(),
        criteria.len()
    );
}

// ---------------------------------------------------------------------------
// Stage 2: sample
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
struct Candidate {
    mode: &'static str,
    stratum: String,
    req_id: String,
    crit_id: Option<String>,
    test_key: Option<String>,
    code_key: Option<String>,
    source: Value,
}

fn key_of(v: &Value) -> Option<String> {
    (!v.is_null()).then(|| format!("{}#{}", s(v, "path"), s(v, "symbol")))
}

fn candidates(population: &Value, docs: &BTreeMap<String, ReqDoc>) -> Vec<Candidate> {
    let mut out = Vec::new();
    for p in population["pairs"].as_array().unwrap() {
        let req = s(p, "req_id");
        let crit = p["crit_id"].as_str().map(str::to_owned);
        let kind = req_kind(&req, crit.as_deref());
        let stratum_tail = format!("{kind}|{}|{}", s(p, "test_kind"), s(p, "crate"));
        let has_code = !p["code"].is_null();
        let modes: &[&'static str] = if has_code {
            &["RTC", "RT", "RC"]
        } else {
            &["RT"]
        };
        for mode in modes {
            let (test_key, code_key) = match *mode {
                "RTC" => (key_of(&p["test"]), key_of(&p["code"])),
                "RT" => (key_of(&p["test"]), None),
                _ => (None, key_of(&p["code"])),
            };
            let stratum = if *mode == "RC" {
                format!("RC|{kind}|-|{}", s(p, "crate"))
            } else {
                format!("{mode}|{stratum_tail}")
            };
            out.push(Candidate {
                mode,
                stratum,
                req_id: req.clone(),
                crit_id: crit.clone(),
                test_key,
                code_key,
                source: p.clone(),
            });
        }
    }
    for p in population["testless_code"].as_array().unwrap() {
        let req = s(p, "req_id");
        let kind = req_kind(&req, None);
        out.push(Candidate {
            mode: "RC",
            stratum: format!("RC|{kind}|-|{}", s(p, "crate")),
            req_id: req,
            crit_id: None,
            test_key: None,
            code_key: key_of(&p["code"]),
            source: p.clone(),
        });
    }
    for c in population["criteria"].as_array().unwrap() {
        let req = s(c, "req_id");
        let crit = s(c, "crit_id");
        let text = docs
            .get(&req)
            .and_then(|d| d.criteria.get(&crit))
            .cloned()
            .unwrap_or_default();
        out.push(Candidate {
            mode: "R",
            stratum: format!("R|{}|{}", req_kind(&req, Some(&crit)), ears_pattern(&text)),
            req_id: req,
            crit_id: Some(crit),
            test_key: None,
            code_key: None,
            source: c.clone(),
        });
    }
    out
}

/// How to run exactly one test: `cargo test -p <package> <selector...> -- --exact <name>`.
/// `null` for a row with no Rust test.
fn owning_test(root: &Path, test: &Value) -> Value {
    if test.is_null() || test["language"] != "rust" {
        return Value::Null;
    }
    let path = s(test, "path");
    let parts: Vec<&str> = path.split('/').collect();
    let package = parts[2];
    let crate_dir = root.join("rust/crates").join(package);
    let symbol = s(test, "symbol");
    let (selector, name) = if parts[3] == "tests" {
        let stem = parts[4].trim_end_matches(".rs");
        (vec!["--test".to_owned(), stem.to_owned()], symbol)
    } else {
        let rel = parts[4..].join("/");
        let module = rel.trim_end_matches(".rs");
        let module = module.strip_suffix("/mod").unwrap_or(module);
        let module = if matches!(module, "lib" | "main") {
            ""
        } else {
            module
        };
        let selector = if crate_dir.join("src/lib.rs").exists() {
            vec!["--lib".to_owned()]
        } else {
            let manifest = read(&crate_dir.join("Cargo.toml"));
            let bin = manifest
                .split("[[bin]]")
                .nth(1)
                .and_then(|b| b.lines().find_map(|l| l.strip_prefix("name = ")))
                .map(|n| n.trim_matches('"').to_owned())
                .expect("a crate with no lib.rs declares a [[bin]]");
            vec!["--bin".to_owned(), bin]
        };
        let name = if module.is_empty() {
            symbol
        } else {
            format!("{}::{symbol}", module.replace('/', "::"))
        };
        (selector, name)
    };
    let filter = name.rsplit("::").next().unwrap().to_owned();
    json!({"package": package, "selector": selector, "name": name, "filter": filter})
}

/// One mode's draw state: strata in rotation order, each a queue.
struct Rotation {
    strata: Vec<VecDeque<Candidate>>,
    next: usize,
}

#[test]
#[ignore = "fixture builder: rewrites sample.json from population.json and the seed"]
fn eval_v2_2_sample() {
    let root = repo_root();
    let population = load_json(&fixture_dir().join("population.json"));
    let (docs, _) = load_docs(&TreeContent(root.clone()));
    let all = candidates(&population, &docs);
    let mut rng = SplitMix64::new(SEED);

    let mut rotations: BTreeMap<&str, Rotation> = BTreeMap::new();
    for mode in ["R", "RT", "RC", "RTC"] {
        let mut groups: BTreeMap<String, Vec<Candidate>> = BTreeMap::new();
        for c in all.iter().filter(|c| c.mode == mode) {
            groups.entry(c.stratum.clone()).or_default().push(c.clone());
        }
        let mut strata: Vec<VecDeque<Candidate>> = groups
            .into_values()
            .map(|mut g| {
                rng.shuffle(&mut g);
                g.into()
            })
            .collect();
        rng.shuffle(&mut strata);
        rotations.insert(mode, Rotation { strata, next: 0 });
    }

    let quota: BTreeMap<&str, usize> = QUOTAS.into_iter().collect();
    let mut filled: BTreeMap<&str, usize> = BTreeMap::new();
    let mut per_req: BTreeMap<String, usize> = BTreeMap::new();
    let mut used_tests = BTreeSet::new();
    let mut used_code = BTreeSet::new();
    let mut used_crit = BTreeSet::new();
    let mut natural: Vec<Candidate> = Vec::new();
    loop {
        let mut progressed = false;
        for (mode, _) in QUOTAS {
            if filled.get(mode).copied().unwrap_or(0) >= quota[mode] {
                continue;
            }
            let rotation = rotations.get_mut(mode).unwrap();
            let mut taken = None;
            while taken.is_none() && !rotation.strata.is_empty() {
                let at = rotation.next % rotation.strata.len();
                let queue = &mut rotation.strata[at];
                while let Some(c) = queue.pop_front() {
                    let ok = per_req.get(&c.req_id).copied().unwrap_or(0)
                        < NATURAL_ROWS_PER_REQUIREMENT
                        && c.test_key.as_ref().is_none_or(|k| !used_tests.contains(k))
                        && c.code_key.as_ref().is_none_or(|k| !used_code.contains(k))
                        && (c.mode != "R"
                            || c.crit_id.as_ref().is_none_or(|k| !used_crit.contains(k)));
                    if ok {
                        taken = Some(c);
                        break;
                    }
                }
                if rotation.strata[at].is_empty() {
                    rotation.strata.remove(at);
                    if taken.is_some() && !rotation.strata.is_empty() {
                        rotation.next = at % rotation.strata.len();
                    }
                } else {
                    rotation.next = at + 1;
                }
            }
            if let Some(c) = taken {
                *per_req.entry(c.req_id.clone()).or_default() += 1;
                if let Some(k) = &c.test_key {
                    used_tests.insert(k.clone());
                }
                if let Some(k) = &c.code_key {
                    used_code.insert(k.clone());
                }
                if c.mode == "R"
                    && let Some(k) = &c.crit_id
                {
                    used_crit.insert(k.clone());
                }
                *filled.entry(mode).or_default() += 1;
                natural.push(c);
                progressed = true;
            }
        }
        if !progressed {
            break;
        }
    }

    // Natural row ids in draw order.
    let natural_rows: Vec<Value> = natural
        .iter()
        .enumerate()
        .map(|(i, c)| {
            json!({
                "id": format!("N-{:03}", i + 1),
                "mode": c.mode,
                "stratum": c.stratum,
                "req_id": c.req_id,
                "crit_id": c.crit_id,
                "source": c.source,
                "owning_test": owning_test(&root, &c.source["test"]),
            })
        })
        .collect();
    let ids_of = |mode: &str| -> Vec<String> {
        natural_rows
            .iter()
            .filter(|r| r["mode"] == mode)
            .map(|r| s(r, "id"))
            .collect()
    };

    // Mutation targets.
    let mut rtc = ids_of("RTC");
    rng.shuffle(&mut rtc);
    let code_sources: Vec<String> = rtc.iter().take(30).cloned().collect();
    let mut plan = Vec::new();
    let additive: Vec<&String> = code_sources.iter().step_by(2).collect();
    let weakening: Vec<&String> = code_sources.iter().skip(1).step_by(2).collect();
    for (i, id) in code_sources.iter().enumerate() {
        plan.push(json!({"slot": format!("VIOL-{:02}", i + 1), "kind": "violating_code",
                         "source": id, "modes": if i < 15 { json!(["RTC", "RC"]) } else { json!(["RTC"]) }}));
    }
    for (i, id) in additive.iter().enumerate() {
        plan.push(json!({"slot": format!("ADD-{:02}", i + 1), "kind": "additive_code",
                         "source": id, "modes": if i < 8 { json!(["RTC", "RC"]) } else { json!(["RTC"]) }}));
    }
    for (i, id) in weakening.iter().enumerate() {
        plan.push(json!({"slot": format!("WEAK-{:02}", i + 1), "kind": "test_weakening",
                         "source": id, "modes": if i < 8 { json!(["RTC", "RT"]) } else { json!(["RTC"]) }}));
    }
    let mut rt = ids_of("RT");
    rng.shuffle(&mut rt);
    for (i, id) in rt.iter().take(16).enumerate() {
        plan.push(
            json!({"slot": format!("REQ-{:02}", i + 1), "kind": "requirement_text",
                         "source": id, "modes": ["RT"]}),
        );
    }
    let mut r = ids_of("R");
    rng.shuffle(&mut r);
    let defects = [
        "vague_term",
        "no_measurable_threshold",
        "compound",
        "missing_trigger",
        "untestable",
    ];
    for (i, id) in r.iter().take(30).enumerate() {
        plan.push(
            json!({"slot": format!("CRIT-{:02}", i + 1), "kind": "criterion",
                         "defect": defects[i % defects.len()], "source": id, "modes": ["R"]}),
        );
    }
    // Trace swaps: sources not already code-mutant sources; partner requirement
    // drawn from those whose tests live in another crate.
    let mut crates_of_req: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for p in population["pairs"].as_array().unwrap() {
        crates_of_req
            .entry(s(p, "req_id"))
            .or_default()
            .insert(s(p, "crate"));
    }
    let source_crate = |id: &str| -> String {
        let row = natural_rows.iter().find(|r| r["id"] == id).unwrap();
        s(&row["source"], "crate")
    };
    let mut swap_n = 0;
    for (mode, n) in [("RTC", 6), ("RT", 5), ("RC", 5)] {
        let mut pool: Vec<String> = ids_of(mode)
            .into_iter()
            .filter(|id| !code_sources.contains(id) && !rt.iter().take(16).any(|x| x == id))
            .collect();
        rng.shuffle(&mut pool);
        for id in pool.into_iter().take(n) {
            let own = source_crate(&id);
            let row_req = s(
                natural_rows.iter().find(|r| r["id"] == id).unwrap(),
                "req_id",
            );
            let partners: Vec<&String> = crates_of_req
                .iter()
                .filter(|(req, crates)| **req != row_req && !crates.contains(&own))
                .map(|(req, _)| req)
                .collect();
            let partner = partners[rng.below(partners.len())].clone();
            swap_n += 1;
            plan.push(
                json!({"slot": format!("SWAP-{swap_n:02}"), "kind": "trace_swap",
                             "source": id, "partner_req_id": partner, "modes": [mode]}),
            );
        }
    }

    // Split by family (a natural row and everything derived from it).
    let mut split: BTreeMap<String, &str> = BTreeMap::new();
    for mode in ["R", "RT", "RC", "RTC"] {
        let mut fam = ids_of(mode);
        rng.shuffle(&mut fam);
        let dev = (fam.len() as f64 * 0.35).round() as usize;
        for (i, id) in fam.into_iter().enumerate() {
            split.insert(id, if i < dev { "dev" } else { "heldout" });
        }
    }

    let sample = json!({
        "generated_by": "eval_v2_build.rs::eval_v2_2_sample",
        "seed": SEED,
        "sampling_rule": SAMPLING_RULE,
        "source_commit": population["source_commit"],
        "quire_version": population["quire_version"],
        "natural": natural_rows,
        "mutation_plan": plan,
        // The full seeded order the code-mutant sources were taken from. A
        // source whose code the mutation author found does not implement the
        // shown requirement (the call-name resolution picked the wrong
        // symbol) is replaced by the next row in this order; mutations.json
        // records every replaced slot and why.
        "code_source_order": rtc,
        "split": split,
    });
    write_json(&fixture_dir().join("sample.json"), &sample);
    println!("natural {} {:?}", natural.len(), filled);
}

// ---------------------------------------------------------------------------
// Stage 3: assemble
// ---------------------------------------------------------------------------

#[test]
#[ignore = "fixture builder: rewrites corpus.json and heldout.sha256"]
fn eval_v2_3_assemble() {
    let dir = fixture_dir();
    let content = GitContent::new(&repo_root(), CONTENT_COMMIT);
    // `QUOIN_EVAL_V2_DRAFT=<path>` writes the rows so far to <path> and stops
    // before validation: that draft is what the mutation authors and the label
    // passes were shown, so they saw exactly the bodies the corpus holds.
    if let Some(path) = std::env::var_os("QUOIN_EVAL_V2_DRAFT").map(PathBuf::from) {
        let inputs = Inputs::load(&dir);
        let draft = assemble::draft_rows(&inputs, &content);
        write_json(
            &path,
            &json!({"rows": draft.rows, "labelling_rules": assemble::labelling_rules()}),
        );
        return;
    }
    let inputs = Inputs::load(&dir);
    let draft = assemble::draft_rows(&inputs, &content);
    let corpus = assemble::assemble(&inputs, &content);
    let text = corpus_bytes(&corpus);
    std::fs::write(dir.join("corpus.json"), &text).unwrap();
    let problems = validate(&parse(&text).unwrap(), Origin::InRepo);
    assert!(problems.is_empty(), "{problems:#?}");
    std::fs::write(
        dir.join("heldout.sha256"),
        format!("{}\n", heldout_digest(&text).unwrap()),
    )
    .unwrap();

    let rows = corpus["rows"].as_array().unwrap();
    let mut mix: BTreeMap<String, usize> = BTreeMap::new();
    let mut tally: BTreeMap<(String, String, String), usize> = BTreeMap::new();
    for row in rows {
        let origin = if row["mutation"].is_null() {
            "natural".to_owned()
        } else {
            s(&row["mutation"], "kind")
        };
        *mix.entry(format!("{} {} {origin}", s(row, "split"), s(row, "mode")))
            .or_default() += 1;
        for (key, v) in row["truth"].as_object().unwrap() {
            *tally
                .entry((key.clone(), s(v, "answer"), s(v, "kind")))
                .or_default() += 1;
        }
    }
    let mut report = String::new();
    for (k, n) in &mix {
        writeln!(report, "{k}: {n}").unwrap();
    }
    for ((key, answer, kind), n) in &tally {
        writeln!(report, "{key}\t{answer}\t{kind}\t{n}").unwrap();
    }
    writeln!(
        report,
        "modes not shown (per-FR cap): {:?}",
        draft.dropped_modes
    )
    .unwrap();
    println!("rows {}\n{report}", rows.len());
}
