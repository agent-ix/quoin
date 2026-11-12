// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! `statusHistory`, derived from the first-parent git history of one file.
//!
//! Nothing here reads an author or a date typed into the document: who moved
//! the status and when is what git recorded.

use std::path::Path;
use std::process::Command;

use serde::Serialize;

use crate::error::AnomalyError;
use crate::record::{assessment_text, status_word};

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
    pub status: String,
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

fn git(dir: &Path, args: &[&str]) -> Result<std::process::Output, AnomalyError> {
    Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(args)
        .output()
        .map_err(|error| AnomalyError::git(format!("git {}: {error}", args.join(" "))))
}

fn git_text(dir: &Path, args: &[&str]) -> Option<String> {
    let output = git(dir, args).ok()?;
    output
        .status
        .success()
        .then(|| String::from_utf8_lossy(&output.stdout).into_owned())
}

/// The ref whose first-parent history is read: the remote default branch, then
/// `main`, then `master`, then `HEAD`.
fn default_branch(dir: &Path) -> String {
    if let Some(head) = git_text(
        dir,
        &[
            "symbolic-ref",
            "--quiet",
            "--short",
            "refs/remotes/origin/HEAD",
        ],
    ) {
        return head.trim().to_owned();
    }
    for name in ["main", "master"] {
        if git_text(dir, &["rev-parse", "--verify", "--quiet", name]).is_some() {
            return name.to_owned();
        }
    }
    "HEAD".to_owned()
}

/// Derive `statusHistory` for `file`.
///
/// Returns [`History::NotApplicable`] when the file is not inside a git
/// repository. The working-tree pending entry is added by the caller, which
/// holds the working-tree record.
///
/// # Errors
///
/// Returns an error when `git` cannot be run.
pub fn read_history(file: &Path) -> Result<History, AnomalyError> {
    let dir = file.parent().unwrap_or_else(|| Path::new("."));
    if git_text(dir, &["rev-parse", "--is-inside-work-tree"]).is_none() {
        return Ok(History::NotApplicable);
    }
    let Some(top) = git_text(dir, &["rev-parse", "--show-toplevel"]) else {
        return Ok(History::NotApplicable);
    };
    let top = std::path::PathBuf::from(top.trim());
    let canonical = file.canonicalize().unwrap_or_else(|_| file.to_path_buf());
    let top_canonical = top.canonicalize().unwrap_or(top);
    let Ok(relative) = canonical.strip_prefix(&top_canonical) else {
        return Ok(History::NotApplicable);
    };
    let relative = relative.to_string_lossy().replace('\\', "/");
    let branch = default_branch(dir);
    let log = git_text(
        dir,
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
    )
    .unwrap_or_default();
    let mut commits = log
        .split('\u{1e}')
        .filter(|block| !block.trim().is_empty())
        .filter_map(parse_block)
        .collect::<Vec<_>>();
    commits.reverse();
    let mut entries = Vec::new();
    let mut assessments = Vec::new();
    let mut previous: Option<String> = None;
    for commit in commits {
        let spec = format!("{}:{}", commit.hash, commit.path);
        let Some(content) = git_text(dir, &["show", &spec]) else {
            continue;
        };
        let Some(status) = status_word(&content) else {
            continue;
        };
        if previous.as_deref() == Some(status.as_str()) {
            continue;
        }
        previous = Some(status.clone());
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
