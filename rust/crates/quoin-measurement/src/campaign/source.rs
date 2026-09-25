// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! Exact clean Git source graph for a generic EA campaign.

use std::collections::BTreeMap;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use engineering_assurance::campaign::{CampaignDefinition, CampaignSource};
use engineering_assurance::producer_execution::{ContentDigest, InputBinding};
use quoin_store::digest_bytes_sha256;
use thiserror::Error;

/// A source checkout does not match the authored source graph.
#[derive(Debug, Error)]
pub enum SourceError {
    /// No checkout was selected for a source repository.
    #[error("missing checkout for campaign source `{0}`")]
    Missing(String),
    /// Git failed to run or returned failure.
    #[error("Git {operation} failed in {path}: {message}")]
    Git {
        /// The operation being checked.
        operation: &'static str,
        /// The selected checkout.
        path: PathBuf,
        /// The failure detail.
        message: String,
    },
    /// The selected checkout has tracked edits or staged changes.
    #[error("campaign source checkout has tracked changes: {0}")]
    Dirty(PathBuf),
    /// `HEAD` is not the exact declared revision.
    #[error("campaign source revision mismatch for {0}")]
    Revision(String),
    /// The raw NUL-terminated Git tree inventory has another digest.
    #[error("campaign source tree digest mismatch for {0}")]
    Tree(String),
    /// A tracked path or Git tree entry is malformed or unsupported.
    #[error("campaign source path is unsafe: {0}")]
    Path(String),
}

/// One selected clean checkout and its exact raw `git ls-tree` inventory.
#[derive(Clone, Debug)]
pub struct VerifiedSource {
    /// Repository name from the campaign definition.
    pub repository: String,
    /// Selected clean checkout root, canonical; the EA capability root.
    pub checkout: PathBuf,
    /// Raw NUL-delimited Git tree inventory, bound to `CampaignSource.digest`.
    pub manifest: Vec<u8>,
}

const MAX_SOURCE_MANIFEST_BYTES: u64 = 8 * 1024 * 1024;
const MAX_SOURCE_FILE_BYTES: u64 = 64 * 1024 * 1024;

impl VerifiedSource {
    /// Whether a selected input path would traverse a Git-tracked source link.
    #[must_use]
    pub fn has_link_ancestor(&self, path: &str) -> bool {
        self.manifest.split(|byte| *byte == 0).any(|record| {
            let Some(tab) = record.iter().position(|byte| *byte == b'\t') else {
                return false;
            };
            let Some(header) = record.get(..tab) else {
                return false;
            };
            if !header.starts_with(b"120000 blob ") {
                return false;
            }
            let Some(raw) = record.get(tab.saturating_add(1)..) else {
                return false;
            };
            std::str::from_utf8(raw)
                .is_ok_and(|link| path == link || path.starts_with(&format!("{link}/")))
        })
    }

    /// Tracked regular files in the exact Git tree, excluding omitted links.
    ///
    /// # Errors
    /// Refuses a malformed manifest entry.
    pub fn tracked_regular_paths(&self) -> Result<Vec<String>, SourceError> {
        let records = self
            .manifest
            .strip_suffix(&[0])
            .ok_or_else(|| SourceError::Path("unterminated Git tree inventory".to_owned()))?;
        let mut paths = Vec::new();
        for record in records.split(|byte| *byte == 0) {
            let tab = record
                .iter()
                .position(|byte| *byte == b'\t')
                .ok_or_else(|| SourceError::Path("missing Git tree separator".to_owned()))?;
            let header = std::str::from_utf8(record.get(..tab).unwrap_or_default())
                .map_err(|_| SourceError::Path("invalid Git header".to_owned()))?;
            let mut tokens = header.split(' ');
            let (Some(mode), Some("blob"), Some(_oid), None) =
                (tokens.next(), tokens.next(), tokens.next(), tokens.next())
            else {
                return Err(SourceError::Path("unsupported Git tree entry".to_owned()));
            };
            if !matches!(mode, "100644" | "100755" | "120000") {
                return Err(SourceError::Path("unsupported Git mode".to_owned()));
            }
            if mode == "120000" {
                continue;
            }
            let raw_path = record
                .get(tab.saturating_add(1)..)
                .ok_or_else(|| SourceError::Path("invalid Git tree separator".to_owned()))?;
            let path = std::str::from_utf8(raw_path)
                .map_err(|_| SourceError::Path("non-UTF-8 Git path".to_owned()))?;
            paths.push(path.to_owned());
        }
        Ok(paths)
    }

    /// Exact EA source input population implied by this Git tree.
    ///
    /// # Errors
    /// Refuses unsupported tree entries and unavailable exact Git blobs.
    pub fn input_inventory(&self) -> Result<Vec<InputBinding>, SourceError> {
        let records = self
            .manifest
            .strip_suffix(&[0])
            .ok_or_else(|| SourceError::Path("unterminated Git tree inventory".to_owned()))?;
        let mut inventory = Vec::new();
        for record in records.split(|byte| *byte == 0) {
            let tab = record
                .iter()
                .position(|byte| *byte == b'\t')
                .ok_or_else(|| SourceError::Path("missing Git tree separator".to_owned()))?;
            let header = std::str::from_utf8(record.get(..tab).unwrap_or_default())
                .map_err(|_| SourceError::Path("invalid Git header".to_owned()))?;
            let mut tokens = header.split(' ');
            let (Some(mode), Some("blob"), Some(_oid), None) =
                (tokens.next(), tokens.next(), tokens.next(), tokens.next())
            else {
                return Err(SourceError::Path("unsupported Git tree entry".to_owned()));
            };
            if !matches!(mode, "100644" | "100755" | "120000") {
                return Err(SourceError::Path("unsupported Git mode".to_owned()));
            }
            let raw_path = record
                .get(tab.saturating_add(1)..)
                .ok_or_else(|| SourceError::Path("invalid Git tree separator".to_owned()))?;
            let path = std::str::from_utf8(raw_path)
                .map_err(|_| SourceError::Path("non-UTF-8 Git path".to_owned()))?;
            if mode == "120000" {
                continue;
            }
            let bytes = self.read_tracked_file(path)?;
            inventory.push(InputBinding {
                role: format!(
                    "{}/{path}",
                    if mode == "100755" {
                        "source-exec"
                    } else {
                        "source"
                    }
                ),
                path: path.to_owned(),
                digest: ContentDigest::of_bytes(&bytes),
                executable: mode == "100755",
            });
        }
        Ok(inventory)
    }
    /// Read an exact tracked regular blob from the verified Git object graph.
    ///
    /// # Errors
    /// Refuses an absent path or an oversized/unreadable Git blob.
    pub fn read_tracked_file(&self, path: &str) -> Result<Vec<u8>, SourceError> {
        for record in self.manifest.split(|byte| *byte == 0) {
            let Some(tab) = record.iter().position(|byte| *byte == b'\t') else {
                continue;
            };
            let Some(raw_path) = record.get(tab.saturating_add(1)..) else {
                continue;
            };
            if raw_path != path.as_bytes() {
                continue;
            }
            let header = std::str::from_utf8(record.get(..tab).unwrap_or_default())
                .map_err(|_| SourceError::Path(path.to_owned()))?;
            let mut tokens = header.split(' ');
            let (Some(mode), Some("blob"), Some(oid), None) =
                (tokens.next(), tokens.next(), tokens.next(), tokens.next())
            else {
                return Err(SourceError::Path(path.to_owned()));
            };
            if !matches!(mode, "100644" | "100755") {
                return Err(SourceError::Path(path.to_owned()));
            }
            return git(
                &self.checkout,
                "cat-file",
                &["cat-file", "blob", oid],
                MAX_SOURCE_FILE_BYTES,
            );
        }
        Err(SourceError::Path(path.to_owned()))
    }

    /// Whether a normal repository-relative file is in the exact Git tree.
    #[must_use]
    pub fn contains_path(&self, path: &str) -> bool {
        self.manifest.split(|byte| *byte == 0).any(|record| {
            record
                .iter()
                .position(|byte| *byte == b'\t')
                .and_then(|tab| record.get(tab.saturating_add(1)..))
                .and_then(|raw| std::str::from_utf8(raw).ok())
                == Some(path)
        })
    }
}

/// Verify every source against a selected checkout before execution or replay.
///
/// Untracked evidence files are not part of the source tree;
/// tracked edits and staged changes always refuse. The manifest digest uses
/// the raw `git ls-tree -r -z --full-tree` bytes, not a text rendering.
///
/// # Errors
/// Refuses missing/dirty/wrong-revision checkouts or changed Git tree bytes.
pub fn verify_source_graph(
    definition: &CampaignDefinition,
    checkouts: &BTreeMap<String, PathBuf>,
) -> Result<BTreeMap<String, VerifiedSource>, SourceError> {
    definition
        .source_graph
        .iter()
        .map(|source| {
            let path = checkouts
                .get(&source.repository)
                .ok_or_else(|| SourceError::Missing(source.repository.clone()))?;
            let verified = verify_source(source, path)?;
            Ok((source.repository.clone(), verified))
        })
        .collect()
}

fn git(
    path: &Path,
    operation: &'static str,
    args: &[&str],
    max_bytes: u64,
) -> Result<Vec<u8>, SourceError> {
    let mut child = Command::new("git")
        .arg("-C")
        .arg(path)
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| SourceError::Git {
            operation,
            path: path.to_path_buf(),
            message: error.to_string(),
        })?;
    let stdout = child.stdout.take().ok_or_else(|| SourceError::Git {
        operation,
        path: path.to_path_buf(),
        message: "stdout pipe unavailable".to_owned(),
    })?;
    let stderr = child.stderr.take().ok_or_else(|| SourceError::Git {
        operation,
        path: path.to_path_buf(),
        message: "stderr pipe unavailable".to_owned(),
    })?;
    let stderr_reader = std::thread::spawn(move || {
        let mut stderr = stderr;
        let mut bytes = Vec::new();
        let mut overflow = false;
        let mut chunk = [0_u8; 8192];
        let outcome = loop {
            match stderr.read(&mut chunk) {
                Ok(0) => break Ok(()),
                Ok(size) => {
                    let room = (64_usize * 1024 + 1).saturating_sub(bytes.len());
                    if let Some(selected) = chunk.get(..size.min(room)) {
                        bytes.extend_from_slice(selected);
                    }
                    overflow |= size > room;
                }
                Err(error) => break Err(error),
            }
        };
        (outcome, bytes, overflow)
    });
    let mut output = Vec::new();
    let read = stdout
        .take(max_bytes.saturating_add(1))
        .read_to_end(&mut output);
    if output.len() as u64 > max_bytes || read.is_err() {
        let _ = child.kill();
    }
    let status = child.wait().map_err(|error| SourceError::Git {
        operation,
        path: path.to_path_buf(),
        message: error.to_string(),
    })?;
    let (stderr_status, stderr, stderr_overflow) =
        stderr_reader.join().map_err(|_| SourceError::Git {
            operation,
            path: path.to_path_buf(),
            message: "stderr reader failed".to_owned(),
        })?;
    if read.is_err()
        || stderr_status.is_err()
        || output.len() as u64 > max_bytes
        || stderr_overflow
        || stderr.len() > 64 * 1024
    {
        return Err(SourceError::Git {
            operation,
            path: path.to_path_buf(),
            message: "Git output exceeds bounded capture".to_owned(),
        });
    }
    if !status.success() {
        return Err(SourceError::Git {
            operation,
            path: path.to_path_buf(),
            message: String::from_utf8_lossy(&stderr).trim().to_owned(),
        });
    }
    Ok(output)
}

fn verify_source(source: &CampaignSource, path: &Path) -> Result<VerifiedSource, SourceError> {
    let head = git(path, "rev-parse", &["rev-parse", "--verify", "HEAD"], 1024)?;
    if String::from_utf8_lossy(&head).trim() != source.revision {
        return Err(SourceError::Revision(source.repository.clone()));
    }
    let status = git(
        path,
        "status",
        &["status", "--porcelain", "--untracked-files=no"],
        MAX_SOURCE_MANIFEST_BYTES,
    )?;
    if !status.is_empty() {
        return Err(SourceError::Dirty(path.to_path_buf()));
    }
    let manifest = git(
        path,
        "ls-tree",
        &["ls-tree", "-r", "-z", "--full-tree", &source.revision],
        MAX_SOURCE_MANIFEST_BYTES,
    )?;
    if digest_bytes_sha256(&manifest).as_hex() != source.digest {
        return Err(SourceError::Tree(source.repository.clone()));
    }
    // EA opens the capability root through a symlink-free absolute path.
    let checkout = path.canonicalize().map_err(|error| SourceError::Git {
        operation: "canonicalize",
        path: path.to_path_buf(),
        message: error.to_string(),
    })?;
    Ok(VerifiedSource {
        repository: source.repository.clone(),
        checkout,
        manifest,
    })
}
