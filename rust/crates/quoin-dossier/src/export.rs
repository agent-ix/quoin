// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The export hand-off to `quire export` (FR-191, "Commit then export",
//! "Export currency").
//!
//! Quoin renders nothing and never bypasses `quire export`: it checks that the
//! committed bytes are current, then runs the installed `quire` as an external
//! process. `quire export` refuses an uncommitted file itself (quire-cli
//! FR-034); quoin passes that refusal through and runs no git.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use crate::engine::{Rendered, render, report_with_comparison};
use crate::error::{DossierError, QuireFailure};
use crate::registry::{Inputs, Registry};
use crate::report::Report;

/// The title of a bundle of several documents.
pub const BUNDLE_TITLE: &str = "Submission Dossier";

/// The id of a bundle of several documents.
pub const BUNDLE_ID: &str = "DOS";

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
    let rendered = render(registry, kinds, out, inputs)?;
    let mut report = report_with_comparison(&rendered);
    if !report.differing().is_empty() {
        return Ok(report);
    }
    report.quire = run_quire(&rendered, options).err();
    Ok(report)
}

/// The arguments of the one `quire export` call, in order.
#[must_use]
pub fn quire_args(documents: &[Rendered], options: &ExportOptions) -> Vec<String> {
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
        "HEAD".to_owned(),
    ]);
    if let Some(release) = &options.release {
        args.extend(["--release".to_owned(), release.clone()]);
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

fn run_quire(documents: &[Rendered], options: &ExportOptions) -> Result<(), QuireFailure> {
    let had_output = fs::symlink_metadata(&options.output).is_ok();
    let result = Command::new(&options.quire)
        .args(quire_args(documents, options))
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
    if result.is_err() && !had_output {
        // quire writes `--output` only on success; this removes a partial one
        // a failed run might still have left, so a refusal leaves nothing.
        let _ = fs::remove_file(&options.output);
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
    use crate::error::SourceFailure;
    use crate::registry::{DocumentSpec, Generated, Generator};
    use ix_trace_rs::trace;

    struct Doc(DocumentSpec);

    impl Generator for Doc {
        fn spec(&self) -> DocumentSpec {
            self.0
        }

        fn generate(&self, _: &Inputs) -> Result<Generated, SourceFailure> {
            Ok(Generated {
                body: String::new(),
                gaps: Vec::new(),
            })
        }
    }

    fn registry() -> Registry {
        let mut registry = Registry::new();
        for (kind, id) in [("a", "DOS-A"), ("b", "DOS-B")] {
            registry
                .register(Box::new(Doc(DocumentSpec {
                    kind,
                    id,
                    title: "T",
                    needs_as_of: false,
                })))
                .unwrap();
        }
        registry
    }

    fn inputs() -> Inputs {
        Inputs {
            scope: PathBuf::from("."),
            modules: Vec::new(),
            clause_bindings: Vec::new(),
            pack: None,
            as_of: None,
            version: "1".to_owned(),
        }
    }

    fn options(quire: &str) -> ExportOptions {
        ExportOptions {
            to: "pdf".to_owned(),
            output: PathBuf::from("out.pdf"),
            release: Some("v1".to_owned()),
            approvals: vec![PathBuf::from("a1.json"), PathBuf::from("a2.json")],
            template: Some(PathBuf::from("t.typ")),
            quire: PathBuf::from(quire),
        }
    }

    fn rendered(kinds: &[&str]) -> Vec<Rendered> {
        let kinds: Vec<String> = kinds.iter().map(|kind| (*kind).to_owned()).collect();
        render(&registry(), &kinds, Path::new("dossier"), &inputs()).unwrap()
    }

    #[trace("FR-191-AC-8")]
    #[test]
    fn a_bundle_passes_files_in_registry_order_with_bundle_arguments_and_options_unchanged() {
        let args = quire_args(&rendered(&["b", "a"]), &options("quire"));
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
                "HEAD",
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
        let args = quire_args(&rendered(&["a"]), &options("quire"));
        assert!(args.contains(&"dossier/DOS-A.md".to_owned()));
        assert!(!args.iter().any(|arg| arg.starts_with("--bundle")));
        assert_eq!(args.iter().filter(|arg| arg.contains("DOS-")).count(), 1);
    }

    #[trace("FR-191-AC-6")]
    #[test]
    fn a_missing_quire_is_reported_naming_the_path_tried() {
        let work = tempfile::tempdir().unwrap();
        let out = work.path().join("dossier");
        crate::engine::generate(&registry(), &[], &out, &inputs()).unwrap();
        let mut options = options("/nonexistent/quire");
        options.output = work.path().join("out.pdf");
        let report = export(&registry(), &[], &out, &inputs(), &options).unwrap();
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
        crate::engine::generate(&registry(), &[], &out, &inputs()).unwrap();
        fs::write(out.join("DOS-B.md"), "edited").unwrap();
        fs::remove_file(out.join("DOS-A.md")).unwrap();
        let mut options = options("/nonexistent/quire");
        options.output = work.path().join("out.pdf");
        let report = export(&registry(), &[], &out, &inputs(), &options).unwrap();
        assert_eq!(report.differing(), ["DOS-A", "DOS-B"]);
        assert!(report.quire.is_none(), "quire was started");
        let elsewhere = export(
            &registry(),
            &[],
            &work.path().join("other"),
            &inputs(),
            &options,
        )
        .unwrap();
        assert_eq!(elsewhere.differing(), ["DOS-A", "DOS-B"]);
    }
}
