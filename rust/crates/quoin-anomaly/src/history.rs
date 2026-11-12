// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! `statusHistory`, derived from the first-parent git history of one file.
//!
//! Nothing here reads an author or a date typed into the document: who moved
//! the status and when is what git recorded.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use serde::Serialize;

use crate::error::AnomalyError;
use crate::record::{assessment_text, status_word};
use crate::status::Status;

/// What git reported about a commit's signature, reduced to three answers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Signature {
    /// `G`, `U`, `X` or `Y`: a valid signature is present; trust and expiry
    /// are not judged here.
    Signed,
    /// `N`: no signature.
    Unsigned,
    /// `B`, `R`, `E` or anything else: a bad, revoked-key or uncheckable
    /// signature is not treated as signed.
    Unknown,
}

impl Signature {
    /// Map the letter `git log --format=%G?` reports.
    #[must_use]
    pub fn from_code(code: char) -> Self {
        match code {
            'G' | 'U' | 'X' | 'Y' => Self::Signed,
            'N' => Self::Unsigned,
            _ => Self::Unknown,
        }
    }
}

/// One commit that changed the value of `status`, or the pending working tree.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StatusEntry {
    /// The status the commit set.
    pub status: Status,
    /// The commit; absent for the pending working-tree entry.
    pub commit: Option<String>,
    /// The commit author's name.
    pub author: Option<String>,
    /// The author date, ISO 8601.
    pub authored_at: Option<String>,
    /// The reduced signature answer.
    pub signature: Option<Signature>,
    /// The letter git reported.
    pub signature_code: Option<char>,
    /// True for the working-tree entry that is not yet committed.
    pub pending: bool,
}

/// The derived history of one document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum History {
    /// The file is not under git.
    NotApplicable,
    /// The entries in commit order, oldest first, with the assessment-section
    /// text of the document at each entry.
    Entries {
        /// The status changes.
        entries: Vec<StatusEntry>,
        /// The assessment section text at each entry, same order.
        assessments: Vec<Option<String>>,
    },
}

fn run_git(dir: &Path, args: &[&str]) -> Result<Output, AnomalyError> {
    Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(args)
        .output()
        .map_err(|error| AnomalyError::git(format!("cannot run `git {}`: {error}", args.join(" "))))
}

/// Run git and return stdout when it succeeds, `None` when it answers with a
/// failure status (an expected answer for a probe), and an error only when git
/// cannot be run at all.
fn probe(dir: &Path, args: &[&str]) -> Result<Option<String>, AnomalyError> {
    let output = run_git(dir, args)?;
    Ok(output
        .status
        .success()
        .then(|| String::from_utf8_lossy(&output.stdout).into_owned()))
}

/// Run git where a failure status is a real failure.
fn require(dir: &Path, args: &[&str]) -> Result<String, AnomalyError> {
    let output = run_git(dir, args)?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).into_owned())
    } else {
        Err(AnomalyError::git(format!(
            "`git {}` failed: {}",
            args.join(" "),
            String::from_utf8_lossy(&output.stderr).trim()
        )))
    }
}

/// The ref whose first-parent history is read: the remote default branch, then
/// `main`, then `master`, then `HEAD`.
fn default_branch(top: &Path) -> Result<String, AnomalyError> {
    if let Some(head) = probe(
        top,
        &[
            "symbolic-ref",
            "--quiet",
            "--short",
            "refs/remotes/origin/HEAD",
        ],
    )? {
        return Ok(head.trim().to_owned());
    }
    for name in ["main", "master"] {
        if probe(top, &["rev-parse", "--verify", "--quiet", name])?.is_some() {
            return Ok(name.to_owned());
        }
    }
    Ok("HEAD".to_owned())
}

/// Derive `statusHistory` for `file`.
///
/// Returns [`History::NotApplicable`] when the file is not inside a git
/// repository. The working-tree pending entry is added by the caller, which
/// holds the working-tree record.
///
/// # Errors
///
/// Returns an error when `git` cannot be run, or when it fails to answer a
/// history query for a file that is inside a repository.
pub fn read_history(file: &Path) -> Result<History, AnomalyError> {
    let dir = file.parent().unwrap_or_else(|| Path::new("."));
    let dir = if dir.as_os_str().is_empty() {
        Path::new(".")
    } else {
        dir
    };
    let Some(top) = probe(dir, &["rev-parse", "--show-toplevel"])? else {
        return Ok(History::NotApplicable);
    };
    let top = PathBuf::from(top.trim());
    let canonical = file.canonicalize().unwrap_or_else(|_| file.to_path_buf());
    let top_canonical = top.canonicalize().unwrap_or_else(|_| top.clone());
    let Ok(relative) = canonical.strip_prefix(&top_canonical) else {
        return Ok(History::NotApplicable);
    };
    let relative = relative.to_string_lossy().replace('\\', "/");
    // Every query runs from the top level so that `relative`, which is relative
    // to it, is also what git resolves the pathspec against.
    if probe(&top, &["rev-parse", "--verify", "--quiet", "HEAD"])?.is_none() {
        return Ok(History::Entries {
            entries: Vec::new(),
            assessments: Vec::new(),
        });
    }
    let branch = default_branch(&top)?;
    let log = require(
        &top,
        &[
            "log",
            &branch,
            "--first-parent",
            "--follow",
            "--name-only",
            "--format=%x1e%H%x1f%an%x1f%aI%x1f%G?",
            "--",
            &relative,
        ],
    )?;
    let mut commits = log
        .split('\u{1e}')
        .filter(|block| !block.trim().is_empty())
        .filter_map(parse_block)
        .collect::<Vec<_>>();
    commits.reverse();
    let mut entries = Vec::new();
    let mut assessments = Vec::new();
    let mut previous: Option<Status> = None;
    for commit in commits {
        let spec = format!("{}:{}", commit.hash, commit.path);
        // A commit that deleted the file has no content to read.
        let Some(content) = probe(&top, &["show", &spec])? else {
            continue;
        };
        let Some(status) = status_word(&content).as_deref().and_then(Status::parse) else {
            continue;
        };
        if previous == Some(status) {
            continue;
        }
        previous = Some(status);
        let code = commit.signature_code;
        entries.push(StatusEntry {
            status,
            commit: Some(commit.hash),
            author: Some(commit.author),
            authored_at: Some(commit.authored_at),
            signature: Some(Signature::from_code(code)),
            signature_code: Some(code),
            pending: false,
        });
        assessments.push(assessment_text(&content));
    }
    Ok(History::Entries {
        entries,
        assessments,
    })
}

struct Commit {
    hash: String,
    author: String,
    authored_at: String,
    signature_code: char,
    path: String,
}

fn parse_block(block: &str) -> Option<Commit> {
    let (header, rest) = block.split_once('\n').unwrap_or((block, ""));
    let mut fields = header.split('\u{1f}');
    let hash = fields.next()?.trim().to_owned();
    let author = fields.next()?.to_owned();
    let authored_at = fields.next()?.to_owned();
    let signature_code = fields.next()?.trim().chars().next().unwrap_or('N');
    let path = rest
        .lines()
        .find(|line| !line.trim().is_empty())?
        .to_owned();
    Some(Commit {
        hash,
        author,
        authored_at,
        signature_code,
        path,
    })
}
