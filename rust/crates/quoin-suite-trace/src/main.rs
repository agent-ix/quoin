// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! `quoin-suite-trace` adds each test's trace tags to a cargo-nextest `JUnit`
//! report, so `quoin evidence record --adapter junit` binds the run to the
//! acceptance criteria the tests actually exercised.
//!
//! The tags are read by the quire engine that defines their grammar; this tool
//! parses no source comments itself. It runs no test and records nothing.

mod annotate;
mod error;
mod targets;

use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

use quoin_quire::coverage::{self, Request};
use quoin_quire::{ModuleRoot, ModuleSelection, ScopeRoot};

use crate::annotate::{TraceIndex, annotate};
use crate::error::Error;

const USAGE: &str = "usage: quoin-suite-trace --repo <dir> --junit <nextest-junit.xml> --out <annotated.xml> [--manifest-path <Cargo.toml>] [--module <dir>]";

fn main() -> ExitCode {
    match run(std::env::args().skip(1).collect()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("quoin-suite-trace: {error}");
            ExitCode::FAILURE
        }
    }
}

/// The parsed command line.
struct Args {
    repo: PathBuf,
    junit: PathBuf,
    out: PathBuf,
    manifest: Option<PathBuf>,
    module: Option<PathBuf>,
}

fn parse(args: Vec<String>) -> Result<Args, Error> {
    let mut repo = None;
    let mut junit = None;
    let mut out = None;
    let mut manifest = None;
    let mut module = None;
    let mut rest = args.into_iter();
    while let Some(flag) = rest.next() {
        let slot = match flag.as_str() {
            "--repo" => &mut repo,
            "--junit" => &mut junit,
            "--out" => &mut out,
            "--manifest-path" => &mut manifest,
            "--module" => &mut module,
            other => return Err(Error::Usage(format!("unknown argument `{other}`; {USAGE}"))),
        };
        *slot = Some(PathBuf::from(rest.next().ok_or_else(|| {
            Error::Usage(format!("`{flag}` needs a value; {USAGE}"))
        })?));
    }
    let need = |value: Option<PathBuf>, name: &str| {
        value.ok_or_else(|| Error::Usage(format!("`{name}` is required; {USAGE}")))
    };
    Ok(Args {
        repo: need(repo, "--repo")?,
        junit: need(junit, "--junit")?,
        out: need(out, "--out")?,
        manifest,
        module,
    })
}

fn run(args: Vec<String>) -> Result<(), Error> {
    let args = parse(args)?;
    let repo = args
        .repo
        .canonicalize()
        .map_err(|error| io(&args.repo, &error))?;
    let manifest = args
        .manifest
        .unwrap_or_else(|| repo.join("rust").join("Cargo.toml"));

    let xml = std::fs::read_to_string(&args.junit).map_err(|error| io(&args.junit, &error))?;
    let targets = targets::from_metadata(&metadata(&manifest)?, &repo)?;

    let modules = match &args.module {
        Some(dir) => ModuleSelection::Closed(vec![
            ModuleRoot::open(dir).map_err(|error| Error::Engine(error.to_string()))?,
        ]),
        None => ModuleSelection::ScopeOrAmbient,
    };
    let request = Request {
        scope: ScopeRoot::open(&repo).map_err(|error| Error::Engine(error.to_string()))?,
        modules,
    };
    let (traces, notices) =
        coverage::test_traces(&request).map_err(|error| Error::Engine(error.to_string()))?;
    for notice in notices {
        eprintln!("quoin-suite-trace: notice: {}", notice.message);
    }

    let index = TraceIndex::build(&traces, &targets);
    let done = annotate(&xml, &index);
    if done.tagged == 0 {
        return Err(Error::NothingMatched {
            tagged: index.len(),
            cases: done.cases,
        });
    }
    std::fs::write(&args.out, &done.xml).map_err(|error| io(&args.out, &error))?;
    eprintln!(
        "quoin-suite-trace: {} of {} test cases carry a trace tag; {} tagged tests were not in this run",
        done.tagged,
        done.cases,
        done.unrun.len()
    );
    Ok(())
}

fn metadata(manifest: &Path) -> Result<String, Error> {
    let output = Command::new("cargo")
        .args([
            "metadata",
            "--no-deps",
            "--format-version",
            "1",
            "--locked",
            "--manifest-path",
        ])
        .arg(manifest)
        .output()
        .map_err(|error| Error::Metadata(error.to_string()))?;
    if !output.status.success() {
        return Err(Error::Metadata(
            String::from_utf8_lossy(&output.stderr).trim().to_owned(),
        ));
    }
    String::from_utf8(output.stdout).map_err(|error| Error::Metadata(error.to_string()))
}

fn io(path: &Path, error: &std::io::Error) -> Error {
    Error::Io {
        path: path.to_path_buf(),
        reason: error.to_string(),
    }
}
