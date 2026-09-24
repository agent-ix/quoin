// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Python units in the corpus-v2 harness, offline (PLAT-1035).
//!
//! The external corpus (PLAT-1026) dropped 17 sampled units because the
//! harness read Rust `fn` items only. These tests check the Python half of
//! `eval_v2_support::units`: cutting one named `def` out of a file
//! (decorators, `async def`, methods, nested defs, docstrings, ambiguity),
//! splitting a Python body into units, choosing the language by extension,
//! and an external `.py` row materializing end to end.
//!
//! Provenance: PLAT-1035, PLAT-1024.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]

mod eval_v2_support;
mod gap_semantic_support;

use serde_json::{Value, json};

use eval_v2_support::corpus::{self, Origin, load_external, validate};
use eval_v2_support::keys::Mode;
use eval_v2_support::units::{
    Language, UnitKind, extract_item, extract_python_def, split_python_units, split_units,
};
use eval_v2_support::variant::{REGISTRY, code_units, per_unit_asks, wording_violations};
use typesafe_sdk_questions::{Questions, noul, questions};

// ---------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------

/// A module with a decorated function (multi-line decorator, attached
/// comment, a docstring holding a column-0 `def` line, a nested `def`), an
/// `async def`, and two classes sharing a method name.
const MODULE: &str = r#""""Module docstring."""
import functools


# Attached comment.
@functools.lru_cache(
    maxsize=None,
)
def load(path):
    """Load a thing.

def fake(): a column-0 line inside the docstring
    """
    def helper(x):
        return x + 1  # a comment: def not_a_def():
    if path:
        return helper(1)
    elif path is None:
        raise ValueError("none")
    else:
        return 0


async def fetch(url):
    data = await get(url)
    return data


class Store:
    """A store."""

    @staticmethod
    def helper(x):
        return x * 2

    async def save(self, item):
        # comment
        self.items.append(item)


class Cache:
    def save(self, item):
        pass
"#;

const LOAD: &str = r#"# Attached comment.
@functools.lru_cache(
    maxsize=None,
)
def load(path):
    """Load a thing.

def fake(): a column-0 line inside the docstring
    """
    def helper(x):
        return x + 1  # a comment: def not_a_def():
    if path:
        return helper(1)
    elif path is None:
        raise ValueError("none")
    else:
        return 0"#;

// ---------------------------------------------------------------------------
// Extraction
// ---------------------------------------------------------------------------

/// Provenance: PLAT-1035. A module-level `def` comes with its attached
/// comment, its multi-line decorator and its docstring; a column-0 line
/// inside the docstring does not end the body.
#[test]
fn tc_1035_a_decorated_def_comes_with_its_decorator_and_docstring() {
    assert_eq!(extract_python_def(MODULE, "load").unwrap(), LOAD);
}

/// Provenance: PLAT-1035. An `async def` is a `def`.
#[test]
fn tc_1035_an_async_def_is_extracted() {
    assert_eq!(
        extract_python_def(MODULE, "fetch").unwrap(),
        "async def fetch(url):\n    data = await get(url)\n    return data"
    );
}

/// Provenance: PLAT-1035. `Class.method` resolves inside `class Class`,
/// decorators and in-body comments included; `::` reads as `.`.
#[test]
fn tc_1035_a_method_resolves_inside_its_class() {
    assert_eq!(
        extract_python_def(MODULE, "Store.save").unwrap(),
        "    async def save(self, item):\n        # comment\n        self.items.append(item)"
    );
    assert_eq!(
        extract_python_def(MODULE, "Cache::save").unwrap(),
        "    def save(self, item):\n        pass"
    );
    assert_eq!(
        extract_python_def(MODULE, "Store.helper").unwrap(),
        "    @staticmethod\n    def helper(x):\n        return x * 2"
    );
    let error = extract_python_def(MODULE, "Store.load").unwrap_err();
    assert_eq!(
        error,
        "no `def Store.load` outside another function's body in the source"
    );
}

/// Provenance: PLAT-1035 (review finding 1). A qualifier naming no class in
/// the file is a module: only defs outside any class match, so `Foo.run`
/// never resolves to `Bar.run`, and `mymod.run` resolves to the module-level
/// `run`.
#[test]
fn tc_1035_a_qualifier_that_is_not_a_class_is_a_module() {
    let only_bar = "class Bar:\n    def run(self):\n        return 1\n";
    assert_eq!(
        extract_python_def(only_bar, "Foo.run").unwrap_err(),
        "no `def Foo.run` outside another function's body in the source"
    );
    let both = "class Bar:\n    def run(self):\n        return 1\n\n\ndef run():\n    return 2\n";
    assert_eq!(
        extract_python_def(both, "mymod.run").unwrap(),
        "def run():\n    return 2"
    );
    assert_eq!(
        extract_python_def(both, "Bar.run").unwrap(),
        "    def run(self):\n        return 1"
    );
}

/// Provenance: PLAT-1035. A `def` nested inside another `def` never matches:
/// `helper` resolves to the one method of that name, not to the helper
/// inside `load`; a name defined only in a nested `def`, only inside a
/// docstring, or only inside a comment is not found.
#[test]
fn tc_1035_a_nested_def_never_matches() {
    assert_eq!(
        extract_python_def(MODULE, "helper").unwrap(),
        "    @staticmethod\n    def helper(x):\n        return x * 2"
    );
    for name in ["fake", "not_a_def"] {
        assert_eq!(
            extract_python_def(MODULE, name).unwrap_err(),
            format!("no `def {name}` outside another function's body in the source")
        );
    }
    let only_nested = "def outer():\n    def inner():\n        pass\n    return inner\n";
    assert!(
        extract_python_def(only_nested, "inner")
            .unwrap_err()
            .starts_with("no `def inner`")
    );
}

/// Provenance: PLAT-1035. Two definitions matching one symbol are refused,
/// never resolved to whichever came first.
#[test]
fn tc_1035_an_ambiguous_symbol_is_refused() {
    assert_eq!(
        extract_python_def(MODULE, "save").unwrap_err(),
        "`def save` is ambiguous: 2 definitions in the source"
    );
    let twice = "def run():\n    return 1\n\n\ndef run():\n    return 2\n";
    assert_eq!(
        extract_python_def(twice, "run").unwrap_err(),
        "`def run` is ambiguous: 2 definitions in the source"
    );
}

/// Provenance: PLAT-1035. The language is the file's extension: `.rs` reads
/// Rust, `.py` reads Python, and anything else is refused with a reason.
#[test]
fn tc_1035_the_language_is_chosen_by_extension() {
    assert_eq!(Language::of_path("src/lib.rs"), Some(Language::Rust));
    assert_eq!(Language::of_path("tests/test_x.py"), Some(Language::Python));
    assert_eq!(Language::of_path("main.go"), None);
    assert_eq!(Language::of_path("Makefile"), None);
    assert_eq!(
        extract_item("pkg/mod.py", MODULE, "fetch").unwrap(),
        "async def fetch(url):\n    data = await get(url)\n    return data"
    );
    assert_eq!(
        extract_item("src/lib.rs", "fn fetch() { 1 }\n", "fetch").unwrap(),
        "fn fetch() { 1 }"
    );
    assert_eq!(
        extract_item("main.go", "func fetch() {}\n", "fetch").unwrap_err(),
        "main.go: the harness extracts items from .rs and .py files only"
    );
}

// ---------------------------------------------------------------------------
// Splitting
// ---------------------------------------------------------------------------

/// Provenance: PLAT-1035. Two or more `def`s not nested in a `def` split into
/// one function unit each, methods labelled with their class; the helper
/// nested in `load` is not a unit of its own.
#[test]
fn tc_1035_several_defs_split_into_function_units() {
    let units = split_python_units(MODULE);
    let labels: Vec<(&str, UnitKind)> = units.iter().map(|u| (u.label.as_str(), u.kind)).collect();
    assert_eq!(
        labels,
        [
            ("def load", UnitKind::Function),
            ("def fetch", UnitKind::Function),
            ("def Store.helper", UnitKind::Function),
            ("def Store.save", UnitKind::Function),
            ("def Cache.save", UnitKind::Function),
        ]
    );
    assert_eq!(units[0].text, LOAD);
}

/// Provenance: PLAT-1035 (review finding 3). One `def` splits like one Rust
/// `fn`: every clause of its top-level `if` / `elif` / `else` is a branch
/// unit; the nested helper and the docstring are not units.
#[test]
fn tc_1035_an_if_chain_splits_into_branch_units() {
    let units = split_python_units(LOAD);
    let labels: Vec<(&str, UnitKind)> = units.iter().map(|u| (u.label.as_str(), u.kind)).collect();
    assert_eq!(
        labels,
        [
            ("if path", UnitKind::Branch),
            ("elif path is None", UnitKind::Branch),
            ("else", UnitKind::Branch),
        ]
    );
    assert_eq!(units[0].text, "if path:\n        return helper(1)");
    assert_eq!(units[2].text, "else:\n        return 0");
}

const TRY_AND_MATCH: &str = r#"def run(items, mode):
    total = 0
    try:
        check(total)
    except (ValueError, KeyError) as error:
        return None
    else:
        total += 1
    finally:
        log(total)
    match mode:
        case "a" | "b":
            return 1
        case {"k": value}:
            return value
        case _:
            return 0
"#;

/// Provenance: PLAT-1035 (review finding 3). `try` / `except` / `else` /
/// `finally` clauses and `match` cases are branch units, as Rust `match`
/// arms are; a `:` inside a pattern's brackets does not end its label.
#[test]
fn tc_1035_try_clauses_and_match_cases_split_into_branch_units() {
    let units = split_python_units(TRY_AND_MATCH);
    let labels: Vec<&str> = units.iter().map(|u| u.label.as_str()).collect();
    assert_eq!(
        labels,
        [
            "try",
            "except (ValueError, KeyError) as error",
            "else",
            "finally",
            "case \"a\" | \"b\"",
            "case {\"k\": value}",
            "case _",
        ]
    );
    assert!(units.iter().all(|unit| unit.kind == UnitKind::Branch));
    assert_eq!(units[3].text, "finally:\n        log(total)");
    assert_eq!(units[6].text, "case _:\n            return 0");
}

const COMMENTED: &str = "def pick(x):\n    if x < 0:\n        return \"-\"\n    \
                         # zero is its own case\n    elif x == 0:\n        return \"0\"\n    \
                         else:\n        # positive\n        return \"+\"\n";

/// Provenance: PLAT-1035 (review finding 6). A comment between two clauses
/// belongs to the clause after it; a comment inside a clause stays in it.
/// Every line from the first clause to the last lands in exactly one unit.
#[test]
fn tc_1035_a_comment_between_clauses_is_kept() {
    let units = split_python_units(COMMENTED);
    let texts: Vec<&str> = units.iter().map(|u| u.text.as_str()).collect();
    assert_eq!(
        texts,
        [
            "if x < 0:\n        return \"-\"",
            "# zero is its own case\n    elif x == 0:\n        return \"0\"",
            "else:\n        # positive\n        return \"+\"",
        ]
    );
}

/// Provenance: PLAT-1035 (review finding 3). Equivalent Rust and Python
/// shapes produce the same unit kinds and counts, so a per-unit variant is
/// not skewed by language: two methods, a three-way `if` chain, a
/// three-way `match`, and a plain body.
#[test]
fn tc_1035_equivalent_rust_and_python_shapes_split_alike() {
    let pairs = [
        (
            "impl A {\n    fn one(&self) -> u8 { 1 }\n    fn two(&self) -> u8 { 2 }\n}",
            "class A:\n    def one(self):\n        return 1\n\n    def two(self):\n        return 2\n",
        ),
        (
            "fn f(x: i32) -> char {\n    if x < 0 { '-' } else if x == 0 { '0' } else { '+' }\n}",
            "def f(x):\n    if x < 0:\n        return '-'\n    elif x == 0:\n        \
             return '0'\n    else:\n        return '+'\n",
        ),
        (
            "fn g(x: u8) -> u8 {\n    match x {\n        0 => 1,\n        1 => 2,\n        _ => 3,\n    }\n}",
            "def g(x):\n    match x:\n        case 0:\n            return 1\n        \
             case 1:\n            return 2\n        case _:\n            return 3\n",
        ),
        (
            "fn h() -> u8 {\n    let a = 1;\n    a + 1\n}",
            "def h():\n    a = 1\n    return a + 1\n",
        ),
    ];
    for (rust, python) in pairs {
        let shape = |units: Vec<eval_v2_support::units::Unit>| -> Vec<UnitKind> {
            units.into_iter().map(|u| u.kind).collect()
        };
        let rust_shape = shape(split_units("src/a.rs", rust));
        assert_eq!(rust_shape, shape(split_units("a.py", python)), "{python}");
        assert!(!rust_shape.is_empty());
    }
}

/// Provenance: PLAT-1035. A body with nothing to split is one whole unit, and
/// a body in a language the harness does not read is one whole unit too.
#[test]
fn tc_1035_a_plain_body_and_an_unknown_language_are_whole() {
    let plain = "def one():\n    \"\"\"Doc.\"\"\"\n    x = 1\n    return x\n";
    let units = split_python_units(plain);
    assert_eq!(units.len(), 1);
    assert_eq!(units[0].kind, UnitKind::Whole);
    assert_eq!(units[0].text, plain);

    let go = "func f() {\n\tif a { b() } else { c() }\n}";
    let units = split_units("main.go", go);
    assert_eq!(units.len(), 1);
    assert_eq!(units[0].kind, UnitKind::Whole);
}

// ---------------------------------------------------------------------------
// Lexical edge cases
// ---------------------------------------------------------------------------

/// Provenance: PLAT-1035 (review finding 2). A leading UTF-8 byte-order mark
/// does not hide the first line: the decorator on it stays attached.
#[test]
fn tc_1035_a_byte_order_mark_does_not_drop_the_first_decorator() {
    let source = "\u{feff}@dec\ndef f():\n    return 1\n";
    assert_eq!(
        extract_python_def(source, "f").unwrap(),
        "@dec\ndef f():\n    return 1"
    );
}

/// Provenance: PLAT-1035 (review finding 8). CRLF line endings: the body is
/// kept verbatim and the trailing `\r` is dropped.
#[test]
fn tc_1035_crlf_sources_extract() {
    let source = "@dec\r\ndef f():\r\n    x = 1\r\n    return x\r\n\r\ndef g():\r\n    pass\r\n";
    assert_eq!(
        extract_python_def(source, "f").unwrap(),
        "@dec\r\ndef f():\r\n    x = 1\r\n    return x"
    );
}

/// Provenance: PLAT-1035 (review finding 8). A tab indents to the next
/// multiple of 8, so a tab and eight spaces are the same level.
#[test]
fn tc_1035_tab_indentation_is_read() {
    let source = "class A:\n\tdef m(self):\n\t\treturn 1\n        \n\tdef n(self):\n        \
                  \treturn 2\n\ndef m():\n\treturn 3\n";
    assert_eq!(
        extract_python_def(source, "A.m").unwrap(),
        "\tdef m(self):\n\t\treturn 1"
    );
    assert_eq!(
        extract_python_def(source, "A.n").unwrap(),
        "\tdef n(self):\n        \treturn 2"
    );
    assert_eq!(
        extract_python_def(source, "m").unwrap_err(),
        "`def m` is ambiguous: 2 definitions in the source"
    );
}

/// Provenance: PLAT-1035 (review finding 8). A `\` continuation and an open
/// bracket both carry a statement onto a column-0 line without ending the
/// function.
#[test]
fn tc_1035_continuation_lines_stay_in_the_body() {
    let source =
        "def f(a, b):\n    total = a + \\\nb\n    return (total,\n0)\n\n\ndef g():\n    pass\n";
    assert_eq!(
        extract_python_def(source, "f").unwrap(),
        "def f(a, b):\n    total = a + \\\nb\n    return (total,\n0)"
    );
}

const PARAMETRIZED: &str = r##"@pytest.mark.parametrize(
    "text, expected",
    [("a#b", "x:y"), ("#(", ":")],
)
def test_split(text, expected):
    assert split(text) == expected


def after():
    pass
"##;

/// Provenance: PLAT-1035 (review finding 8). A `#`, `:` or bracket inside a
/// decorator's strings is string content, not a comment or structure.
#[test]
fn tc_1035_a_parametrize_decorator_with_hash_and_colon_strings() {
    assert_eq!(
        extract_python_def(PARAMETRIZED, "test_split").unwrap(),
        PARAMETRIZED.split("\n\n\n").next().unwrap()
    );
}

// ---------------------------------------------------------------------------
// End to end: an external `.py` row
// ---------------------------------------------------------------------------

const TEST_FILE: &str = r#"import pytest


class TestStore:
    @pytest.mark.parametrize(
        "item",
        [1, 2],
    )
    def test_save(self, item):
        store = Store()
        store.save(item)
        assert store.items == [item]
"#;

const CODE_FILE: &str = r#"class Store:
    def save(self, item):
        if item is None:
            raise ValueError("item")
        else:
            self.items.append(item)


class Cache:
    def save(self, item):
        pass
"#;

fn sha(text: &str) -> String {
    quoin_store::digest_bytes_sha256(text.as_bytes()).to_stored()
}

/// An `RTC` external row reading `TEST_FILE` and `CODE_FILE` from
/// `sibling`, with `symbol` as its code symbol.
fn python_row(id: &str, symbol: &str) -> Value {
    json!({
        "id": id,
        "mode": Mode::ReqTestCode.as_str(),
        "split": "dev",
        "strata": {"fr_id": "FR-001", "req_kind": "functional", "test_kind": null,
                   "crate": "sibling", "ears_pattern": null},
        "requirement": {"fr_id": "FR-001", "ac_id": "FR-001-AC-1",
                        "statement": "The store shall keep every saved item.",
                        "ac_text": "A saved item is in the store's items.",
                        "context": null},
        "test": {"path": "tests/test_store.py", "fn_name": "TestStore.test_save"},
        "code": {"path": "src/store.py", "symbol": symbol},
        "ref": {"repo": "agent-ix/sibling", "commit": "deadbeef",
                "paths": {"test": "tests/test_store.py", "code": "src/store.py"},
                "sha256": {"test": sha(TEST_FILE), "code": sha(CODE_FILE)}},
        "mutation": null,
        "truth": {"code_implements_intent": {"answer": true, "kind": "mechanical",
                                             "alternatives": [], "rationale": "stated"}},
    })
}

fn unit_questions(_unit: &eval_v2_support::units::Unit) -> Questions {
    questions([("in_scope", noul("Is this unit about the requirement?"))])
}

/// Provenance: PLAT-1035. An external row whose test and code are Python
/// materializes from its checkout; its code splits into branch units for
/// per-unit asks; every registered variant's wording rule holds on it as on
/// a Rust row; and an ambiguous symbol excludes the row with its reason.
#[test]
fn tc_1035_an_external_python_row_materializes_and_splits() {
    let root = tempfile::tempdir().unwrap();
    let checkout = root.path().join("sibling");
    std::fs::create_dir_all(checkout.join("tests")).unwrap();
    std::fs::create_dir_all(checkout.join("src")).unwrap();
    std::fs::write(checkout.join("tests/test_store.py"), TEST_FILE).unwrap();
    std::fs::write(checkout.join("src/store.py"), CODE_FILE).unwrap();
    let corpus_path = root.path().join("external.json");
    let text = serde_json::to_string_pretty(&json!({
        "schema": corpus::SCHEMA,
        "sampling_rule": "every row, synthetic",
        "seed": 7,
        "source_commit": "0000000",
        "rows": [python_row("EVX-0001", "Store.save"), python_row("EVX-0002", "save")],
    }))
    .unwrap();
    std::fs::write(&corpus_path, text).unwrap();

    let source = load_external(Some(&corpus_path), Some(root.path()))
        .unwrap()
        .unwrap();
    assert_eq!(source.excluded.len(), 1, "{:?}", source.excluded);
    assert_eq!(source.excluded[0].id, "EVX-0002");
    assert_eq!(
        source.excluded[0].reason,
        "`def save` is ambiguous: 2 definitions in the source"
    );
    assert_eq!(
        validate(&source.file, Origin::External),
        Vec::<String>::new()
    );

    let row = &source.file.rows[0];
    assert_eq!(
        row.test.as_ref().unwrap().body,
        "    @pytest.mark.parametrize(\n        \"item\",\n        [1, 2],\n    )\n    \
         def test_save(self, item):\n        store = Store()\n        store.save(item)\n        \
         assert store.items == [item]"
    );
    assert_eq!(
        row.code.as_ref().unwrap().body,
        "    def save(self, item):\n        if item is None:\n            \
         raise ValueError(\"item\")\n        else:\n            self.items.append(item)"
    );

    let labels: Vec<String> = code_units(row).into_iter().map(|u| u.label).collect();
    assert_eq!(labels, ["if item is None", "else"]);
    let asks = per_unit_asks(row, unit_questions);
    assert_eq!(asks.len(), 2);
    let state = serde_json::to_value(&asks[1].request.state).unwrap();
    assert_eq!(
        state.pointer("/code_unit").and_then(Value::as_str),
        Some("else")
    );

    for variant in REGISTRY.iter().filter(|variant| variant.applies_to(row)) {
        assert_eq!(wording_violations(variant, row), Vec::<String>::new());
    }
}

// ---------------------------------------------------------------------------
// The per-FR cap is per repo
// ---------------------------------------------------------------------------

/// A natural `R` row citing `FR-008` in `repo`, by reference.
fn fr_008_row(id: &str, repo: &str) -> Value {
    json!({
        "id": id,
        "mode": Mode::Req.as_str(),
        "split": "dev",
        "strata": {"fr_id": "FR-008", "req_kind": "functional", "test_kind": null,
                   "crate": repo, "ears_pattern": null},
        "requirement": {"fr_id": "FR-008", "ac_id": null,
                        "statement": "The system shall do one thing.",
                        "ac_text": null, "context": null},
        "test": null,
        "code": null,
        "ref": {"repo": repo, "commit": "c", "paths": {}, "sha256": {}},
        "mutation": null,
        "truth": {"criterion_sound": {"answer": true, "kind": "agent_dual",
                                      "alternatives": [], "rationale": "stated"}},
    })
}

fn external_file(rows: &[Value]) -> corpus::CorpusFile {
    corpus::parse(
        &json!({
            "schema": corpus::SCHEMA,
            "sampling_rule": "every row, synthetic",
            "seed": 7,
            "source_commit": "0000000",
            "rows": rows,
        })
        .to_string(),
    )
    .unwrap()
}

/// Provenance: PLAT-1035 (coordinator fix), PLAT-1024 rule 1. FR ids are per
/// repo: three natural rows of FR-008 in each of two repos is within the
/// cap, and a fourth in one repo is not.
#[test]
fn tc_1035_the_per_fr_cap_counts_each_repo_separately() {
    let mut rows: Vec<Value> = (1..=3)
        .map(|n| fr_008_row(&format!("EVX-000{n}"), "agent-ix/quire-rs"))
        .chain(
            (4..=6).map(|n| fr_008_row(&format!("EVX-000{n}"), "agent-ix/engineering-assurance")),
        )
        .collect();
    assert_eq!(
        validate(&external_file(&rows), Origin::External),
        Vec::<String>::new()
    );

    rows.push(fr_008_row("EVX-0007", "agent-ix/quire-rs"));
    assert_eq!(
        validate(&external_file(&rows), Origin::External),
        [
            "FR-008 in quire-rs: 4 natural rows (EVX-0001, EVX-0002, EVX-0003, EVX-0007), \
          at most 3 per FR per repo"
        ]
    );
}

/// Provenance: PLAT-1035 (review finding 4). The cap is keyed on the
/// checkout name, `ref.repo`'s last segment, so `agent-ix/quire-rs` and
/// `quire-rs` (the same checkout) count together.
#[test]
fn tc_1035_the_per_fr_cap_keys_on_the_checkout_name() {
    let rows = [
        fr_008_row("EVX-0001", "agent-ix/quire-rs"),
        fr_008_row("EVX-0002", "agent-ix/quire-rs"),
        fr_008_row("EVX-0003", "quire-rs"),
        fr_008_row("EVX-0004", "quire-rs"),
    ];
    assert_eq!(
        validate(&external_file(&rows), Origin::External),
        [
            "FR-008 in quire-rs: 4 natural rows (EVX-0001, EVX-0002, EVX-0003, EVX-0004), \
          at most 3 per FR per repo"
        ]
    );
}
