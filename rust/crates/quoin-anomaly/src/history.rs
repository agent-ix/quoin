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
use crate::record::{assessment_text, frontmatter, status_word};
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

/// The entry for a working-tree `status` that no commit has set yet.
pub(crate) fn pending_entry(status: Status) -> StatusEntry {
    StatusEntry {
        status,
        commit: None,
        author: None,
        authored_at: None,
        signature: None,
        signature_code: None,
        pending: true,
    }
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

/// One first-parent commit that touched the file, with the path the file had
/// in it.
pub(crate) struct Commit {
    pub(crate) hash: String,
    pub(crate) author: String,
    pub(crate) authored_at: String,
    pub(crate) signature_code: char,
    pub(crate) path: String,
    /// Whether the commit created the file (`A`, or a copy `C`): the history
    /// of the document starts here.
    pub(crate) created: bool,
    /// The path the file had before this commit, when git reports a rename.
    pub(crate) renamed_from: Option<String>,
}

/// The first-parent history of a file on the default branch, read once: the
/// commits and the file's text at each.
pub(crate) struct Located {
    /// Oldest first; empty while the repository has no commit.
    pub(crate) commits: Vec<Commit>,
    /// The file's text at each commit, same order; `None` where that commit
    /// deleted it.
    pub(crate) texts: Vec<Option<String>>,
}

/// Find the first-parent history of `file`, following renames. `None` when the
/// file is not inside a git repository.
///
/// `git log --follow` also follows *copies*: a new anomaly file that resembles
/// an existing one would inherit that file's history and so its status moves,
/// its fixing change and its rationale change. The walk therefore stops at the
/// commit that created the file (`A`, or the copy `C` git reports for it) and
/// continues only across a rename (`R`).
pub(crate) fn locate(file: &Path) -> Result<Option<Located>, AnomalyError> {
    let dir = file.parent().unwrap_or_else(|| Path::new("."));
    let dir = if dir.as_os_str().is_empty() {
        Path::new(".")
    } else {
        dir
    };
    let Some(top) = probe(dir, &["rev-parse", "--show-toplevel"])? else {
        return Ok(None);
    };
    let top = PathBuf::from(top.trim());
    let canonical = file.canonicalize().unwrap_or_else(|_| file.to_path_buf());
    let top_canonical = top.canonicalize().unwrap_or_else(|_| top.clone());
    let Ok(relative) = canonical.strip_prefix(&top_canonical) else {
        return Ok(None);
    };
    let relative = relative.to_string_lossy().replace('\\', "/");
    if probe(&top, &["rev-parse", "--verify", "--quiet", "HEAD"])?.is_none() {
        return Ok(Some(Located {
            commits: Vec::new(),
            texts: Vec::new(),
        }));
    }
    let branch = default_branch(&top)?;
    let log = require(
        &top,
        &[
            "log",
            &branch,
            "--first-parent",
            "--follow",
            "--name-status",
            "--format=%x1e%H%x1f%an%x1f%aI%x1f%G?",
            "--",
            &relative,
        ],
    )?;
    // Newest first: keep commits down to and including the one that created
    // the file.
    let mut commits = Vec::new();
    for commit in log
        .split('\u{1e}')
        .filter(|block| !block.trim().is_empty())
        .filter_map(parse_block)
    {
        // `--follow` pairs files by similarity within the path-limited diff, so a
        // commit that moves one anomaly and adds a similar one can report the
        // moved file's old path as renamed into the new one. A rename is the
        // same document only when its `id` is the same on both sides.
        let created = commit.created
            || match &commit.renamed_from {
                Some(old) => !same_document(&top, &commit, old)?,
                None => false,
            };
        commits.push(commit);
        if created {
            break;
        }
    }
    commits.reverse();
    let mut texts = Vec::with_capacity(commits.len());
    for commit in &commits {
        // A commit that deleted the file has no content to read.
        texts.push(probe(
            &top,
            &["show", &format!("{}:{}", commit.hash, commit.path)],
        )?);
    }
    Ok(Some(Located { commits, texts }))
}

/// Whether the file at `commit` and `old` in its first parent carry one `id`.
fn same_document(top: &Path, commit: &Commit, old: &str) -> Result<bool, AnomalyError> {
    let id_of = |spec: String| -> Result<Option<String>, AnomalyError> {
        Ok(probe(top, &["show", &spec])?
            .and_then(|text| frontmatter(&text))
            .and_then(|front| front.get("id").and_then(|v| v.as_str().map(str::to_owned))))
    };
    let after = id_of(format!("{}:{}", commit.hash, commit.path))?;
    let before = id_of(format!("{}^1:{old}", commit.hash))?;
    Ok(after.is_some() && after == before)
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
    Ok(history_of(locate(file)?.as_ref()))
}

/// `statusHistory` from an already-read history.
pub(crate) fn history_of(located: Option<&Located>) -> History {
    let Some(located) = located else {
        return History::NotApplicable;
    };
    let mut entries = Vec::new();
    let mut assessments = Vec::new();
    let mut previous: Option<Status> = None;
    for (commit, text) in located.commits.iter().zip(&located.texts) {
        let Some(content) = text else { continue };
        let Some(status) = status_word(content).as_deref().and_then(Status::parse) else {
            continue;
        };
        if previous == Some(status) {
            continue;
        }
        previous = Some(status);
        let code = commit.signature_code;
        entries.push(StatusEntry {
            status,
            commit: Some(commit.hash.clone()),
            author: Some(commit.author.clone()),
            authored_at: Some(commit.authored_at.clone()),
            signature: Some(Signature::from_code(code)),
            signature_code: Some(code),
            pending: false,
        });
        assessments.push(assessment_text(content));
    }
    History::Entries {
        entries,
        assessments,
    }
}

fn parse_block(block: &str) -> Option<Commit> {
    let (header, rest) = block.split_once('\n').unwrap_or((block, ""));
    let mut fields = header.split('\u{1f}');
    let hash = fields.next()?.trim().to_owned();
    let author = fields.next()?.to_owned();
    let authored_at = fields.next()?.to_owned();
    let signature_code = fields.next()?.trim().chars().next().unwrap_or('N');
    // `<status>\t<path>` or, for a rename or copy, `<status>\t<from>\t<to>`.
    let entry = rest.lines().find(|line| !line.trim().is_empty())?;
    let mut columns = entry.split('\t');
    let status = columns.next()?;
    let path = columns.next_back()?.to_owned();
    let renamed_from = if status.starts_with('R') {
        columns.next().map(str::to_owned)
    } else {
        None
    };
    Some(Commit {
        hash,
        author,
        authored_at,
        signature_code,
        path,
        created: status.starts_with(['A', 'C']),
        renamed_from,
    })
}
