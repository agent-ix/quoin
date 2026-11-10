// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The semantic-module cookiecutter's refusal and absent-tool paths, driven
//! (PLAT-194, FR-077, FR-080, NFR-020).
//!
//! Each test renders a variant into its own temporary directory, installs the
//! rendered repository's npm toolchain there from the public registry, mutates
//! that tree, and runs the rendered emit command or gate against it. Nothing is
//! written inside the working tree, and dropping the `Render` removes the
//! directory even when an assertion fails. The developer's own installed
//! packages are never touched: every `node_modules` lives in the temp tree.
//!
//! Every refusal is asserted by the value it names, not by exit status alone.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]

mod common;

use ix_trace_rs::trace;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use common::template::{IMPORT, Render, render};
use regex::Regex;

/// Render `kind`, then install its npm toolchain inside the rendered tree.
/// A failed install fails the test naming npm's own message; it never skips.
fn installed(kind: &str, imports: &str) -> Render {
    let r = render(kind, imports, &[]);
    let out = Command::new("npm")
        .args(["install", "--no-audit", "--no-fund"])
        .current_dir(&r.root)
        .env("npm_config_userconfig", "/dev/null")
        .env("npm_config_registry", "https://registry.npmjs.org")
        .output()
        .expect("`npm` is not installed; install Node 20 or later");
    assert!(
        out.status.success(),
        "variant {kind}: `npm install` failed in the rendered tree: {}",
        text(&out)
    );
    r
}

/// The rendered `poe schemas` / `poe schemas-check` task is `npx --no-install
/// semantic-module-emit ...`; this runs the same installed bin directly.
fn emit(r: &Render, check: bool) -> Output {
    let mut command = Command::new(r.root.join("node_modules/.bin/semantic-module-emit"));
    command
        .args(["--package-dir", &r.package])
        .current_dir(&r.root);
    if check {
        command.arg("--check");
    }
    command.output().unwrap()
}

fn emit_ok(r: &Render) {
    let out = emit(r, false);
    assert!(out.status.success(), "baseline emit failed: {}", text(&out));
}

fn text(out: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    )
}

fn files_under(root: &Path) -> BTreeMap<String, Vec<u8>> {
    fn walk(base: &Path, dir: &Path, out: &mut BTreeMap<String, Vec<u8>>) {
        for entry in fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                walk(base, &path, out);
            } else {
                let rel = path.strip_prefix(base).unwrap();
                out.insert(rel.to_string_lossy().into_owned(), fs::read(&path).unwrap());
            }
        }
    }
    let mut out = BTreeMap::new();
    walk(root, root, &mut out);
    out
}

/// Every committed schema, the manifest and `package.json`: the bytes a failed
/// emission must leave exactly as they were.
fn committed(r: &Render) -> BTreeMap<String, Vec<u8>> {
    let mut out = files_under(&r.path("{package}/schemas"));
    for file in ["{package}/manifest.yaml", "package.json"] {
        out.insert(file.to_owned(), fs::read(r.path(file)).unwrap());
    }
    out
}

fn append(path: &Path, tail: &str) {
    fs::OpenOptions::new()
        .append(true)
        .open(path)
        .unwrap()
        .write_all(tail.as_bytes())
        .unwrap();
}

fn main_tsp(r: &Render) -> PathBuf {
    r.path("typespec/main.tsp")
}

// ---------------------------------------------------------------- FR-077

#[trace("FR-077-AC-4", "TC-1414")]
#[trace("FR-077-CON-1")]
#[test]
fn tc_194_001_check_mode_is_red_naming_the_file_after_one_emitted_byte_changes() {
    let r = installed("object", "");
    emit_ok(&r);
    let green = emit(&r, true);
    assert!(
        green.status.success(),
        "check on clean output: {}",
        text(&green)
    );

    let schema = r.path("{package}/schemas/Element.json");
    let mut bytes = fs::read(&schema).unwrap();
    let at = bytes.windows(6).position(|w| w == b"object").unwrap();
    bytes[at] = b'0';
    fs::write(&schema, &bytes).unwrap();
    let before = committed(&r);

    let red = emit(&r, true);
    let message = text(&red);
    assert!(
        !red.status.success(),
        "one changed byte must fail: {message}"
    );
    assert!(
        message.contains("spec_objects_example/schemas/Element.json"),
        "the changed file is not named: {message}"
    );
    assert!(
        !message.contains("Note.json") && !message.contains("IdentityField.json"),
        "an untouched file is named: {message}"
    );
    assert_eq!(before, committed(&r), "check mode wrote a file");
}

#[trace("FR-077-AC-5", "TC-1415")]
#[test]
fn tc_194_002_a_base_that_disagrees_with_the_package_identity_fails_naming_both() {
    let r = installed("object", "");
    emit_ok(&r);
    let source = fs::read_to_string(main_tsp(&r)).unwrap();
    let declared = "https://schemas.agent-ix.org/agent-ix/spec-objects-exampl3/";
    let expected = "https://schemas.agent-ix.org/agent-ix/spec-objects-example/";
    assert!(source.contains(expected), "rendered base changed shape");
    fs::write(main_tsp(&r), source.replace(expected, declared)).unwrap();
    let before = committed(&r);

    let out = emit(&r, false);
    let message = text(&out);
    assert!(
        !out.status.success(),
        "a disagreeing base must fail: {message}"
    );
    assert!(
        message.contains(declared),
        "declared base not named: {message}"
    );
    assert!(
        message.contains(expected),
        "expected base not named: {message}"
    );
    assert_eq!(
        before,
        committed(&r),
        "a refused emission changed a committed byte"
    );
}

/// Models a source declares, by name.
fn declared_models(tsp: &str) -> BTreeSet<String> {
    let model = Regex::new(r"(?m)^\s*model\s+([A-Za-z_][A-Za-z0-9_]*)").unwrap();
    model.captures_iter(tsp).map(|c| c[1].to_owned()).collect()
}

/// Models the installed semantic-core grammar already declares: one emitted
/// schema per model.
fn grammar_models(r: &Render) -> BTreeSet<String> {
    let dir = r
        .root
        .join("node_modules/@agent-ix/semantic-core/generated/json-schema");
    fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("{}: {e}", dir.display()))
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|x| x == "json"))
        .map(|p| p.file_stem().unwrap().to_string_lossy().into_owned())
        .collect()
}

#[trace("FR-077-AC-6", "TC-1416")]
#[test]
fn tc_194_003_main_tsp_imports_the_grammar_and_redeclares_none_of_it() {
    for (kind, imports) in [("object", ""), ("artifact", ""), ("mixed", IMPORT)] {
        let r = installed(kind, imports);
        let source = fs::read_to_string(main_tsp(&r)).unwrap();
        let grammar = grammar_models(&r);
        assert!(
            grammar.contains("FieldDecl"),
            "the grammar list is empty or wrong: {grammar:?}"
        );
        assert!(
            source.contains("import \"@agent-ix/semantic-core\";"),
            "variant {kind}: main.tsp does not import the grammar"
        );
        let redeclared: Vec<_> = declared_models(&source)
            .intersection(&grammar)
            .cloned()
            .collect();
        assert!(
            redeclared.is_empty(),
            "variant {kind}: main.tsp redeclares {redeclared:?}"
        );

        // The check can fail: a grammar model written into the source is seen.
        let injected = format!("{source}\nmodel FieldDecl {{}}\n");
        let seen: Vec<_> = declared_models(&injected)
            .intersection(&grammar)
            .cloned()
            .collect();
        assert_eq!(
            seen,
            ["FieldDecl"],
            "variant {kind}: the redeclaration check cannot fire"
        );
    }
}

#[trace("FR-077-AC-7", "TC-1449")]
#[test]
fn tc_194_004_a_broken_main_tsp_fails_with_the_compiler_diagnostic_and_changes_nothing() {
    let r = installed("object", "");
    emit_ok(&r);
    append(&main_tsp(&r), "\nmodel Broken {\n");
    let before = committed(&r);

    for check in [false, true] {
        let out = emit(&r, check);
        let message = text(&out);
        assert!(
            !out.status.success(),
            "a TypeSpec error must fail: {message}"
        );
        assert!(
            message.contains("tsp compile failed"),
            "no compile failure named: {message}"
        );
        assert!(
            message.contains("main.tsp"),
            "the broken source is not named: {message}"
        );
        assert!(
            message.contains("error"),
            "no compiler diagnostic carried: {message}"
        );
        assert_eq!(
            before,
            committed(&r),
            "a failed compile changed a committed byte"
        );
    }
}

#[trace("FR-077-AC-8", "TC-1457")]
#[test]
fn tc_194_005_a_ref_resolves_to_an_imported_base_or_fails_naming_it() {
    let r = installed("mixed", IMPORT);
    emit_ok(&r);

    // A property whose `$ref` names the imported module.
    let probe = |target: &str| {
        format!("\nmodel Probe {{\n  @extension(\"$ref\", \"{target}\")\n  other: string;\n}}\n")
    };
    append(
        &main_tsp(&r),
        &probe("agent-ix/spec-artifacts-iso/Thing.json"),
    );
    emit_ok(&r);
    let written = r.text("{package}/schemas/Probe.json");
    assert!(
        written.contains("\"https://schemas.agent-ix.org/agent-ix/spec-artifacts-iso/Thing.json\""),
        "the imported ref was not written against the imported base: {written}"
    );

    // A `$ref` matching no declared base.
    let source = fs::read_to_string(main_tsp(&r)).unwrap();
    fs::write(
        main_tsp(&r),
        source.replace(
            "agent-ix/spec-artifacts-iso/Thing.json",
            "nobody/knows/Thing.json",
        ),
    )
    .unwrap();
    let before = committed(&r);
    let out = emit(&r, false);
    let message = text(&out);
    assert!(
        !out.status.success(),
        "an unresolvable ref must fail: {message}"
    );
    assert!(
        message.contains("nobody/knows/Thing.json"),
        "the ref is not named: {message}"
    );
    assert!(
        message.contains("matches no declared base"),
        "wrong refusal: {message}"
    );
    assert_eq!(
        before,
        committed(&r),
        "a refused emission changed a committed byte"
    );
}

// ------------------------------------------------------------- NFR-020

#[trace("FR-083-AC-6", "TC-1448")]
#[trace("NFR-020-AC-3")]
#[test]
fn tc_194_006_an_absent_schema_toolchain_fails_the_drift_check_naming_the_install_command() {
    let r = installed("object", "");
    emit_ok(&r);
    fs::remove_dir_all(r.root.join("node_modules/@typespec/compiler")).unwrap();
    let before = committed(&r);

    for check in [false, true] {
        let out = emit(&r, check);
        let message = text(&out);
        assert!(
            !out.status.success(),
            "an absent toolchain must fail: {message}"
        );
        assert!(
            message.contains("@typespec/compiler"),
            "toolchain not named: {message}"
        );
        assert!(
            message.contains("make install"),
            "install command not named: {message}"
        );
        assert!(
            !message.contains("match the committed output"),
            "the drift check reported success without compiling: {message}"
        );
        assert_eq!(
            before,
            committed(&r),
            "an absent toolchain changed a committed byte"
        );
    }
}

fn find_on_path(name: &str) -> PathBuf {
    std::env::split_paths(&std::env::var_os("PATH").unwrap_or_default())
        .map(|dir| dir.join(name))
        .find(|candidate| candidate.is_file())
        .unwrap_or_else(|| panic!("`{name}` is not on PATH; install it"))
}

#[trace("NFR-020-AC-4", "TC-1465")]
#[test]
fn tc_194_007_an_absent_validator_fails_the_validation_leg_naming_quire() {
    let r = render("object", "", &[]);
    let make = find_on_path("make");
    // A PATH holding nothing at all: `quire` cannot resolve.
    let empty = r.root.join("empty-path");
    fs::create_dir(&empty).unwrap();

    for target in ["validate", "gate"] {
        let out = Command::new(&make)
            .arg(target)
            .current_dir(&r.root)
            .env("PATH", &empty)
            .output()
            .unwrap();
        let message = text(&out);
        assert!(
            !out.status.success(),
            "make {target} passed with no validator: {message}"
        );
        assert!(
            message.contains("quire"),
            "make {target} does not name the validator: {message}"
        );
        assert!(message.contains("not found"), "make {target}: {message}");
        assert!(
            !message.contains("0 document"),
            "make {target} reported zero documents instead of failing: {message}"
        );
    }
}

#[trace("FR-080-AC-5")]
#[test]
fn tc_194_008_a_failing_validation_leg_fails_the_gate_before_lint_and_test_run() {
    let r = render("object", "", &[]);
    let make = find_on_path("make");
    let run = |target: &str| {
        Command::new(&make)
            .arg(target)
            .current_dir(&r.root)
            .output()
            .unwrap()
    };
    let green = run("validate");
    assert!(
        green.status.success(),
        "clean spec must validate: {}",
        text(&green)
    );

    let spec = r.path("spec/spec.md");
    let broken = r
        .text("spec/spec.md")
        .replacen("type: master-requirements\n", "", 1);
    fs::write(&spec, broken).unwrap();
    let out = run("gate");
    let message = text(&out);
    assert!(
        !out.status.success(),
        "make gate passed with an invalid spec: {message}"
    );
    assert!(
        message.contains("spec.md"),
        "the invalid document is not named: {message}"
    );
    assert!(
        !message.contains("ruff") && !message.contains("pytest"),
        "a later leg ran after the validation leg failed: {message}"
    );
}
