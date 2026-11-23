// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The export hand-off to `quire export` (FR-191, "Commit then export",
//! "Export currency").
//!
//! Quoin renders nothing and never bypasses `quire export`: it checks that the
//! committed bytes are current, then runs the installed `quire` as an external
//! process. `quire export` refuses an uncommitted file itself (quire-cli
//! FR-034); quoin passes that refusal through and runs no git.

use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

#[cfg(test)]
use crate::engine::render;
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
    let rendered = render_for(registry, kinds, out, inputs, Operation::Export)?;
    let source_rev = resolve_head(inputs.scope.as_path()).ok();
    let mut report = report_with_comparison(&rendered);
    if let Some(source_rev) = source_rev.as_deref() {
        for (entry, document) in report.documents.iter_mut().zip(&rendered) {
            if committed_blob(inputs.scope.as_path(), source_rev, &document.path)
                .is_none_or(|blob| blob != document.text.as_bytes())
            {
                entry.comparison = Some(crate::report::Comparison::Differs);
            }
        }
    } else {
        for entry in &mut report.documents {
            entry.comparison = Some(crate::report::Comparison::Differs);
        }
    }
    if !report.differing().is_empty() {
        return Ok(report);
    }
    let Some(source_rev) = source_rev else {
        return Ok(report);
    };
    report.quire = run_quire(&rendered, options, &source_rev).err();
    Ok(report)
}

fn git(scope: &Path, args: &[&str]) -> Option<Vec<u8>> {
    let output = Command::new("git")
        .args(args)
        .current_dir(scope)
        .stdin(Stdio::null())
        .output()
        .ok()?;
    output.status.success().then_some(output.stdout)
}

fn resolve_head(scope: &Path) -> Option<String> {
    let output = git(scope, &["rev-parse", "--verify", "HEAD^{commit}"])?;
    let revision = String::from_utf8(output).ok()?;
    let revision = revision.trim();
    (revision.len() == 40 && revision.bytes().all(|byte| byte.is_ascii_hexdigit()))
        .then(|| revision.to_owned())
}

fn committed_blob(scope: &Path, revision: &str, path: &Path) -> Option<Vec<u8>> {
    let scope = scope.canonicalize().ok()?;
    let path = if path.is_absolute() {
        path.canonicalize().ok()?
    } else {
        std::env::current_dir().ok()?.join(path).canonicalize().ok()?
    };
    let relative = path.strip_prefix(scope).ok()?;
    let relative = relative.to_str()?;
    let spec = format!("{revision}:{relative}");
    git(&scope, &["cat-file", "blob", &spec])
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
    let previous_output = fs::read(&options.output).ok();
    let mut result = Command::new(&options.quire)
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
    if let Err(failure) = &mut result {
        if let Some(bytes) = previous_output {
            if let Err(error) = fs::write(&options.output, bytes) {
                if !failure.stderr.is_empty() && !failure.stderr.ends_with('\n') {
                    failure.stderr.push('\n');
                }
                let _ = write!(
                    failure.stderr,
                    "could not restore the pre-existing output {}: {error}",
                    options.output.display()
                );
            }
            return result;
        }
        // quire writes `--output` only on success; remove a partial one a
        // failed run might still have left, so a refusal leaves nothing. A
        // pre-existing output is not ours to remove, and an output that is
        // already absent needs no cleanup. Any other failure is reported
        // beside quire's own.
        match fs::remove_file(&options.output) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => {
                if !failure.stderr.is_empty() && !failure.stderr.ends_with('\n') {
                    failure.stderr.push('\n');
                }
                let _ = write!(
                    failure.stderr,
                    "could not remove the partial output {}: {error}",
                    options.output.display()
                );
            }
        }
    }
    result
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

    fn rendered(kinds: &[&str]) -> Vec<Rendered> {
        let kinds: Vec<String> = kinds.iter().map(|kind| (*kind).to_owned()).collect();
        render(&two(), &kinds, Path::new("dossier"), &inputs()).unwrap()
    }

    fn commit_docs(work: &Path) {
        for args in [
            & ["init", "-q"][..],
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
            assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
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
                "#!/bin/sh\nout=\nwhile [ $# -gt 0 ]; do\n  if [ \"$1\" = --output ]; then out=$2; fi\n  shift\ndone\n{body}\n"
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

    #[trace("FR-191-AC-8")]
    #[test]
    fn a_bundle_passes_files_in_registry_order_with_bundle_arguments_and_options_unchanged() {
        let args = quire_args(&rendered(&["b", "a"]), &options("quire"), "0123456789012345678901234567890123456789");
        assert_eq!(
            args,
            [
                "export",
                "dossier/DOS-A.md",
                "dossier/DOS-B.md",
                "--to",
                "pdf",
                "--output",
                "out.pdf",
                "--source-rev",
                "0123456789012345678901234567890123456789",
                "--release",
                "v1",
                "--approval",
                "a1.json",
                "--approval",
                "a2.json",
                "--template",
                "t.typ",
                "--bundle-title",
                "Submission Dossier",
                "--bundle-id",
                "DOS",
            ]
        );
    }

    #[trace("FR-191-AC-9")]
    #[test]
    fn one_document_is_exported_without_bundle_arguments() {
        let args = quire_args(&rendered(&["a"]), &options("quire"), "0123456789012345678901234567890123456789");
        assert!(args.contains(&"dossier/DOS-A.md".to_owned()));
        assert!(!args.iter().any(|arg| arg.starts_with("--bundle")));
        assert_eq!(args.iter().filter(|arg| arg.contains("DOS-")).count(), 1);
    }

    #[trace("FR-191-AC-8")]
    #[test]
    fn module_roots_are_passed_to_quire_for_document_validation() {
        let mut options = options("quire");
        options.modules = vec![
            PathBuf::from("process-module"),
            PathBuf::from("other-module"),
        ];
        let args = quire_args(&rendered(&["a"]), &options, "0123456789012345678901234567890123456789");
        let module_args: Vec<_> = args
            .windows(2)
            .filter(|pair| pair[0] == "--module")
            .map(|pair| pair[1].as_str())
            .collect();
        assert_eq!(module_args, ["process-module", "other-module"]);
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
        let report = export(&two(), &[], &out, &inputs(), &options).unwrap();
        assert_eq!(report.differing(), ["DOS-A", "DOS-B"]);
        assert!(report.quire.is_none(), "quire was started");
        let elsewhere = export(&two(), &[], &work.path().join("other"), &input, &options).unwrap();
        assert_eq!(elsewhere.differing(), ["DOS-A", "DOS-B"]);
    }

    #[cfg(unix)]
    #[trace("FR-191-AC-26")]
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
    #[trace("FR-191-AC-7")]
    #[test]
    fn an_output_that_existed_before_a_failed_quire_is_preserved() {
        let work = tempfile::tempdir().unwrap();
        fs::write(work.path().join("out.pdf"), b"sentinel bytes\0\xff").unwrap();
        let quire = script(work.path(), "echo partial > \"$out\"; exit 7");
        let failure = export_with(work.path(), &quire).quire.unwrap();
        assert!(!failure.stderr.contains("could not remove"));
        assert_eq!(fs::read(work.path().join("out.pdf")).unwrap(), b"sentinel bytes\0\xff");
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
}
