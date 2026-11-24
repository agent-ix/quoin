// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The export hand-off to `quire export` (FR-191, "Commit then export",
//! "Export currency").
//!
//! Quoin renders nothing and never bypasses `quire export`: it resolves the
//! repository revision, checks committed bytes, then runs the installed
//! `quire` as an external process.

use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use crate::engine::{Operation, Rendered, render_for, report_with_comparison};
use crate::error::{DossierError, QuireFailure};
use crate::registry::{Inputs, Registry};
use crate::report::Report;

/// The title of a bundle of several documents.
pub(crate) const BUNDLE_TITLE: &str = "Submission Dossier";

/// The id of a bundle of several documents.
pub(crate) const BUNDLE_ID: &str = "DOS";

/// What `quoin dossier export` passes through to `quire export`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExportOptions {
    /// `pdf` or `docx`.
    pub to: String,
    /// The file `quire export` writes.
    pub output: PathBuf,
    /// `--release`, unchanged.
    pub release: Option<String>,
    /// `--approval`, unchanged and in order.
    pub approvals: Vec<PathBuf>,
    /// Module roots used to validate Markdown inputs.
    pub modules: Vec<PathBuf>,
    /// `--template`, unchanged.
    pub template: Option<PathBuf>,
    /// The `quire` executable.
    pub quire: PathBuf,
}

/// Check currency, then export.
///
/// A missing or stale file is reported in the returned report and
/// `quire export` is not started. A `quire export` failure is reported in
/// [`Report::quire`].
///
/// # Errors
///
/// As [`render`].
pub fn export(
    registry: &Registry,
    kinds: &[String],
    out: &Path,
    inputs: &Inputs,
    options: &ExportOptions,
) -> Result<Report, DossierError> {
    let rendered = render_for(
        registry,
        kinds,
        out,
        inputs,
        Operation::Export {
            release: options.release.is_some(),
        },
    )?;
    let resolved_head = resolve_head(inputs.scope.as_path());
    let mut report = report_with_comparison(&rendered);
    let head = match resolved_head {
        Ok(head) => head,
        Err(detail) => {
            report.export_currency_failure = Some(detail);
            for entry in &mut report.documents {
                entry.comparison = Some(crate::report::Comparison::Differs);
            }
            return Ok(report);
        }
    };
    for (entry, document) in report.documents.iter_mut().zip(&rendered) {
        if committed_blob(&head.repository_root, &head.revision, &document.path)
            .is_none_or(|blob| blob != document.text.as_bytes())
        {
            entry.comparison = Some(crate::report::Comparison::Differs);
        }
    }
    if !report.differing().is_empty() {
        return Ok(report);
    }
    report.quire = run_quire(&rendered, options, &head.revision).err();
    Ok(report)
}

struct Head {
    revision: String,
    repository_root: PathBuf,
}

fn git_output(cwd: &Path, args: &[&str]) -> Result<Vec<u8>, String> {
    let output = Command::new("git")
        .args(args)
        .current_dir(cwd)
        .stdin(Stdio::null())
        .output()
        .map_err(|error| format!("cannot start git {}: {error}", args.join(" ")))?;
    if output.status.success() {
        Ok(output.stdout)
    } else {
        Err(format!(
            "git {} failed: {}",
            args.join(" "),
            String::from_utf8_lossy(&output.stderr).trim()
        ))
    }
}

fn resolve_head(scope: &Path) -> Result<Head, String> {
    let object_format = git_output(scope, &["rev-parse", "--show-object-format"])?;
    let object_format = String::from_utf8(object_format)
        .map_err(|error| format!("git returned a non-UTF-8 object format: {error}"))?;
    let expected_len = match object_format.trim() {
        "sha1" => 40,
        "sha256" => 64,
        format => return Err(format!("unsupported git object format {format:?}")),
    };
    let output = git_output(scope, &["rev-parse", "--verify", "HEAD^{commit}"])?;
    let revision = String::from_utf8(output)
        .map_err(|error| format!("git returned a non-UTF-8 HEAD id: {error}"))?;
    let revision = revision.trim();
    if revision.len() != expected_len || !revision.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(format!(
            "git returned an invalid full HEAD id for {object_format}: {revision:?}"
        ));
    }
    let root = git_output(scope, &["rev-parse", "--show-toplevel"])?;
    let root = String::from_utf8(root)
        .map_err(|error| format!("git returned a non-UTF-8 repository root: {error}"))?;
    let repository_root = PathBuf::from(root.trim())
        .canonicalize()
        .map_err(|error| format!("cannot resolve git repository root: {error}"))?;
    Ok(Head {
        revision: revision.to_owned(),
        repository_root,
    })
}

fn committed_blob(repository_root: &Path, revision: &str, path: &Path) -> Option<Vec<u8>> {
    let path = if path.is_absolute() {
        path.canonicalize().ok()?
    } else {
        std::env::current_dir()
            .ok()?
            .join(path)
            .canonicalize()
            .ok()?
    };
    let relative = path.strip_prefix(repository_root).ok()?;
    let relative = relative.to_str()?;
    let spec = format!("{revision}:{relative}");
    let output = Command::new("git")
        .args(["cat-file", "blob", &spec])
        .current_dir(repository_root)
        .stdin(Stdio::null())
        .output()
        .ok()?;
    output.status.success().then_some(output.stdout)
}

/// The arguments of the one `quire export` call, in order.
#[must_use]
pub(crate) fn quire_args(
    documents: &[Rendered],
    options: &ExportOptions,
    source_rev: &str,
) -> Vec<String> {
    let mut args = vec!["export".to_owned()];
    args.extend(
        documents
            .iter()
            .map(|document| document.path.display().to_string()),
    );
    args.extend([
        "--to".to_owned(),
        options.to.clone(),
        "--output".to_owned(),
        options.output.display().to_string(),
        "--source-rev".to_owned(),
        source_rev.to_owned(),
    ]);
    if let Some(release) = &options.release {
        args.extend(["--release".to_owned(), release.clone()]);
    }
    for module in &options.modules {
        args.extend(["--module".to_owned(), module.display().to_string()]);
    }
    for approval in &options.approvals {
        args.extend(["--approval".to_owned(), approval.display().to_string()]);
    }
    if let Some(template) = &options.template {
        args.extend(["--template".to_owned(), template.display().to_string()]);
    }
    // A bundle needs at least two inputs (quire-cli FR-039-AC-7), so one
    // document is exported without bundle arguments.
    if documents.len() > 1 {
        args.extend([
            "--bundle-title".to_owned(),
            BUNDLE_TITLE.to_owned(),
            "--bundle-id".to_owned(),
            BUNDLE_ID.to_owned(),
        ]);
    }
    args
}

fn run_quire(
    documents: &[Rendered],
    options: &ExportOptions,
    source_rev: &str,
) -> Result<(), QuireFailure> {
    let backup = backup_existing_output(&options.output)?;
    let result = Command::new(&options.quire)
        .args(quire_args(documents, options, source_rev))
        .stdin(Stdio::null())
        .output()
        .map_err(|error| QuireFailure {
            exit_code: None,
            stderr: format!("cannot start {}: {error}", options.quire.display()),
        })
        .and_then(|output| {
            if output.status.success() {
                Ok(())
            } else {
                Err(QuireFailure {
                    exit_code: output.status.code(),
                    stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
                })
            }
        });
    match result {
        Ok(()) => {
            if let Some(backup) = backup {
                fs::remove_file(&backup).map_err(|error| QuireFailure {
                    exit_code: None,
                    stderr: format!(
                        "export succeeded, but could not remove the output backup {}: {error}",
                        backup.display()
                    ),
                })?;
            }
            Ok(())
        }
        Err(mut failure) => {
            if let Err(error) = remove_partial_output(&options.output) {
                append_failure(
                    &mut failure,
                    &format!(
                        "could not remove the partial output {}: {error}",
                        options.output.display()
                    ),
                );
            }
            if let Some(backup) = backup
                && let Err(error) = fs::rename(&backup, &options.output)
            {
                append_failure(
                    &mut failure,
                    &format!(
                        "could not restore the pre-existing output {} from {}: {error}",
                        options.output.display(),
                        backup.display()
                    ),
                );
            }
            Err(failure)
        }
    }
}

fn backup_existing_output(output: &Path) -> Result<Option<PathBuf>, QuireFailure> {
    match fs::symlink_metadata(output) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => {
            return Err(QuireFailure {
                exit_code: None,
                stderr: format!("cannot inspect output {}: {error}", output.display()),
            });
        }
        Ok(metadata) if !metadata.file_type().is_file() => {
            return Err(QuireFailure {
                exit_code: None,
                stderr: format!(
                    "refusing to export over non-regular output {}",
                    output.display()
                ),
            });
        }
        Ok(_) => {}
    }
    let parent = output
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let temporary = tempfile::Builder::new()
        .prefix(".quoin-dossier-output-")
        .tempfile_in(parent)
        .map_err(|error| QuireFailure {
            exit_code: None,
            stderr: format!(
                "cannot reserve output backup beside {}: {error}",
                output.display()
            ),
        })?;
    let backup = temporary.path().to_owned();
    drop(temporary);
    fs::rename(output, &backup).map_err(|error| QuireFailure {
        exit_code: None,
        stderr: format!(
            "cannot preserve pre-existing output {}: {error}",
            output.display()
        ),
    })?;
    Ok(Some(backup))
}

fn remove_partial_output(output: &Path) -> std::io::Result<()> {
    match fs::symlink_metadata(output) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error),
        Ok(metadata) if metadata.file_type().is_dir() => fs::remove_dir(output),
        Ok(_) => fs::remove_file(output),
    }
}

fn append_failure(failure: &mut QuireFailure, detail: &str) {
    if !failure.stderr.is_empty() && !failure.stderr.ends_with('\n') {
        failure.stderr.push('\n');
    }
    let _ = write!(failure.stderr, "{detail}");
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]
mod tests {
    use super::*;
    use crate::testing::{doc, inputs, registry};
    use ix_trace_rs::trace;

    fn two() -> Registry {
        registry(vec![doc("a", "DOS-A", false), doc("b", "DOS-B", false)])
    }

    fn options(quire: &str) -> ExportOptions {
        ExportOptions {
            to: "pdf".to_owned(),
            output: PathBuf::from("out.pdf"),
            release: Some("v1".to_owned()),
            approvals: vec![PathBuf::from("a1.json"), PathBuf::from("a2.json")],
            modules: Vec::new(),
            template: Some(PathBuf::from("t.typ")),
            quire: PathBuf::from(quire),
        }
    }

    fn commit_docs(work: &Path) {
        for args in [
            &["init", "-q"][..],
            &["config", "user.name", "Dossier test"],
            &["config", "user.email", "dossier@example.invalid"],
            &["add", "dossier"],
            &["commit", "-qm", "dossier fixtures"],
        ] {
            let output = Command::new("git")
                .args(args)
                .current_dir(work)
                .output()
                .unwrap();
            assert!(
                output.status.success(),
                "{}",
                String::from_utf8_lossy(&output.stderr)
            );
        }
    }

    /// A `quire` that runs `body` with `$out` set to its `--output`.
    #[cfg(unix)]
    fn script(work: &Path, body: &str) -> PathBuf {
        use std::os::unix::fs::PermissionsExt as _;
        let path = work.join("quire");
        fs::write(
            &path,
            format!(
                "#!/bin/sh\nall=\"$*\"\nout=\nwhile [ $# -gt 0 ]; do\n  if [ \"$1\" = --output ]; then out=$2; fi\n  shift\ndone\n{body}\n"
            ),
        )
        .unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
        path
    }

    /// Generate into `work/dossier` and export with `quire`, writing `work/out.pdf`.
    #[cfg(unix)]
    fn export_with(work: &Path, quire: &Path) -> Report {
        let out = work.join("dossier");
        crate::engine::generate(&two(), &[], &out, &inputs()).unwrap();
        commit_docs(work);
        let mut options = options(quire.to_str().unwrap());
        options.output = work.join("out.pdf");
        let mut input = inputs();
        input.scope = work.to_owned();
        export(&two(), &[], &out, &input, &options).unwrap()
    }

    #[trace("FR-191-AC-6")]
    #[test]
    fn a_missing_quire_is_reported_naming_the_path_tried() {
        let work = tempfile::tempdir().unwrap();
        let out = work.path().join("dossier");
        crate::engine::generate(&two(), &[], &out, &inputs()).unwrap();
        commit_docs(work.path());
        let mut options = options("/nonexistent/quire");
        options.output = work.path().join("out.pdf");
        let mut input = inputs();
        input.scope = work.path().to_owned();
        let report = export(&two(), &[], &out, &input, &options).unwrap();
        let quire = report.quire.unwrap();
        assert_eq!(quire.exit_code, None);
        assert!(
            quire.stderr.contains("/nonexistent/quire"),
            "{}",
            quire.stderr
        );
    }

    #[trace("FR-191-AC-10", "FR-191-AC-11")]
    #[test]
    fn a_missing_or_stale_file_stops_before_quire_starts() {
        let work = tempfile::tempdir().unwrap();
        let out = work.path().join("dossier");
        crate::engine::generate(&two(), &[], &out, &inputs()).unwrap();
        commit_docs(work.path());
        fs::write(out.join("DOS-B.md"), "edited").unwrap();
        fs::remove_file(out.join("DOS-A.md")).unwrap();
        let mut options = options("/nonexistent/quire");
        options.output = work.path().join("out.pdf");
        let mut input = inputs();
        input.scope = work.path().to_owned();
        let report = export(&two(), &[], &out, &input, &options).unwrap();
        assert_eq!(report.differing(), ["DOS-A", "DOS-B"]);
        assert!(report.quire.is_none(), "quire was started");
        let elsewhere = export(&two(), &[], &work.path().join("other"), &input, &options).unwrap();
        assert_eq!(elsewhere.differing(), ["DOS-A", "DOS-B"]);
    }

    #[cfg(unix)]
    #[trace("FR-191-AC-7")]
    #[test]
    fn a_partial_output_from_a_failed_quire_is_removed() {
        let work = tempfile::tempdir().unwrap();
        let quire = script(
            work.path(),
            "echo partial > \"$out\"; echo refused >&2; exit 7",
        );
        let report = export_with(work.path(), &quire);
        let failure = report.quire.unwrap();
        assert_eq!(failure.exit_code, Some(7));
        assert_eq!(failure.stderr, "refused\n");
        assert!(!work.path().join("out.pdf").exists());
    }

    #[cfg(unix)]
    #[trace("FR-191-AC-7")]
    #[test]
    fn a_failed_quire_that_wrote_nothing_is_not_a_cleanup_error() {
        let work = tempfile::tempdir().unwrap();
        let quire = script(work.path(), "echo refused >&2; exit 7");
        let failure = export_with(work.path(), &quire).quire.unwrap();
        assert_eq!(failure.stderr, "refused\n");
    }

    #[cfg(unix)]
    #[trace("FR-191-AC-26")]
    #[test]
    fn an_output_that_existed_before_a_failed_quire_is_preserved() {
        let work = tempfile::tempdir().unwrap();
        fs::write(work.path().join("out.pdf"), b"sentinel bytes\0\xff").unwrap();
        let quire = script(work.path(), "echo partial > \"$out\"; exit 7");
        let failure = export_with(work.path(), &quire).quire.unwrap();
        assert!(!failure.stderr.contains("could not remove"));
        assert_eq!(
            fs::read(work.path().join("out.pdf")).unwrap(),
            b"sentinel bytes\0\xff"
        );
    }

    #[cfg(unix)]
    #[trace("FR-191-AC-7")]
    #[test]
    fn a_partial_output_that_cannot_be_removed_is_reported_beside_quires_stderr() {
        let work = tempfile::tempdir().unwrap();
        // A non-empty directory at --output: remove_file cannot remove it.
        let quire = script(
            work.path(),
            "mkdir -p \"$out/inner\"; echo refused >&2; exit 7",
        );
        let failure = export_with(work.path(), &quire).quire.unwrap();
        assert_eq!(failure.exit_code, Some(7));
        assert!(
            failure.stderr.starts_with("refused\n"),
            "{}",
            failure.stderr
        );
        assert!(
            failure
                .stderr
                .contains("could not remove the partial output"),
            "{}",
            failure.stderr
        );
        assert!(failure.stderr.contains("out.pdf"));
    }

    /// `DOS-A` plus a release-only document (as `DOS-ROP` is), generated and committed.
    #[cfg(unix)]
    fn release_only_fixture(work: &Path) -> (Registry, PathBuf) {
        let mut only = doc("only", "DOS-ONLY", false);
        only.release_only = true;
        let registry = registry(vec![doc("a", "DOS-A", false), only]);
        let out = work.join("dossier");
        let mut input = inputs();
        input.version.clear();
        crate::engine::generate(&registry, &["only".to_owned()], &out, &input).unwrap();
        input.version = "1.0".to_owned();
        crate::engine::generate(&registry, &["a".to_owned()], &out, &input).unwrap();
        commit_docs(work);
        (registry, out)
    }

    #[cfg(unix)]
    fn export_release_only(
        work: &Path,
        kinds: &[&str],
        release: Option<&str>,
    ) -> (Report, Vec<String>) {
        let (registry, out) = release_only_fixture(work);
        let args_file = work.join("quire-args.txt");
        let quire = script(
            work,
            &format!(
                "echo \"$all\" > {}\ncase \" $all \" in\n  *\" --release \"*) echo ok > \"$out\";;\n  *) echo refused >&2; exit 3;;\nesac",
                args_file.display()
            ),
        );
        let mut options = options(quire.to_str().unwrap());
        options.output = work.join("out.pdf");
        options.release = release.map(str::to_owned);
        options.approvals.clear();
        options.template = None;
        let mut input = inputs();
        input.scope = work.to_owned();
        let kinds: Vec<String> = kinds.iter().map(|kind| (*kind).to_owned()).collect();
        let report = export(&registry, &kinds, &out, &input, &options).unwrap();
        let args = fs::read_to_string(&args_file)
            .map(|text| text.split_whitespace().map(str::to_owned).collect())
            .unwrap_or_default();
        (report, args)
    }

    #[cfg(unix)]
    #[trace("FR-191-AC-30", "FR-207-AC-10")]
    #[test]
    fn an_unfiltered_export_leaves_a_release_only_document_out_unless_release_is_given() {
        let work = tempfile::tempdir().unwrap();
        let (report, args) = export_release_only(work.path(), &[], None);
        assert_eq!(
            report
                .documents
                .iter()
                .map(|document| document.id.as_str())
                .collect::<Vec<_>>(),
            ["DOS-A"]
        );
        assert!(report.quire.is_some(), "quire ran and refused the sentinel");
        assert!(args.iter().any(|arg| arg.ends_with("DOS-A.md")));
        assert!(!args.iter().any(|arg| arg.ends_with("DOS-ONLY.md")));
        assert!(!args.iter().any(|arg| arg == "--bundle-id"));
    }

    #[cfg(unix)]
    #[trace("FR-191-AC-30", "FR-207-AC-10")]
    #[test]
    fn an_unfiltered_export_with_release_bundles_the_release_only_document_last_with_no_version() {
        let work = tempfile::tempdir().unwrap();
        let (report, args) = export_release_only(work.path(), &[], Some("v1"));
        assert!(report.quire.is_none(), "{:?}", report.quire);
        let inputs: Vec<_> = args
            .iter()
            .filter(|arg| Path::new(arg).extension().is_some_and(|ext| ext == "md"))
            .map(|arg| arg.rsplit('/').next().unwrap())
            .collect();
        assert_eq!(inputs, ["DOS-A.md", "DOS-ONLY.md"]);
        assert!(args.windows(2).any(|pair| pair == ["--release", "v1"]));
        assert!(!args.iter().any(|arg| arg == "--version"));
        assert!(args.iter().any(|arg| arg == "--bundle-id"));
    }

    #[cfg(unix)]
    #[trace("FR-191-AC-29", "FR-207-AC-10")]
    #[test]
    fn an_explicit_release_only_selection_without_release_is_refused_by_quire_and_output_is_unchanged()
     {
        for kinds in [&["only"][..], &["a", "only"][..]] {
            let work = tempfile::tempdir().unwrap();
            fs::write(work.path().join("out.pdf"), b"sentinel").unwrap();
            let (report, args) = export_release_only(work.path(), kinds, None);
            let failure = report.quire.unwrap();
            assert_eq!(failure.exit_code, Some(3));
            assert!(!args.iter().any(|arg| arg == "--version"));
            assert!(args.iter().any(|arg| arg.ends_with("DOS-ONLY.md")));
            assert_eq!(fs::read(work.path().join("out.pdf")).unwrap(), b"sentinel");
        }
        let work = tempfile::tempdir().unwrap();
        let (report, args) = export_release_only(work.path(), &["only"], Some("v1"));
        assert!(report.quire.is_none());
        assert!(!args.iter().any(|arg| arg == "--version"));
    }
}
