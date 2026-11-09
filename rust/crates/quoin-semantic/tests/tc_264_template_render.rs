// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The semantic-module cookiecutter, verified by instantiation (PLAT-264,
//! FR-076, FR-078, FR-081, FR-082, FR-083).
//!
//! Every test renders the real template at `templates/semantic-module` with the
//! `cookiecutter` executable into a fresh temporary directory and reads the
//! result. `cookiecutter` is a declared toolchain floor (NFR-020), so a host
//! without it fails here naming the install command; it is not skipped.
//!
//! What is not asserted here, because it needs a registry or a toolchain this
//! crate does not own: the `TypeSpec` emit run (`@agent-ix/semantic-core` and
//! `@typespec/*` from npm), the rendered Python suite (the Quire engine from the
//! internal index), wheel and tarball packing, and `quire validate` over the
//! rendered `spec/` tree. Those are FR-077, FR-080, FR-081-AC-3 and FR-082-AC-1.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]

mod common;

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::{Path, PathBuf};
use std::process::Command;

use common::{schema_dir, validators};
use regex::Regex;
use serde_json::Value;
use tempfile::TempDir;

const TEMPLATE: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../../templates/semantic-module"
);

/// An import a `mixed` variant needs; the form is `<org>/<repo>@<requirement>`.
const IMPORT: &str = "agent-ix/spec-artifacts-iso@0.1.0";

const VARIANTS: [(&str, &str); 3] = [("object", ""), ("artifact", ""), ("mixed", IMPORT)];

/// One rendered repository, living only as long as this value.
struct Render {
    _dir: TempDir,
    root: PathBuf,
    package: String,
}

impl Render {
    fn path(&self, relative: &str) -> PathBuf {
        self.root.join(relative.replace("{package}", &self.package))
    }

    fn text(&self, relative: &str) -> String {
        let path = self.path(relative);
        fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
    }

    fn manifest(&self) -> Value {
        quoin_yaml::from_str(&self.text("{package}/manifest.yaml")).unwrap()
    }
}

/// `cookiecutter` 1.x silently replaces an off-list choice with the first one,
/// so FR-076-AC-6 cannot hold there; the floor is 2.x (FR-076).
fn assert_renderer_floor() {
    static CHECKED: std::sync::Once = std::sync::Once::new();
    CHECKED.call_once(|| {
        let out = Command::new("cookiecutter").arg("--version").output().expect(
            "`cookiecutter` is not installed; install it with `pipx install cookiecutter` \
             (NFR-020 declares it a toolchain floor, so this is a failure, not a skip)",
        );
        let text = String::from_utf8_lossy(&out.stdout);
        let major: u32 = text
            .split_whitespace()
            .nth(1)
            .and_then(|v| v.split('.').next())
            .and_then(|m| m.parse().ok())
            .unwrap_or_else(|| panic!("unreadable `cookiecutter --version`: {text}"));
        assert!(
            major >= 2,
            "cookiecutter {major}.x is below the 2.x floor (FR-076); upgrade with `pipx upgrade cookiecutter`"
        );
    });
}

fn cookiecutter(output: &Path, args: &[&str]) -> std::process::Output {
    assert_renderer_floor();
    Command::new("cookiecutter")
        .args(["--no-input", "--output-dir"])
        .arg(output)
        .arg(TEMPLATE)
        .args(args)
        .stdin(std::process::Stdio::null())
        .output()
        .expect(
            "`cookiecutter` is not installed; install it with `pipx install cookiecutter` \
             (NFR-020 declares it a toolchain floor, so this is a failure, not a skip)",
        )
}

/// Render `kind` with extra `key=value` inputs; panics if rendering fails.
fn render(kind: &str, imports: &str, extra: &[&str]) -> Render {
    let dir = TempDir::new().unwrap();
    let mut args = vec![format!("module_kind={kind}")];
    if !imports.is_empty() {
        args.push(format!("imported_modules={imports}"));
    }
    args.extend(extra.iter().map(|s| (*s).to_owned()));
    let refs: Vec<&str> = args.iter().map(String::as_str).collect();
    let out = cookiecutter(dir.path(), &refs);
    assert!(
        out.status.success(),
        "variant {kind}: rendering failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let root = dir.path().join("spec-objects-example");
    Render {
        _dir: dir,
        root,
        package: "spec_objects_example".to_owned(),
    }
}

fn all_variants() -> Vec<(&'static str, Render)> {
    VARIANTS
        .iter()
        .map(|(kind, imports)| (*kind, render(kind, imports, &[])))
        .collect()
}

/// Render expecting refusal: returns stderr, and asserts nothing was left.
fn refused(args: &[&str]) -> String {
    let dir = TempDir::new().unwrap();
    let out = cookiecutter(dir.path(), args);
    assert!(!out.status.success(), "{args:?} must abort rendering");
    let left: Vec<_> = fs::read_dir(dir.path())
        .unwrap()
        .map(|e| e.unwrap().file_name())
        .collect();
    assert!(left.is_empty(), "a refused rendering left {left:?} behind");
    String::from_utf8_lossy(&out.stderr).into_owned()
}

fn files_under(root: &Path) -> BTreeMap<String, Vec<u8>> {
    fn walk(base: &Path, dir: &Path, out: &mut BTreeMap<String, Vec<u8>>) {
        for entry in fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                walk(base, &path, out);
            } else {
                let rel = path
                    .strip_prefix(base)
                    .unwrap()
                    .to_string_lossy()
                    .into_owned();
                out.insert(rel, fs::read(&path).unwrap());
            }
        }
    }
    let mut out = BTreeMap::new();
    walk(root, root, &mut out);
    out
}

fn type_names(manifest: &Value, section: &str) -> Vec<String> {
    manifest[section]
        .as_array()
        .unwrap_or_else(|| panic!("manifest has no {section} list"))
        .iter()
        .map(|t| t["name"].as_str().unwrap().to_owned())
        .collect()
}

// ---------------------------------------------------------------- FR-076

/// Trace: FR-076-AC-1
#[test]
fn tc_264_001_object_variant_declares_object_types_and_no_artifact_types() {
    let r = render("object", "", &[]);
    let manifest = r.manifest();
    assert_eq!(type_names(&manifest, "object_types"), ["element"]);
    assert!(type_names(&manifest, "artifact_types").is_empty());
}

/// Trace: FR-076-AC-2
#[test]
fn tc_264_002_artifact_variant_declares_artifact_types_and_no_object_types() {
    let r = render("artifact", "", &[]);
    let manifest = r.manifest();
    assert_eq!(type_names(&manifest, "artifact_types"), ["note"]);
    assert!(type_names(&manifest, "object_types").is_empty());
}

/// Trace: FR-076-AC-3
#[test]
fn tc_264_003_mixed_variant_declares_both_sections_and_an_import() {
    let r = render("mixed", IMPORT, &[]);
    let manifest = r.manifest();
    assert_eq!(type_names(&manifest, "artifact_types"), ["note"]);
    assert_eq!(type_names(&manifest, "object_types"), ["element"]);
    let imports = manifest["semantic"]["imports"].as_object().unwrap();
    assert_eq!(imports.len(), 1);
    assert_eq!(imports["agent-ix/spec-artifacts-iso"], "0.1.0");
}

/// One template core: the template has a single project directory, and every
/// file that is byte-identical across the three variants comes from exactly
/// one template file.
///
/// Trace: FR-076-AC-4
#[test]
fn tc_264_004_shared_surfaces_exist_exactly_once_in_the_template_source() {
    let template_roots: Vec<_> = fs::read_dir(TEMPLATE)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .filter(|name| name.starts_with("{{"))
        .collect();
    assert_eq!(
        template_roots.len(),
        1,
        "one project core: {template_roots:?}"
    );

    let project = Path::new(TEMPLATE).join(&template_roots[0]);
    let mut rendered_names: Vec<String> = files_under(&project)
        .into_keys()
        .map(|rel| rel.replace("{{cookiecutter.package_name}}", "spec_objects_example"))
        .collect();
    rendered_names.sort();
    let unique: BTreeSet<_> = rendered_names.iter().cloned().collect();
    assert_eq!(
        unique.len(),
        rendered_names.len(),
        "a surface is carried twice"
    );

    let variants = all_variants();
    let trees: Vec<_> = variants.iter().map(|(_, r)| files_under(&r.root)).collect();
    let shared: Vec<&String> = trees[0]
        .iter()
        .filter(|(name, bytes)| trees[1..].iter().all(|t| t.get(*name) == Some(*bytes)))
        .map(|(name, _)| name)
        .collect();
    assert!(
        shared.len() > 20,
        "the shared core is large: {}",
        shared.len()
    );
    for name in shared {
        assert!(
            unique.contains(name),
            "{name} is shared but not in the template once"
        );
    }
}

/// Trace: FR-076-AC-5
#[test]
fn tc_264_005_a_non_agpl_license_aborts_and_the_default_renders_agpl() {
    assert!(refused(&["license=MIT"]).contains("MIT"));
    let r = render("object", "", &[]);
    assert!(
        r.text("LICENSE")
            .contains("GNU AFFERO GENERAL PUBLIC LICENSE")
    );
    assert!(r.text("LICENSE").contains("Version 3, 19 November 2007"));
}

/// Trace: FR-076-AC-6
#[test]
fn tc_264_006_an_unknown_module_kind_aborts_naming_it() {
    assert!(refused(&["module_kind=hybrid"]).contains("hybrid"));
}

/// Trace: FR-076-AC-7
#[test]
fn tc_264_007_an_import_without_an_exact_version_aborts_naming_the_entry() {
    let bare = "agent-ix/spec-objects-business";
    assert!(refused(&[&format!("imported_modules={bare}")]).contains(bare));
    let zeros = "agent-ix/spec-objects-business@01.2.3";
    assert!(refused(&["module_kind=mixed", &format!("imported_modules={zeros}")]).contains(zeros));
    let pre = render("mixed", "agent-ix/spec-objects-business@1.2.3-rc.1", &[]);
    assert_eq!(
        pre.manifest()["semantic"]["imports"]["agent-ix/spec-objects-business"],
        "1.2.3-rc.1"
    );
    let range = "agent-ix/spec-objects-business@^1.2.0";
    assert!(refused(&["module_kind=mixed", &format!("imported_modules={range}")]).contains(range));
    let r = render("mixed", "agent-ix/spec-objects-business@1.2.0", &[]);
    assert_eq!(
        r.manifest()["semantic"]["imports"]["agent-ix/spec-objects-business"],
        "1.2.0"
    );
}

/// Trace: FR-076-AC-8
#[test]
fn tc_264_008_a_target_outside_the_registry_aborts_naming_it() {
    assert!(refused(&["generated_targets=json-schema,go"]).contains("go"));
}

/// The target list the pre-generation hook validates against is the
/// filament-core-data registry Quoin reads (`target` plus
/// `representationFormat`), not a stale copy of it.
///
/// Trace: FR-076-AC-8
#[test]
fn tc_264_009_the_hooks_target_registry_equals_the_published_registry() {
    let common: Value = serde_json::from_str(
        &fs::read_to_string(schema_dir().join("filament-core-data/common.schema.json")).unwrap(),
    )
    .unwrap();
    let defs = &common["$defs"];
    let published: BTreeSet<String> = ["target", "representationFormat"]
        .iter()
        .flat_map(|name| defs[*name]["enum"].as_array().unwrap().clone())
        .map(|v| v.as_str().unwrap().to_owned())
        .collect();

    let hook = fs::read_to_string(Path::new(TEMPLATE).join("hooks/pre_gen_project.py")).unwrap();
    let block = hook
        .split("TARGET_REGISTRY = {")
        .nth(1)
        .and_then(|rest| rest.split('}').next())
        .expect("the hook declares TARGET_REGISTRY");
    let in_hook: BTreeSet<String> = Regex::new(r#""([^"]+)""#)
        .unwrap()
        .captures_iter(block)
        .map(|c| c[1].to_owned())
        .collect();
    assert_eq!(in_hook, published);
}

/// Trace: FR-076-AC-9
/// Trace: FR-076-CON-2
#[test]
fn tc_264_010_no_variant_carries_an_npmrc() {
    for (kind, r) in all_variants() {
        let bad: Vec<_> = files_under(&r.root)
            .into_keys()
            .filter(|n| n.rsplit('/').next() == Some(".npmrc"))
            .collect();
        assert!(bad.is_empty(), "variant {kind} carries {bad:?}");
    }
}

/// A declared target with no emitter is recorded as declared-not-emitted in the
/// README and in `semantic.targets`, and the two agree.
///
/// Trace: FR-076-AC-10
#[test]
fn tc_264_011_declared_not_emitted_targets_agree_between_readme_and_manifest() {
    let r = render(
        "object",
        "",
        &["generated_targets=json-schema,rust,markdown"],
    );
    let targets: Vec<String> = r.manifest()["semantic"]["targets"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t.as_str().unwrap().to_owned())
        .collect();
    assert_eq!(targets, ["json-schema", "rust", "markdown"]);
    let readme = r.text("README.md");
    assert!(readme.contains("Only `json-schema` is emitted today"));
    for declared_only in ["rust", "markdown"] {
        assert!(
            readme.contains(&format!("`{declared_only}`")),
            "README must record {declared_only} as declared, not emitted"
        );
    }
}

/// Trace: FR-076-AC-11
#[test]
fn tc_264_012_every_variant_renders_unattended() {
    // `cookiecutter` runs with a null stdin and `--no-input`; reaching here for
    // each variant is the assertion.
    assert_eq!(all_variants().len(), 3);
}

/// Trace: FR-076-AC-12
#[test]
fn tc_264_013_a_mixed_module_without_imports_aborts() {
    assert!(refused(&["module_kind=mixed"]).contains("at least one import"));
}

/// Trace: FR-076-AC-13
#[test]
fn tc_264_014_every_refusal_leaves_no_directory_behind() {
    for args in [
        &["license=MIT"][..],
        &["module_kind=hybrid"],
        &["module_kind=mixed"],
        &["generated_targets=go"],
        &["imported_modules=agent-ix/x"],
    ] {
        let _ = refused(args); // asserts the output directory is empty
    }
}

// ---------------------------------------------------------------- FR-078

/// Trace: FR-078-AC-1
/// Trace: FR-078-AC-6
/// Trace: FR-078-CON-1
#[test]
fn tc_264_020_the_semantic_block_has_the_nine_keys_validates_and_matches_the_types() {
    let nine: BTreeSet<&str> = [
        "contract_version",
        "semantic_core",
        "package",
        "exports",
        "imports",
        "targets",
        "mappings",
        "compatibility_posture",
        "legacy_forms",
    ]
    .into();
    let validators = validators();
    for (kind, r) in all_variants() {
        let manifest = r.manifest();
        let block = &manifest["semantic"];
        let keys: BTreeSet<&str> = block
            .as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect();
        assert_eq!(keys, nine, "variant {kind}");
        let errors = validators.semantic_block().errors(block);
        assert!(errors.is_empty(), "variant {kind}: {errors:?}");

        let mut declared: Vec<String> = type_names(&manifest, "artifact_types");
        declared.extend(type_names(&manifest, "object_types"));
        let mut exports: Vec<String> = block["exports"]
            .as_array()
            .unwrap()
            .iter()
            .map(|e| e.as_str().unwrap().to_owned())
            .collect();
        declared.sort();
        exports.sort();
        assert_eq!(exports, declared, "variant {kind}");
    }
}

/// Trace: FR-078-AC-2
/// Trace: FR-078-AC-3
#[test]
fn tc_264_021_every_type_names_an_emitted_schema_never_an_inline_object() {
    for (kind, r) in all_variants() {
        let manifest = r.manifest();
        for section in ["artifact_types", "object_types"] {
            for entry in manifest[section].as_array().unwrap() {
                let schema = &entry["data_schema"];
                let path = schema["schema"].as_str().unwrap_or_else(|| {
                    panic!(
                        "variant {kind}: {} has no schema path: {schema}",
                        entry["name"]
                    )
                });
                assert!(
                    path.starts_with("schemas/")
                        && Path::new(path).extension().is_some_and(|e| e == "json"),
                    "{path}"
                );
                assert!(
                    schema.get("type").is_none(),
                    "variant {kind}: inline {{type: object}}"
                );
            }
        }
    }
}

/// Trace: FR-078-AC-5
#[test]
fn tc_264_022_imports_carry_the_declared_exact_versions() {
    let two = "agent-ix/spec-artifacts-iso@0.1.0,agent-ix/spec-objects-business@2.3.0";
    let r = render("mixed", two, &[]);
    let imports = r.manifest()["semantic"]["imports"]
        .as_object()
        .unwrap()
        .clone();
    assert_eq!(imports.len(), 2);
    assert_eq!(imports["agent-ix/spec-artifacts-iso"], "0.1.0");
    assert_eq!(imports["agent-ix/spec-objects-business"], "2.3.0");
    let none = render("object", "", &[]);
    assert!(
        none.manifest()["semantic"]["imports"]
            .as_object()
            .unwrap()
            .is_empty()
    );
}

/// Trace: FR-078-AC-8
#[test]
fn tc_264_023_a_duplicated_import_identity_aborts_naming_it() {
    let dup = "agent-ix/spec-artifacts-iso@0.1.0,agent-ix/spec-artifacts-iso@0.2.0";
    assert!(
        refused(&["module_kind=mixed", &format!("imported_modules={dup}")])
            .contains("agent-ix/spec-artifacts-iso")
    );
}

// ---------------------------------------------------------------- FR-081 / FR-082

/// Trace: FR-081-AC-1
#[test]
fn tc_264_030_one_spdx_identifier_across_every_declaration() {
    for (kind, r) in all_variants() {
        let pyproject = r.text("pyproject.toml");
        assert!(
            pyproject.contains("license = \"AGPL-3.0-or-later\""),
            "variant {kind}"
        );
        let package: Value = serde_json::from_str(&r.text("package.json")).unwrap();
        assert_eq!(package["license"], "AGPL-3.0-or-later", "variant {kind}");
        assert!(
            r.text("README.md").contains("**AGPL-3.0-or-later**"),
            "variant {kind}"
        );
        for (name, bytes) in files_under(&r.root) {
            let text = String::from_utf8_lossy(&bytes);
            for other in ["MIT", "Apache-2.0", "GPL-3.0-only", "BSD-3-Clause"] {
                assert!(
                    !text.contains(&format!("license = \"{other}\""))
                        && !text.contains(&format!("\"license\": \"{other}\"")),
                    "variant {kind}: {name} declares {other}"
                );
            }
        }
    }
}

/// Trace: FR-081-AC-2
#[test]
fn tc_264_031_the_public_repository_baseline_is_present() {
    for (kind, r) in all_variants() {
        for file in [
            ".github/CODEOWNERS",
            "AGENTS.md",
            "CLAUDE.md",
            "README.md",
            "CONTRIBUTING.md",
            "SECURITY.md",
            ".gitignore",
            ".gitattributes",
            "Makefile",
        ] {
            assert!(r.path(file).is_file(), "variant {kind} lacks {file}");
        }
    }
}

/// Trace: FR-081-AC-5
#[test]
fn tc_264_032_the_npm_target_is_the_public_registry() {
    let r = render("object", "", &[]);
    let package: Value = serde_json::from_str(&r.text("package.json")).unwrap();
    assert_eq!(
        package["publishConfig"]["registry"],
        "https://registry.npmjs.org/"
    );
    assert_eq!(package["publishConfig"]["access"], "public");
}

/// Trace: FR-081-AC-6
#[test]
fn tc_264_033_release_workflows_are_manual_and_delegate_publication() {
    let r = render("object", "", &[]);
    for name in ["ci.yml", "release-npm.yml"] {
        let workflow = r.text(&format!(".github/workflows/{name}"));
        let doc = quoin_yaml::from_str(&workflow).unwrap();
        let on = doc
            .get("on")
            .or_else(|| doc.get("true"))
            .expect("an `on` trigger");
        let triggers: Vec<&String> = on.as_object().unwrap().keys().collect();
        assert_eq!(
            triggers,
            ["workflow_dispatch"],
            "{name} runs only on a manual trigger"
        );
        for job in doc["jobs"].as_object().unwrap().values() {
            assert!(
                job.get("uses").is_some(),
                "{name}: a job does not delegate to a shared reusable workflow"
            );
            assert!(
                job.get("steps").is_none(),
                "{name}: a job carries its own steps"
            );
        }
    }
}

/// The rendered catalog document names the Quoin catalog file and the tracking
/// project, and names the real install command rather than a retired one.
///
/// Trace: FR-081-AC-7
#[test]
fn tc_264_034_the_catalog_document_names_the_catalog_file_and_tracking_project() {
    let r = render("object", "", &[]);
    let doc = r.text("docs/catalog-entry.md");
    assert!(doc.contains("`default-modules.yaml` in `agent-ix/quoin`"));
    assert!(doc.contains("type: git-subdir"));
    assert!(doc.contains("url: agent-ix/spec-objects-example"));
    assert!(doc.contains("path: spec_objects_example"));
    assert!(doc.contains("Project 18"));
    assert!(
        doc.contains(
            "quoin module install github:agent-ix/spec-objects-example//spec_objects_example"
        ),
        "the verification step uses the git-subdir install the catalog entry uses"
    );
    let readme = r.text("README.md");
    for text in [&doc, &readme] {
        assert!(
            !text.contains("quoin plugin"),
            "`quoin plugin` is a deprecated alias"
        );
        assert!(
            !text.contains("npm:@"),
            "npm sources are not installable by quoin"
        );
    }
}

/// Trace: FR-082-AC-2
/// Trace: FR-082-AC-5
#[test]
fn tc_264_035_the_governance_tree_has_its_folders_and_no_hand_kept_matrix() {
    for (kind, r) in all_variants() {
        for file in [
            "spec/spec.md",
            "spec/index.md",
            "spec/stakeholder/index.md",
            "spec/usecase/index.md",
            "spec/functional/index.md",
            "spec/non-functional/index.md",
        ] {
            assert!(r.path(file).is_file(), "variant {kind} lacks {file}");
        }
        for matrix in ["spec/matrix.md", "spec/tests.md"] {
            assert!(!r.path(matrix).exists(), "variant {kind} carries {matrix}");
        }
        let index = r.text("spec/index.md");
        assert!(!index.contains("matrix.md") && !index.contains("tests.md"));
    }
}

// ---------------------------------------------------------------- FR-083

/// One conformance finding: which variant, which surface or file, and why.
#[derive(Debug)]
struct Finding {
    variant: String,
    subject: String,
    reason: String,
}

fn contract() -> Value {
    quoin_yaml::from_str(&fs::read_to_string(Path::new(TEMPLATE).join("conformance.yaml")).unwrap())
        .unwrap()
}

fn is_forbidden(name: &str, glob: &str) -> bool {
    let leaf = glob.trim_start_matches("**/");
    let file = name.rsplit('/').next().unwrap();
    leaf.strip_prefix('*')
        .map_or(file == leaf, |suffix| file.ends_with(suffix))
}

/// Judge one rendered tree against the declared conformance contract.
fn conformance(variant: &str, kind: &str, root: &Path, package: &str) -> Vec<Finding> {
    let contract = contract();
    let mut findings = Vec::new();
    let mut flag = |subject: &str, reason: String| {
        findings.push(Finding {
            variant: variant.to_owned(),
            subject: subject.to_owned(),
            reason,
        });
    };

    let required = |list: &Value| -> Vec<String> {
        list.as_array()
            .map(|a| {
                a.iter()
                    .map(|p| p.as_str().unwrap().replace("{package}", package))
                    .collect()
            })
            .unwrap_or_default()
    };
    let mut surfaces = required(&contract["all"]["required_paths"]);
    surfaces.extend(required(&contract["kinds"][kind]["required_paths"]));
    for surface in surfaces {
        let path = root.join(&surface);
        let present = if surface.ends_with('/') {
            fs::read_dir(&path).is_ok_and(|mut d| d.next().is_some())
        } else {
            path.is_file()
        };
        if !present {
            flag(
                &surface,
                "a surface the conformance contract requires is missing".into(),
            );
        }
    }

    let files = files_under(root);
    let globs: Vec<String> = contract["all"]["forbidden_globs"]
        .as_array()
        .unwrap()
        .iter()
        .map(|g| g.as_str().unwrap().to_owned())
        .collect();
    let hosts: Vec<String> = contract["private_registry"]["hosts"]
        .as_array()
        .unwrap()
        .iter()
        .map(|h| h.as_str().unwrap().to_owned())
        .collect();
    let residue: Vec<(String, Regex, String)> = contract["residue"]
        .as_object()
        .unwrap()
        .iter()
        .map(|(name, class)| {
            (
                name.clone(),
                Regex::new(class["pattern"].as_str().unwrap()).unwrap(),
                class["reason"].as_str().unwrap().to_owned(),
            )
        })
        .collect();
    for (name, bytes) in &files {
        for glob in &globs {
            if is_forbidden(name, glob) {
                flag(name, format!("file matches forbidden glob {glob}"));
            }
        }
        let Ok(text) = std::str::from_utf8(bytes) else {
            continue;
        };
        for host in &hosts {
            if text.contains(host.as_str()) {
                flag(name, format!("names the private registry {host}"));
            }
        }
        for (class, pattern, reason) in &residue {
            if pattern.is_match(text) {
                flag(name, format!("{class}: {reason}"));
            }
        }
    }
    findings
}

/// Trace: FR-083-AC-1
/// Trace: FR-081-AC-4
#[test]
fn tc_264_040_every_variant_renders_and_conforms() {
    let mut failures = Vec::new();
    for (kind, r) in all_variants() {
        failures.extend(conformance(kind, kind, &r.root, &r.package));
    }
    assert!(failures.is_empty(), "{failures:#?}");
}

/// A rendered directory does not outlive its checks, whether they pass or fail.
///
/// Trace: FR-083-AC-2
/// Trace: FR-083-CON-1
#[test]
fn tc_264_041_a_rendered_directory_is_removed_after_a_failing_check() {
    let r = render("object", "", &[]);
    let root = r.root.clone();
    assert!(root.is_dir());
    assert!(
        !root.starts_with(env!("CARGO_MANIFEST_DIR")),
        "rendered inside the working tree"
    );
    let outcome = catch_unwind(AssertUnwindSafe(move || {
        let _held = r;
        panic!("a check failed while the rendering was held");
    }));
    assert!(outcome.is_err(), "the check failed");
    assert!(
        !root.exists(),
        "the rendered directory survived the failure"
    );
}

/// Each class of residue, injected into a rendered file, fails conformance
/// naming that file.
///
/// Trace: FR-083-AC-3
#[test]
fn tc_264_042_injected_residue_fails_naming_the_file() {
    let injections = [
        ("unresolved_token", "{{ cookiecutter.org }}"),
        (
            "placeholder_org",
            "git clone https://github.com/your-org/repo",
        ),
        ("absolute_render_path", "/home/someone/dev/repo"),
        ("credential", "password = \"hunter2hunter2\""),
        ("npm.ix", "registry=http://npm.ix/"),
    ];
    for (class, payload) in injections {
        let r = render("object", "", &[]);
        let target = r.path("README.md");
        let mut text = fs::read_to_string(&target).unwrap();
        text.push('\n');
        text.push_str(payload);
        text.push('\n');
        fs::write(&target, text).unwrap();
        let findings = conformance("object", "object", &r.root, &r.package);
        assert!(
            findings
                .iter()
                .any(|f| f.subject == "README.md" && f.reason.contains(class)),
            "{class}: {findings:#?}"
        );
    }
    let r = render("object", "", &[]);
    fs::write(r.path(".npmrc"), "registry=https://registry.npmjs.org/\n").unwrap();
    let findings = conformance("object", "object", &r.root, &r.package);
    assert!(
        findings.iter().any(|f| f.subject == ".npmrc"),
        "{findings:#?}"
    );
}

/// Removing a surface the contract names fails naming that surface and variant.
///
/// Trace: FR-083-AC-4
#[test]
fn tc_264_043_a_missing_surface_fails_naming_it() {
    let r = render("artifact", "", &[]);
    fs::remove_file(r.path("docs/catalog-entry.md")).unwrap();
    fs::remove_dir_all(r.path("{package}/examples")).unwrap();
    let findings = conformance("artifact", "artifact", &r.root, &r.package);
    for surface in ["docs/catalog-entry.md", "spec_objects_example/examples/"] {
        assert!(
            findings
                .iter()
                .any(|f| f.subject == surface && f.variant == "artifact"),
            "{surface}: {findings:#?}"
        );
    }
}

/// Trace: FR-083-CON-2
#[test]
fn tc_264_044_the_conformance_contract_is_a_declared_file_with_exactly_these_blocks() {
    let contract = contract();
    let keys: BTreeSet<&str> = contract
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect();
    assert_eq!(
        keys,
        BTreeSet::from([
            "contract_version",
            "all",
            "kinds",
            "residue",
            "private_registry"
        ]),
        "the gate reads `all`, `kinds`, `residue` and `private_registry`; `contract_version` versions the file itself; a block nothing reads is deleted"
    );
}

// ---------------------------------------------------------------- render-only criteria

/// Trace: FR-078-AC-7
#[test]
fn tc_264_050_no_type_name_is_declared_twice() {
    for (kind, r) in all_variants() {
        let manifest = r.manifest();
        let mut names = type_names(&manifest, "artifact_types");
        names.extend(type_names(&manifest, "object_types"));
        let unique: BTreeSet<&String> = names.iter().collect();
        assert_eq!(unique.len(), names.len(), "variant {kind}: {names:?}");
    }
}

/// Trace: FR-081-AC-8
/// Trace: FR-081-CON-1
#[test]
fn tc_264_051_no_dependency_is_a_local_path_or_has_an_upper_bound() {
    let r = render("mixed", IMPORT, &[]);
    let pyproject = r.text("pyproject.toml");
    let mut in_deps = false;
    for line in pyproject.lines() {
        if line.starts_with('[') {
            in_deps = line.ends_with("dependencies]");
            continue;
        }
        // The interpreter constraint is not a dependency declaration.
        if !in_deps || line.trim().is_empty() || line.starts_with("python ") {
            continue;
        }
        for bad in ["path", "develop", "<", "file:", "git ="] {
            assert!(
                !line.contains(bad),
                "dependency line `{line}` contains {bad}"
            );
        }
    }
    let package: Value = serde_json::from_str(&r.text("package.json")).unwrap();
    for section in ["dependencies", "devDependencies"] {
        for (name, spec) in package[section].as_object().into_iter().flatten() {
            let spec = spec.as_str().unwrap();
            for bad in ["file:", "link:", "workspace:", "<", ".."] {
                assert!(!spec.contains(bad), "{name}: {spec} contains {bad}");
            }
        }
    }
}

fn spec_files(r: &Render) -> Vec<(String, String)> {
    files_under(&r.root.join("spec"))
        .into_iter()
        .map(|(name, bytes)| (name, String::from_utf8(bytes).unwrap()))
        .collect()
}

/// Trace: FR-082-AC-3
#[test]
fn tc_264_052_no_verification_cell_carries_a_test_case_id() {
    let tc = Regex::new(r"\bTC-[A-Z]*-?\d+").unwrap();
    for (kind, r) in all_variants() {
        for (name, text) in spec_files(&r) {
            for line in text
                .lines()
                .filter(|l| l.starts_with("| ") && l.contains("-AC-"))
            {
                assert!(!tc.is_match(line), "variant {kind}: {name}: {line}");
            }
        }
    }
}

/// Trace: FR-082-AC-4
#[test]
fn tc_264_053_every_rendered_test_carries_a_trace_tag_naming_an_existing_criterion() {
    let tag = Regex::new(r#"pytest\.mark\.trace\("([^"]+)"\)"#).unwrap();
    for (kind, r) in all_variants() {
        let spec: String = spec_files(&r).into_iter().map(|(_, t)| t).collect();
        for (name, bytes) in files_under(&r.root.join("tests")) {
            if Path::new(&name).extension().is_none_or(|e| e != "py") {
                continue;
            }
            let text = String::from_utf8(bytes).unwrap();
            let lines: Vec<&str> = text.lines().collect();
            for (i, line) in lines.iter().enumerate() {
                if !line.starts_with("def test_") {
                    continue;
                }
                let mut ids = Vec::new();
                let mut j = i;
                while j > 0 && lines[j - 1].starts_with('@') {
                    j -= 1;
                    ids.extend(tag.captures_iter(lines[j]).map(|c| c[1].to_owned()));
                }
                assert!(
                    !ids.is_empty(),
                    "variant {kind}: {name}: `{line}` has no trace tag"
                );
                for id in ids {
                    assert!(
                        spec.contains(&format!("| {id} |")),
                        "variant {kind}: {name}: `{line}` traces {id}, which is not a rendered criterion"
                    );
                }
            }
        }
    }
}

/// Trace: FR-082-AC-6
/// Trace: FR-082-CON-2
#[test]
fn tc_264_054_the_rendered_spec_leaves_the_domain_to_the_maintainer_and_copies_no_module() {
    let catalog = quoin_yaml::from_str(
        &fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../default-modules.yaml"
        ))
        .unwrap(),
    )
    .unwrap();
    let maintained: Vec<String> = catalog["entries"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| e["name"].as_str().unwrap().to_owned())
        .collect();
    assert!(maintained.len() >= 10);
    for (kind, r) in all_variants() {
        let spec = r.text("spec/spec.md");
        let scope = spec
            .split("### Out of Scope")
            .nth(1)
            .expect("an Out of Scope section");
        assert!(
            scope.contains("This module's domain vocabulary") && scope.contains("maintainer's"),
            "variant {kind}: Out of Scope does not leave the domain to the maintainer"
        );
        for (name, text) in spec_files(&r) {
            for module in &maintained {
                assert!(
                    !text.contains(module.as_str()),
                    "variant {kind}: {name} names {module}"
                );
            }
        }
    }
}

/// Trace: FR-076-CON-1
#[test]
fn tc_264_055_the_template_carries_no_emitter_runtime_or_grammar_copy() {
    let project = Path::new(TEMPLATE).join("{{cookiecutter.repo_name}}");
    for (name, bytes) in files_under(&project) {
        let text = String::from_utf8_lossy(&bytes);
        assert!(!name.contains("node_modules"), "{name}");
        let ext = Path::new(&name)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("");
        assert!(
            !["rs", "ts", "js", "wasm"].contains(&ext),
            "{name} is source of an emitter or runtime"
        );
        assert!(
            !text.contains("model FieldDecl") && !text.contains("model ClauseRef"),
            "{name} copies the semantic-core grammar"
        );
    }
    let main = fs::read_to_string(project.join("typespec/main.tsp")).unwrap();
    assert!(main.contains("import \"@agent-ix/semantic-core\";"));
    assert!(main.contains("import \"@typespec/json-schema\";"));
}
