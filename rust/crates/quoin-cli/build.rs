// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Bake the same truthful version provenance into the native CLI that the
//! retired Node entrypoint carried: a clean tag is semver, while a development
//! build identifies its distance and commit. Source archives have no Git
//! metadata, so their Cargo package version remains the deterministic fallback.

use std::path::Path;
use std::process::Command;
use std::{fmt::Write as _, io};

mod build_git;

fn main() -> Result<(), Box<dyn std::error::Error>> {
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
    embed_selftest_cases()?;
    embed_validation_inputs()?;
    Ok(())
}

fn embed_validation_inputs() -> io::Result<()> {
    use std::fs;

    let repository = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
    let out_dir = output_directory()?;
    let mut generated =
        String::from("pub(crate) static EMBEDDED_VALIDATION_INPUTS: Option<&[EmbeddedFile]> = ");
    append_embedded_files(&repository, "spec/validation", "validation", &mut generated)?;
    generated.push_str(";\npub(crate) static EMBEDDED_ANOMALIES: Option<&[EmbeddedFile]> = ");
    append_embedded_files(&repository, "spec/anomalies", "anomalies", &mut generated)?;
    generated.push_str(";\npub(crate) static EMBEDDED_EXTERNAL_TOOLS_INDEX: Option<&[u8]> = ");
    let index = repository.join("spec/validation/external-tools.v1.json");
    println!("cargo:rerun-if-changed={}", index.display());
    if index.is_file() {
        let source = index.canonicalize()?;
        let source = source.to_str().ok_or_else(|| {
            io::Error::other("external tools index path is not representable as UTF-8")
        })?;
        let _ = write!(generated, "Some(include_bytes!({source:?}))");
    } else {
        generated.push_str("None");
    }
    generated.push_str(";\n");
    fs::write(out_dir.join("embedded_validation.rs"), generated)
}

fn append_embedded_files(
    repository: &Path,
    relative_root: &str,
    label: &str,
    out: &mut String,
) -> io::Result<()> {
    let root = repository.join(relative_root);
    println!("cargo:rerun-if-changed={}", root.display());
    if let Some(parent) = root.parent() {
        println!("cargo:rerun-if-changed={}", parent.display());
    }
    if !root.is_dir() {
        out.push_str("None");
        return Ok(());
    }

    let mut paths = Vec::new();
    collect_validation_files(&root, &root, &mut paths)
        .map_err(|error| io::Error::other(format!("cannot embed {label} records: {error}")))?;
    paths.sort_by(|left, right| left.0.cmp(&right.0));
    out.push_str("Some(&[\n");
    for (relative, path) in paths {
        let source = path.canonicalize()?;
        let source = source.to_str().ok_or_else(|| {
            io::Error::other("embedded validation record path is not representable as UTF-8")
        })?;
        let _ = writeln!(
            out,
            "EmbeddedFile {{ path: {relative:?}, bytes: include_bytes!({source:?}) }},"
        );
    }
    out.push_str("])");
    Ok(())
}

fn collect_validation_files(
    root: &Path,
    directory: &Path,
    files: &mut Vec<(String, std::path::PathBuf)>,
) -> io::Result<()> {
    use std::fs;
    use std::path::Component;

    println!("cargo:rerun-if-changed={}", directory.display());
    let mut entries = fs::read_dir(directory)?
        .map(|entry| entry.map(|entry| entry.path()))
        .collect::<Result<Vec<_>, _>>()?;
    entries.sort();
    for path in entries {
        println!("cargo:rerun-if-changed={}", path.display());
        let metadata = fs::symlink_metadata(&path)?;
        if metadata.file_type().is_symlink() {
            return Err(io::Error::other(format!(
                "symlink is not allowed in embedded validation records: {}",
                path.display()
            )));
        }
        if metadata.is_dir() {
            collect_validation_files(root, &path, files)?;
            continue;
        }
        if !metadata.is_file() || path.extension().is_none_or(|ext| ext != "md") {
            continue;
        }
        let relative = path.strip_prefix(root).map_err(io::Error::other)?;
        if relative
            .components()
            .any(|part| !matches!(part, Component::Normal(_)))
        {
            return Err(io::Error::other(
                "embedded validation record path is not normalized",
            ));
        }
        let relative = relative
            .to_str()
            .ok_or_else(|| io::Error::other("embedded validation path is not UTF-8"))?
            .replace(std::path::MAIN_SEPARATOR, "/");
        files.push((relative, path));
    }
    Ok(())
}

fn output_directory() -> io::Result<std::path::PathBuf> {
    std::env::var_os("OUT_DIR")
        .map(std::path::PathBuf::from)
        .ok_or_else(|| io::Error::other("Cargo did not set OUT_DIR for the build script"))
}

fn embed_selftest_cases() -> io::Result<()> {
    use std::fs;

    let repository = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
    let cases_root = repository.join("selftest/cases");
    println!("cargo:rerun-if-changed={}", cases_root.display());

    let mut case_dirs = fs::read_dir(&cases_root)?
        .map(|entry| entry.map(|entry| entry.path()))
        .collect::<Result<Vec<_>, _>>()?;
    case_dirs.sort();

    let mut cases = Vec::new();
    for case_dir in case_dirs {
        let id = validate_case_directory(&case_dir)?;
        let mut files = Vec::new();
        collect_case_files(&case_dir, &case_dir, &mut files)?;
        files.sort_by(|left, right| left.0.cmp(&right.0));
        cases.push((id, files));
    }

    let mut generated = String::from("pub(crate) static EMBEDDED_CASES: &[EmbeddedCase] = &[\n");
    for (id, files) in cases {
        let _ = writeln!(generated, "    EmbeddedCase {{ id: {id:?}, files: &[");
        for (relative, source) in files {
            let source = source.canonicalize()?;
            let source = source.to_str().ok_or_else(|| {
                io::Error::other("embedded selftest source path is not representable as UTF-8")
            })?;
            let _ = writeln!(
                generated,
                "        EmbeddedCaseFile {{ path: {relative:?}, bytes: include_bytes!({source:?}) }},"
            );
        }
        generated.push_str("    ] },\n");
    }
    generated.push_str("];\n");
    fs::write(output_directory()?.join("embedded_selftest.rs"), generated)
}

fn validate_case_directory(case_dir: &Path) -> io::Result<String> {
    let file_type = std::fs::symlink_metadata(case_dir)?.file_type();
    if file_type.is_symlink() || !file_type.is_dir() {
        return Err(io::Error::other(format!(
            "quoin-cli selftest cases must be directories: {}",
            case_dir.display()
        )));
    }

    let id = case_dir
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or_else(|| io::Error::other("quoin-cli selftest case ids must be UTF-8"))?;
    if id.is_empty()
        || !id
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
    {
        return Err(io::Error::other(format!(
            "invalid quoin-cli selftest case id {id:?}"
        )));
    }

    validate_required_case_files(case_dir, id)?;
    Ok(id.to_owned())
}

fn validate_required_case_files(case_dir: &Path, id: &str) -> io::Result<()> {
    for required in ["run.sh", "input", "expected/exit_code", "expected/stdout"] {
        let path = case_dir.join(required);
        if !path.exists() {
            return Err(io::Error::other(format!(
                "quoin-cli selftest case {id:?} is missing {}",
                path.display()
            )));
        }
    }
    Ok(())
}

fn collect_case_files(
    case_root: &Path,
    directory: &Path,
    files: &mut Vec<(String, std::path::PathBuf)>,
) -> io::Result<()> {
    use std::fs;

    println!("cargo:rerun-if-changed={}", directory.display());
    let mut entries = fs::read_dir(directory)?
        .map(|entry| entry.map(|entry| entry.path()))
        .collect::<Result<Vec<_>, _>>()?;
    entries.sort();
    for path in entries {
        collect_case_entry(case_root, &path, files)?;
    }
    Ok(())
}

fn collect_case_entry(
    case_root: &Path,
    path: &Path,
    files: &mut Vec<(String, std::path::PathBuf)>,
) -> io::Result<()> {
    use std::fs;
    use std::path::Component;

    println!("cargo:rerun-if-changed={}", path.display());
    let kind = fs::symlink_metadata(path)?.file_type();
    if kind.is_symlink() {
        return Err(io::Error::other(format!(
            "symlink is not allowed in a selftest case: {}",
            path.display()
        )));
    }
    if kind.is_dir() {
        return collect_case_files(case_root, path, files);
    }
    if !kind.is_file() {
        return Err(io::Error::other(format!(
            "non-file entry in a selftest case: {}",
            path.display()
        )));
    }

    let relative = path.strip_prefix(case_root).map_err(io::Error::other)?;
    if relative
        .components()
        .any(|component| !matches!(component, Component::Normal(_)))
    {
        return Err(io::Error::other("selftest file path is not normalized"));
    }
    let relative = relative
        .to_str()
        .ok_or_else(|| io::Error::other("selftest path is not UTF-8"))?
        .replace(std::path::MAIN_SEPARATOR, "/");
    files.push((relative, path.to_path_buf()));
    Ok(())
}
