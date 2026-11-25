// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Bake the same truthful version provenance into the native CLI that the
//! retired Node entrypoint carried: a clean tag is semver, while a development
//! build identifies its distance and commit. Source archives have no Git
//! metadata, so their Cargo package version remains the deterministic fallback.

use std::path::Path;
use std::process::Command;

mod build_git;

fn main() {
    // `.git` is a text file in a linked worktree, so watching it never fires;
    // watch the files Git itself says move HEAD, branch and tags.
    for path in build_git::watched_paths(Path::new(".")) {
        println!("cargo:rerun-if-changed={}", path.display());
    }
    let version = Command::new("git")
        .args(["describe", "--tags", "--dirty"])
        .output()
        .ok()
        .filter(|output| output.status.success())
        .and_then(|output| String::from_utf8(output.stdout).ok())
        .map(|version| version.trim().trim_start_matches('v').to_owned())
        .filter(|version| !version.is_empty())
        .unwrap_or_else(|| env!("CARGO_PKG_VERSION").to_owned());
    println!("cargo:rustc-env=QUOIN_VERSION={version}");
    embed_selftest_cases();
}

fn embed_selftest_cases() {
    use std::fs;
    use std::path::PathBuf;

    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let repository = manifest_dir.join("../../..");
    let cases_root = repository.join("selftest/cases");
    println!("cargo:rerun-if-changed={}", cases_root.display());

    let mut cases = Vec::new();
    let entries = fs::read_dir(&cases_root).unwrap_or_else(|error| {
        panic!("quoin-cli selftest case directory {} is unavailable: {error}", cases_root.display())
    });
    let mut case_dirs = entries
        .map(|entry| entry.expect("selftest case directory entry is readable").path())
        .collect::<Vec<_>>();
    case_dirs.sort();

    for case_dir in case_dirs {
        let file_type = fs::symlink_metadata(&case_dir)
            .expect("selftest case entry metadata is readable")
            .file_type();
        if file_type.is_symlink() || !file_type.is_dir() {
            panic!("quoin-cli selftest cases must be directories: {}", case_dir.display());
        }
        let id = case_dir
            .file_name()
            .and_then(|value| value.to_str())
            .expect("selftest case ids are UTF-8");
        if id.is_empty()
            || !id
                .bytes()
                .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
        {
            panic!("invalid quoin selftest case id {id:?}");
        }
        for required in ["run.sh", "input", "expected/exit_code", "expected/stdout"] {
            let path = case_dir.join(required);
            if !path.exists() {
                panic!("quoin selftest case {id:?} is missing {}", path.display());
            }
        }

        let mut files = Vec::new();
        collect_case_files(&case_dir, &case_dir, &mut files)
            .unwrap_or_else(|error| panic!("cannot embed quoin selftest case {id:?}: {error}"));
        files.sort_by(|left, right| left.0.cmp(&right.0));
        cases.push((id.to_owned(), files));
    }

    let mut generated = String::from("pub(crate) static EMBEDDED_CASES: &[EmbeddedCase] = &[\n");
    for (id, files) in cases {
        generated.push_str(&format!("    EmbeddedCase {{ id: {id:?}, files: &[\n"));
        for (relative, source) in files {
            let source = source
                .canonicalize()
                .expect("embedded selftest source path canonicalizes");
            let source = source
                .to_str()
                .expect("embedded selftest source path is UTF-8");
            generated.push_str(&format!(
                "        EmbeddedCaseFile {{ path: {relative:?}, bytes: include_bytes!({source:?}) }},\n"
            ));
        }
        generated.push_str("    ] },\n");
    }
    generated.push_str("];\n");

    let out_dir = std::env::var_os("OUT_DIR").expect("Cargo sets OUT_DIR for build scripts");
    fs::write(PathBuf::from(out_dir).join("embedded_selftest.rs"), generated)
        .expect("write embedded selftest case table");

    fn collect_case_files(
        case_root: &Path,
        directory: &Path,
        files: &mut Vec<(String, PathBuf)>,
    ) -> std::io::Result<()> {
        use std::fs;
        use std::path::Component;

        println!("cargo:rerun-if-changed={}", directory.display());
        let mut entries = fs::read_dir(directory)?
            .map(|entry| entry.map(|entry| entry.path()))
            .collect::<Result<Vec<_>, _>>()?;
        entries.sort();
        for path in entries {
            println!("cargo:rerun-if-changed={}", path.display());
            let kind = fs::symlink_metadata(&path)?.file_type();
            if kind.is_symlink() {
                return Err(std::io::Error::other(format!(
                    "symlink is not allowed in a selftest case: {}",
                    path.display()
                )));
            }
            if kind.is_dir() {
                collect_case_files(case_root, &path, files)?;
                continue;
            }
            if !kind.is_file() {
                return Err(std::io::Error::other(format!(
                    "non-file entry in a selftest case: {}",
                    path.display()
                )));
            }
            let relative = path.strip_prefix(case_root).map_err(std::io::Error::other)?;
            if relative
                .components()
                .any(|component| !matches!(component, Component::Normal(_)))
            {
                return Err(std::io::Error::other("selftest file path is not normalized"));
            }
            let relative = relative
                .to_str()
                .ok_or_else(|| std::io::Error::other("selftest path is not UTF-8"))?
                .replace(std::path::MAIN_SEPARATOR, "/");
            files.push((relative, path));
        }
        Ok(())
    }
}
