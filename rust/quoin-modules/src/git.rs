// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Git-backed source resolution with `gix` (quoin#381, FR-019).
//!
//! # Why a bare repository and a tree extraction
//!
//! The TypeScript oracle shells out to `git`: `clone --filter=blob:none
//! --no-checkout`, then `sparse-checkout set <subdir>`, then `checkout
//! --detach <rev>`, then `rev-parse HEAD`. That sequence exists to get one
//! subtree onto disk using only the porcelain.
//!
//! With `gix` in-process there is no porcelain to work around. This module
//! keeps a **bare** repository per remote and extracts the wanted subtree
//! straight from the object database into a content directory keyed by commit
//! id. No worktree, no index, no sparse-checkout state, and the extraction is
//! idempotent and cacheable by `(url, sha, subdir)`.
//!
//! That is also why [`IxHome::cache_root`](crate::paths::IxHome::cache_root)
//! does not point at `~/.ix/cache/ts-plugin-kit`: the two layouts are
//! incompatible, and sharing a directory during staged coexistence would break
//! whichever implementation ran second.
//!
//! # Blocking, deliberately
//!
//! `gix`'s mainline API is blocking, and `quoin-core` is a one-shot subprocess:
//! a reconcile fetches its remotes and exits. There is nothing to overlap, so
//! an async runtime would buy a `block_on` bridge and nothing else. The
//! consequence a blocking fetch must not have — hanging forever on an
//! unresponsive remote — is handled explicitly by [`Deadline`], a watchdog
//! thread that raises `gix`'s interrupt flag when the budget expires. Bounds
//! here are wall-clock and byte ceilings, not cancellation.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use crate::error::ModulesError;
use crate::ids::CommitSha;

/// Resource and time ceilings for a git resolution.
///
/// Every field is a deliberate refusal point, not a tuning knob: exceeding one
/// is [`ModulesError::GitTimeout`] or [`ModulesError::ResourceBoundExceeded`],
/// never a truncated success.
#[derive(Debug, Clone, Copy)]
pub struct GitLimits {
    /// Wall-clock budget for one fetch, including connection setup.
    ///
    /// Default 300s: the largest module in the default set is a few MiB, and a
    /// cold clone over a slow link is the case this must not fail. It is a
    /// hang-breaker, not a latency target.
    pub fetch_budget: Duration,
    /// Maximum total bytes written when extracting one tree.
    ///
    /// Default 256 MiB. A spec module is data files; anything near this is a
    /// wrong revision or a hostile repository.
    pub max_extracted_bytes: u64,
    /// Maximum number of files written when extracting one tree.
    pub max_extracted_files: u64,
    /// Maximum directory nesting depth in an extracted tree.
    ///
    /// The tree walk is recursive and the tree's shape is remote-supplied, so
    /// the depth ceiling is what keeps a hostile repository from exhausting the
    /// stack.
    pub max_tree_depth: usize,
}

impl Default for GitLimits {
    fn default() -> Self {
        Self {
            fetch_budget: Duration::from_secs(300),
            max_extracted_bytes: 256 << 20,
            max_extracted_files: 200_000,
            max_tree_depth: 64,
        }
    }
}

/// A wall-clock budget backed by `gix`'s cooperative interrupt flag.
///
/// `gix`'s blocking fetch polls `should_interrupt` between packets, so raising
/// it from a watchdog thread turns an unbounded network wait into a bounded
/// one. The watchdog is joined on drop so no thread outlives the operation.
struct Deadline {
    flag: Arc<AtomicBool>,
    started: Instant,
    budget: Duration,
    watchdog: Option<std::thread::JoinHandle<()>>,
    done: Arc<AtomicBool>,
}

impl Deadline {
    fn start(budget: Duration) -> Self {
        let flag = Arc::new(AtomicBool::new(false));
        let done = Arc::new(AtomicBool::new(false));
        let watchdog = {
            let flag = Arc::clone(&flag);
            let done = Arc::clone(&done);
            std::thread::Builder::new()
                .name("quoin-modules-fetch-deadline".to_owned())
                .spawn(move || {
                    let started = Instant::now();
                    // Poll rather than sleep for the whole budget, so a fetch
                    // that finishes early releases the thread promptly.
                    while started.elapsed() < budget {
                        if done.load(Ordering::Acquire) {
                            return;
                        }
                        std::thread::sleep(Duration::from_millis(100));
                    }
                    flag.store(true, Ordering::Release);
                })
                .ok()
        };
        Self {
            flag,
            started: Instant::now(),
            budget,
            watchdog,
            done,
        }
    }

    fn flag(&self) -> &AtomicBool {
        &self.flag
    }

    /// Whether the budget was what stopped the operation.
    fn expired(&self) -> bool {
        self.flag.load(Ordering::Acquire) || self.started.elapsed() >= self.budget
    }
}

impl Drop for Deadline {
    fn drop(&mut self) {
        self.done.store(true, Ordering::Release);
        if let Some(handle) = self.watchdog.take() {
            // Joining keeps the watchdog from outliving the fetch it guards.
            let _ = handle.join();
        }
    }
}

/// A git source resolved to a content directory and a durable pin.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedGitSource {
    /// Absolute path to the extracted content root.
    pub dir: PathBuf,
    /// The resolved commit id — the durable pin.
    pub sha: CommitSha,
    /// The ref that was requested, echoed back for the registry record.
    pub requested_ref: Option<String>,
}

/// Resolves a git source to a directory on disk.
///
/// A trait so install and reconcile can be tested without a network: the tests
/// supply a resolver backed by a local fixture directory, and every path
/// through install — rollback included — runs against the real code.
pub trait GitResolver {
    /// Fetch `url` and extract `subdir` (or the whole tree) at `revision`.
    ///
    /// # Errors
    /// Any [`ModulesError`] in the `Git*` or `ResourceBound*` families.
    fn resolve(
        &self,
        url: &str,
        revision: Revision<'_>,
        subdir: Option<&str>,
    ) -> Result<ResolvedGitSource, ModulesError>;
}

/// What revision to resolve.
///
/// A closed enum rather than an `Option<&str>` pair, so "pinned to a sha" and
/// "follow a ref" cannot both be set and cannot both be absent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Revision<'a> {
    /// An exact commit id. A rebuilt history can orphan one, which is
    /// [`ModulesError::GitRevisionNotFound`], not a silent fallback to HEAD.
    Sha(&'a str),
    /// A tag or branch name.
    Ref(&'a str),
    /// The remote's default branch.
    Head,
}

impl<'a> Revision<'a> {
    /// The revision a source requests: its sha if pinned, else its ref, else
    /// the default branch.
    #[must_use]
    pub fn of(sha: Option<&'a str>, r#ref: Option<&'a str>) -> Self {
        match (sha, r#ref) {
            (Some(sha), _) => Self::Sha(sha),
            (None, Some(r#ref)) => Self::Ref(r#ref),
            (None, None) => Self::Head,
        }
    }

    /// The revision as the string a cache key or an error message names.
    #[must_use]
    pub fn as_str(&self) -> &str {
        match self {
            Self::Sha(s) | Self::Ref(s) => s,
            Self::Head => "HEAD",
        }
    }
}

/// The production resolver: a bare `gix` repository per remote, plus a
/// content directory per `(commit, subdir)`.
#[derive(Debug, Clone)]
pub struct GixResolver {
    cache_root: PathBuf,
    limits: GitLimits,
}

impl GixResolver {
    /// Build a resolver caching under `cache_root`.
    #[must_use]
    pub fn new(cache_root: impl Into<PathBuf>) -> Self {
        Self {
            cache_root: cache_root.into(),
            limits: GitLimits::default(),
        }
    }

    /// Override the resource ceilings.
    #[must_use]
    pub fn with_limits(mut self, limits: GitLimits) -> Self {
        self.limits = limits;
        self
    }

    /// The bare repository directory for `url`.
    fn repo_dir(&self, url: &str) -> PathBuf {
        self.cache_root.join("git").join(sanitize_cache_key(url))
    }

    /// The extracted-content directory for one commit and subdirectory.
    fn content_dir(&self, url: &str, sha: &CommitSha, subdir: Option<&str>) -> PathBuf {
        let leaf = subdir.map_or_else(|| "_root".to_owned(), sanitize_cache_key);
        self.cache_root
            .join("content")
            .join(sanitize_cache_key(url))
            .join(sha.as_str())
            .join(leaf)
    }
}

/// Replace every character outside `[A-Za-z0-9._-]` with `_`.
///
/// The result is a single path segment by construction, which is what makes a
/// remote-supplied url safe to use as a directory name.
#[must_use]
pub fn sanitize_cache_key(raw: &str) -> String {
    let mapped: String = raw
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-') {
                c
            } else {
                '_'
            }
        })
        .collect();
    // `.` is kept because host names need it, which leaves exactly two mapped
    // values that are not inert segment names. Both are renamed rather than
    // stripped, so two distinct inputs still cannot collide.
    match mapped.as_str() {
        "" => "_empty".to_owned(),
        "." => "_dot".to_owned(),
        ".." => "_dotdot".to_owned(),
        _ => mapped,
    }
}

/// A marker written into a content directory once extraction completed.
///
/// Its presence is what makes a cache hit safe: an extraction interrupted
/// half-way leaves no marker, so the next run redoes it rather than serving a
/// partial tree.
const COMPLETE_MARKER: &str = ".quoin-extract-complete";

impl GitResolver for GixResolver {
    fn resolve(
        &self,
        url: &str,
        revision: Revision<'_>,
        subdir: Option<&str>,
    ) -> Result<ResolvedGitSource, ModulesError> {
        let requested_ref = match revision {
            Revision::Ref(r) => Some(r.to_owned()),
            Revision::Sha(_) | Revision::Head => None,
        };

        // An exact pin whose content is already extracted needs no network at
        // all — the hot path for every CLI run once the default set is settled.
        if let Revision::Sha(sha) = revision {
            if let Ok(sha) = CommitSha::new(sha) {
                let dir = self.content_dir(url, &sha, subdir);
                if dir.join(COMPLETE_MARKER).is_file() {
                    return Ok(ResolvedGitSource {
                        dir,
                        sha,
                        requested_ref,
                    });
                }
            }
        }

        let repo_dir = self.repo_dir(url);
        let repo = fetch_into(&repo_dir, url, revision, self.limits)?;
        let sha = resolve_revision(&repo, url, revision)?;

        let dir = self.content_dir(url, &sha, subdir);
        if !dir.join(COMPLETE_MARKER).is_file() {
            extract_tree(&repo, &sha, subdir, &dir, self.limits)?;
        }
        Ok(ResolvedGitSource {
            dir,
            sha,
            requested_ref,
        })
    }
}

/// Open or create the bare repository and fetch from `url`.
fn fetch_into(
    repo_dir: &Path,
    url: &str,
    revision: Revision<'_>,
    limits: GitLimits,
) -> Result<gix::Repository, ModulesError> {
    let transport = |detail: String| ModulesError::GitTransport {
        url: url.to_owned(),
        detail,
    };

    let repo = if repo_dir.join("HEAD").is_file() {
        gix::open(repo_dir).map_err(|e| transport(e.to_string()))?
    } else {
        fs::create_dir_all(repo_dir).map_err(|e| transport(e.to_string()))?;
        gix::init_bare(repo_dir).map_err(|e| transport(e.to_string()))?
    };

    // An exact pin that is already in the object database is settled: no
    // network. This is the equivalent of the oracle's "zero git when settled"
    // reconcile path, moved one layer down so ad-hoc installs get it too.
    if let Revision::Sha(sha) = revision {
        if repo.rev_parse_single(sha).is_ok() {
            return Ok(repo);
        }
    }

    let deadline = Deadline::start(limits.fetch_budget);
    let refspecs: &[&str] = &[
        "+refs/heads/*:refs/remotes/origin/*",
        "+refs/tags/*:refs/tags/*",
    ];
    let remote = repo
        .remote_at(url)
        .map_err(|e| transport(e.to_string()))?
        .with_refspecs(refspecs.iter().copied(), gix::remote::Direction::Fetch)
        .map_err(|e| transport(e.to_string()))?;

    let outcome = remote
        .connect(gix::remote::Direction::Fetch)
        .map_err(|e| transport(e.to_string()))
        .and_then(|connection| {
            connection
                .prepare_fetch(
                    gix::progress::Discard,
                    gix::remote::ref_map::Options::default(),
                )
                .map_err(|e| transport(e.to_string()))
        })
        .and_then(|prepare| {
            prepare
                .receive(gix::progress::Discard, deadline.flag())
                .map_err(|e| transport(e.to_string()))
        });

    match outcome {
        Ok(_) => Ok(repo),
        // A fetch stopped by the watchdog surfaces as a transport error. The
        // timeout is the cause and the transport error the symptom, so the
        // budget is reported and the symptom dropped.
        Err(_) if deadline.expired() => Err(ModulesError::GitTimeout {
            url: url.to_owned(),
            budget_secs: limits.fetch_budget.as_secs(),
        }),
        Err(err) => Err(err),
    }
}

/// Resolve `revision` to a commit id in the fetched repository.
fn resolve_revision(
    repo: &gix::Repository,
    url: &str,
    revision: Revision<'_>,
) -> Result<CommitSha, ModulesError> {
    let not_found = || ModulesError::GitRevisionNotFound {
        url: url.to_owned(),
        revision: revision.as_str().to_owned(),
    };
    let spec = match revision {
        Revision::Sha(sha) => sha.to_owned(),
        Revision::Ref(r) => r.to_owned(),
        Revision::Head => "refs/remotes/origin/HEAD".to_owned(),
    };
    let id = repo
        .rev_parse_single(spec.as_str())
        .or_else(|_| repo.rev_parse_single(format!("refs/remotes/origin/{spec}").as_str()))
        .or_else(|_| repo.rev_parse_single(format!("refs/tags/{spec}").as_str()))
        .map_err(|_| not_found())?;
    let commit = id
        .object()
        .map_err(|_| not_found())?
        .peel_to_kind(gix::object::Kind::Commit)
        .map_err(|_| not_found())?;
    CommitSha::new(commit.id().to_hex().to_string()).map_err(|_| not_found())
}

/// Extract the tree at `sha` (optionally descending into `subdir`) into `dest`.
fn extract_tree(
    repo: &gix::Repository,
    sha: &CommitSha,
    subdir: Option<&str>,
    dest: &Path,
    limits: GitLimits,
) -> Result<(), ModulesError> {
    let store = |detail: String| ModulesError::GitObjectStore {
        path: repo.path().to_path_buf(),
        detail,
    };

    let tree = repo
        .rev_parse_single(sha.as_str())
        .map_err(|e| store(e.to_string()))?
        .object()
        .map_err(|e| store(e.to_string()))?
        .peel_to_tree()
        .map_err(|e| store(e.to_string()))?;

    let root = match subdir {
        Some(path) => descend(repo, &tree, path)?,
        None => tree,
    };

    // Extract into a staging directory and rename, so an interrupted run never
    // leaves a partial tree where a cache hit would find it.
    let staging = dest.with_extension(format!("partial.{}", std::process::id()));
    let _ = fs::remove_dir_all(&staging);
    fs::create_dir_all(&staging).map_err(|source| ModulesError::MaterializeFailed {
        target: staging.clone(),
        source,
    })?;

    let mut budget = ExtractBudget {
        bytes: 0,
        files: 0,
        limits,
    };
    write_tree(repo, &root, &staging, 0, &mut budget)?;
    fs::write(staging.join(COMPLETE_MARKER), sha.as_str()).map_err(|source| {
        ModulesError::MaterializeFailed {
            target: staging.clone(),
            source,
        }
    })?;

    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent).map_err(|source| ModulesError::MaterializeFailed {
            target: dest.to_path_buf(),
            source,
        })?;
    }
    let _ = fs::remove_dir_all(dest);
    fs::rename(&staging, dest).map_err(|source| ModulesError::MaterializeFailed {
        target: dest.to_path_buf(),
        source,
    })
}

/// Running totals for one extraction, checked against the ceilings.
struct ExtractBudget {
    bytes: u64,
    files: u64,
    limits: GitLimits,
}

impl ExtractBudget {
    fn charge(&mut self, bytes: u64) -> Result<(), ModulesError> {
        self.files += 1;
        if self.files > self.limits.max_extracted_files {
            return Err(ModulesError::ResourceBoundExceeded {
                what: "extracted file count",
                observed: self.files,
                limit: self.limits.max_extracted_files,
            });
        }
        self.bytes = self.bytes.saturating_add(bytes);
        if self.bytes > self.limits.max_extracted_bytes {
            return Err(ModulesError::ResourceBoundExceeded {
                what: "extracted byte count",
                observed: self.bytes,
                limit: self.limits.max_extracted_bytes,
            });
        }
        Ok(())
    }
}

/// Descend into a subdirectory of a tree.
fn descend<'repo>(
    repo: &'repo gix::Repository,
    tree: &gix::Tree<'repo>,
    path: &str,
) -> Result<gix::Tree<'repo>, ModulesError> {
    let missing = || ModulesError::GitObjectStore {
        path: repo.path().to_path_buf(),
        detail: format!("subdirectory {path:?} is absent from the resolved tree"),
    };
    let entry = tree
        .lookup_entry_by_path(path)
        .map_err(|e| ModulesError::GitObjectStore {
            path: repo.path().to_path_buf(),
            detail: e.to_string(),
        })?
        .ok_or_else(missing)?;
    entry
        .object()
        .map_err(|e| ModulesError::GitObjectStore {
            path: repo.path().to_path_buf(),
            detail: e.to_string(),
        })?
        .try_into_tree()
        .map_err(|_| missing())
}

/// Write one tree level to disk, recursing into subtrees.
fn write_tree(
    repo: &gix::Repository,
    tree: &gix::Tree<'_>,
    dest: &Path,
    depth: usize,
    budget: &mut ExtractBudget,
) -> Result<(), ModulesError> {
    if depth > budget.limits.max_tree_depth {
        return Err(ModulesError::ResourceBoundExceeded {
            what: "tree depth",
            observed: depth as u64,
            limit: budget.limits.max_tree_depth as u64,
        });
    }
    let store = |detail: String| ModulesError::GitObjectStore {
        path: repo.path().to_path_buf(),
        detail,
    };

    for entry in tree.iter() {
        let entry = entry.map_err(|e| store(e.to_string()))?;
        let name = entry.filename().to_string();
        // Tree entry names are remote-supplied. Anything that is not a plain
        // path segment is refused rather than sanitized, because a sanitized
        // name silently installs different content than the repository names.
        if name.is_empty()
            || name == "."
            || name == ".."
            || name.contains('/')
            || name.contains('\\')
            || name.contains('\0')
        {
            return Err(ModulesError::UnsafeTreePath { entry: name });
        }
        let target = dest.join(&name);
        let mode = entry.mode();

        if mode.is_tree() {
            let subtree = entry
                .object()
                .map_err(|e| store(e.to_string()))?
                .try_into_tree()
                .map_err(|e| store(e.to_string()))?;
            fs::create_dir_all(&target).map_err(|source| ModulesError::MaterializeFailed {
                target: target.clone(),
                source,
            })?;
            write_tree(repo, &subtree, &target, depth + 1, budget)?;
            continue;
        }
        if mode.is_link() || mode.is_commit() {
            // A symlink can point anywhere, and a gitlink names a submodule
            // this crate does not fetch. Both are skipped rather than
            // materialized: a spec module is data files.
            continue;
        }

        let blob = entry.object().map_err(|e| store(e.to_string()))?;
        let data = blob.data.as_slice();
        budget.charge(data.len() as u64)?;
        fs::write(&target, data).map_err(|source| ModulesError::MaterializeFailed {
            target: target.clone(),
            source,
        })?;
        #[cfg(unix)]
        if mode.is_executable() {
            use std::os::unix::fs::PermissionsExt as _;
            let _ = fs::set_permissions(&target, fs::Permissions::from_mode(0o755));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Trace: FR-019-AC-2
    #[test]
    fn tc_381_250_cache_keys_are_single_path_segments() {
        let key = sanitize_cache_key("https://github.com/agent-ix/quoin.git");
        assert!(!key.contains('/'), "{key}");
        assert!(!key.contains(".."), "{key}");
        assert_eq!(sanitize_cache_key("../../etc/passwd"), ".._.._etc_passwd");
        // The only two mapped values that would name a parent or self
        // directory are renamed, so a key is always an inert segment.
        assert_eq!(sanitize_cache_key(".."), "_dotdot");
        assert_eq!(sanitize_cache_key("."), "_dot");
        assert_eq!(sanitize_cache_key(""), "_empty");
    }

    /// Trace: FR-019-AC-2
    #[test]
    fn tc_381_251_revision_of_prefers_the_pinned_sha() {
        assert_eq!(Revision::of(Some("abc"), Some("v1")), Revision::Sha("abc"));
        assert_eq!(Revision::of(None, Some("v1")), Revision::Ref("v1"));
        assert_eq!(Revision::of(None, None), Revision::Head);
    }

    /// Trace: NFR-004
    #[test]
    fn tc_381_252_deadline_raises_the_interrupt_flag_and_joins() {
        let deadline = Deadline::start(Duration::from_millis(50));
        // A positive, observable signal rather than a sleep-then-assert: poll
        // the flag the fetch itself polls.
        let deadline = std::thread::scope(|_| deadline);
        let mut raised = false;
        for _ in 0..200 {
            if deadline.flag().load(Ordering::Acquire) {
                raised = true;
                break;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        assert!(raised, "the watchdog must raise the interrupt flag");
        assert!(deadline.expired());
        drop(deadline);
    }
}
