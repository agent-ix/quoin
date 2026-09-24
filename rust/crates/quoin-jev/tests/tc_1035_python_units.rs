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

const RUN: &str = r#"def run(items):
    """Doc."""
    total = 0
    for item in items:
        total += item
    else:
        total -= 1
    try:
        check(total)
    except ValueError:
        return None
    finally:
        log(total)
    @wrap
    def inner():
        return total
    return (total,
            inner)
"#;

/// Provenance: PLAT-1035. One `def` splits into its body's top-level
/// statements: a compound statement keeps its clauses, a decorator stays with
/// its `def`, a bracketed statement spans its lines, the docstring is not a
/// unit, and an `if` chain is one statement.
#[test]
fn tc_1035_one_def_splits_into_statement_units() {
    let units = split_python_units(RUN);
    let labels: Vec<&str> = units.iter().map(|u| u.label.as_str()).collect();
    assert_eq!(
        labels,
        [
            "statement total = 0",
            "statement for item in items:",
            "statement try:",
            "statement @wrap",
            "statement return (total,",
        ]
    );
    assert!(units.iter().all(|unit| unit.kind == UnitKind::Statement));
    assert_eq!(
        units[1].text,
        "for item in items:\n        total += item\n    else:\n        total -= 1"
    );
    assert_eq!(
        units[2].text,
        "try:\n        check(total)\n    except ValueError:\n        return None\n    \
         finally:\n        log(total)"
    );
    assert_eq!(
        units[3].text,
        "@wrap\n    def inner():\n        return total"
    );
    assert_eq!(units[4].text, "return (total,\n            inner)");

    let load = split_python_units(LOAD);
    let labels: Vec<&str> = load.iter().map(|u| u.label.as_str()).collect();
    assert_eq!(labels, ["statement def helper(x):", "statement if path:"]);
    assert!(load[1].text.ends_with("    else:\n        return 0"));
}

/// Provenance: PLAT-1035. A body with one statement is one whole unit, and a
/// body in a language the harness does not read is one whole unit too.
#[test]
fn tc_1035_a_plain_body_and_an_unknown_language_are_whole() {
    let plain = "def one():\n    \"\"\"Doc.\"\"\"\n    return 1\n";
    let units = split_python_units(plain);
    assert_eq!(units.len(), 1);
    assert_eq!(units[0].kind, UnitKind::Whole);
    assert_eq!(units[0].text, plain);

    let go = "func f() {\n\tif a { b() } else { c() }\n}";
    let units = split_units("main.go", go);
    assert_eq!(units.len(), 1);
    assert_eq!(units[0].kind, UnitKind::Whole);

    let rust = "fn f(x: i32) -> char {\n    if x < 0 { '-' } else { '+' }\n}";
    let labels: Vec<String> = split_units("src/f.rs", rust)
        .into_iter()
        .map(|u| u.label)
        .collect();
    assert_eq!(labels, ["if x < 0", "else"]);
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
/// materializes from its checkout; its code splits into statement units for
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
         raise ValueError(\"item\")\n        self.items.append(item)"
    );

    let labels: Vec<String> = code_units(row).into_iter().map(|u| u.label).collect();
    assert_eq!(
        labels,
        ["statement if item is None:", "statement self.items.append(item)"]
    );
    let asks = per_unit_asks(row, unit_questions);
    assert_eq!(asks.len(), 2);
    let state = serde_json::to_value(&asks[1].request.state).unwrap();
    assert_eq!(
        state.pointer("/code_unit").and_then(Value::as_str),
        Some("statement self.items.append(item)")
    );

    for variant in REGISTRY.iter().filter(|variant| variant.applies_to(row)) {
        assert_eq!(wording_violations(variant, row), Vec::<String>::new());
    }
}
