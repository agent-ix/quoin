// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Authoring-organization resolution (quoin#381, FR-025 / FR-027).
//!
//! Precedence, first non-empty wins: an explicit `--org`, `QUOIN_ORG`, the
//! stored config, then the `origin` remote in the repository's git config.
//!
//! A stored value outranks the remote because it is something a person said,
//! where the remote is only what quoin could infer. Neither weakens the
//! no-substitution rule: when nothing yields an organization the answer is
//! [`OrgSource::None`], never a default. An org qualifies a repository so that
//! same-named repos in different organizations stay distinguishable, so
//! inventing one defeats the point of carrying it — and a declared org is
//! human-facing identity in a published artifact, where a plausible-but-wrong
//! value is harder to notice than an absent one that stops the author and asks.
//!
//! The git config is **read as a file**, never by invoking `git`, so resolution
//! holds where no git executable is present (NFR-004). This is deliberate and
//! is the reason this module does not use `gix` even though `quoin-modules`
//! does: `gix` would open a repository, and an unreadable or half-initialised
//! `.git` must be "no org here", not an error.

use std::fs;
use std::path::{Path, PathBuf};

use crate::error::ConfigError;
use crate::ids::OrgName;
use crate::paths::{Environment, RuntimeContext};
use crate::schema::QuoinConfig;
use crate::service::{ConfigService, IncidentLog};

/// Message shown when no source yielded an organization (FR-025, NFR-003).
pub const UNRESOLVED_ORG_MESSAGE: &str = concat!(
    "could not determine the authoring organization: no --org flag, no QUOIN_ORG, ",
    "no stored config, and no [remote \"origin\"] in .git/config. ",
    "Pass --org <name>, or store one with `quoin config set org <name>`."
);

/// Upper bound on a git config file read, in bytes.
///
/// `.git/config` is a small ini file. The bound stops a symlinked or corrupt
/// path being read whole; over the limit resolves to "no org here", which is
/// the same answer every other unreadable-metadata path gives.
pub const MAX_GIT_CONFIG_BYTES: u64 = 4 << 20;

/// How deep a chain of `.git` pointer files may be followed.
///
/// One hop is the real layout (worktree `.git` file → gitdir → `commondir`).
/// The ceiling exists because the pointer target is attacker-influencable
/// content on disk and a cycle would otherwise not terminate.
const MAX_GITDIR_HOPS: usize = 8;

/// Where a resolved organization came from, or `None` when nothing yielded one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrgSource {
    /// An explicit `--org` value.
    Flag,
    /// The `QUOIN_ORG` environment variable.
    Env,
    /// The stored configuration file.
    Config,
    /// The `origin` remote in the repository's git config.
    Git,
    /// Nothing yielded an organization.
    None,
}

impl OrgSource {
    /// The stable wire string, matching the TypeScript `OrgSource` union.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Flag => "flag",
            Self::Env => "env",
            Self::Config => "config",
            Self::Git => "git",
            Self::None => "none",
        }
    }
}

impl std::fmt::Display for OrgSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// A resolved organization and its provenance.
///
/// The two fields are not independent: `org` is `Some` exactly when `source` is
/// not [`OrgSource::None`]. [`ResolvedOrg::org`] and [`ResolvedOrg::source`] are
/// produced together by [`resolve_org`] so the pairing cannot drift.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedOrg {
    /// The organization, absent when unresolved.
    pub org: Option<OrgName>,
    /// Where it came from.
    pub source: OrgSource,
}

impl ResolvedOrg {
    /// The unresolved answer.
    #[must_use]
    pub fn unresolved() -> Self {
        Self {
            org: None,
            source: OrgSource::None,
        }
    }

    fn found(org: OrgName, source: OrgSource) -> Self {
        Self {
            org: Some(org),
            source,
        }
    }

    /// The organization, or [`UNRESOLVED_ORG_MESSAGE`] as an error.
    ///
    /// # Errors
    /// [`ConfigError::InvalidOrgName`] when nothing resolved. Callers that must
    /// have an org use this; callers reporting provenance read the fields.
    pub fn require(&self) -> Result<&OrgName, ConfigError> {
        self.org.as_ref().ok_or(ConfigError::InvalidOrgName)
    }
}

/// Options for [`resolve_org`].
#[derive(Debug, Clone, Default)]
pub struct OrgOptions<'a> {
    /// The explicit `--org` value.
    pub flag: Option<&'a str>,
}

/// Resolve the authoring organization for the repository at `repo_root`.
///
/// `env` supplies `QUOIN_ORG` and the XDG roots; `ctx` supplies the
/// `--config-root` override and the project-local `.ix` layer. Incidents from a
/// broken config file are appended to `log` — the read still succeeds, with no
/// stored org, and resolution carries on to the remote (FR-027-AC-5).
pub fn resolve_org(
    repo_root: &Path,
    options: &OrgOptions<'_>,
    env: &dyn Environment,
    ctx: &RuntimeContext,
    log: &mut IncidentLog,
) -> ResolvedOrg {
    if let Some(flag) = options.flag.and_then(OrgName::parse_opt) {
        return ResolvedOrg::found(flag, OrgSource::Flag);
    }

    let service = ConfigService::<QuoinConfig>::for_plugin(env, ctx);
    let stored = service
        .get(log)
        .value
        .org
        .as_ref()
        .and_then(|org| OrgName::parse_opt(org.as_str()));

    if let Some(stored) = stored {
        // `QUOIN_ORG` is layered over the file by the env binding, so a value
        // here came from whichever of the two won. Report which.
        let from_env = env
            .var("QUOIN_ORG")
            .and_then(|raw| OrgName::parse_opt(&raw))
            .is_some_and(|e| e == stored);
        return ResolvedOrg::found(
            stored,
            if from_env {
                OrgSource::Env
            } else {
                OrgSource::Config
            },
        );
    }

    if let Some(from_git) = org_from_git_config(repo_root) {
        return ResolvedOrg::found(from_git, OrgSource::Git);
    }

    ResolvedOrg::unresolved()
}

/// Read the organization from the `origin` remote in `<repo_root>`'s git config.
///
/// An unreadable config, an absent `origin`, or an unparsable url yields `None`
/// — never an error, since "no org here" is a resolution outcome, not a failure.
#[must_use]
pub fn org_from_git_config(repo_root: &Path) -> Option<OrgName> {
    let git_dir = resolve_git_dir(repo_root)?;
    let path = git_dir.join("config");
    let meta = fs::symlink_metadata(&path).ok()?;
    if meta.is_file() && meta.len() > MAX_GIT_CONFIG_BYTES {
        return None;
    }
    let text = fs::read_to_string(&path).ok()?;
    origin_org(&text)
}

/// Locate the git directory holding the config for `repo_root`.
///
/// In an ordinary checkout that is `<repo_root>/.git`. In a worktree (and in a
/// submodule) `.git` is a *file* holding `gitdir: <path>`, and the config lives
/// in the shared common directory that `<gitdir>/commondir` points at — so a
/// worktree resolves the org its main checkout would.
#[must_use]
pub fn resolve_git_dir(repo_root: &Path) -> Option<PathBuf> {
    let dot_git = repo_root.join(".git");
    let meta = fs::metadata(&dot_git).ok()?;
    if meta.is_dir() {
        return Some(dot_git);
    }

    let mut base = repo_root.to_path_buf();
    let mut pointer_file = dot_git;
    for _ in 0..MAX_GITDIR_HOPS {
        let text = fs::read_to_string(&pointer_file).ok()?;
        let target = gitdir_pointer(&text)?;
        let git_dir = resolve_against(&base, Path::new(target));
        // `commondir` is how a worktree names the checkout that owns the
        // config; its absence means this gitdir holds the config itself.
        let Ok(common) = fs::read_to_string(git_dir.join("commondir")) else {
            return Some(git_dir);
        };
        let common = common.trim();
        if common.is_empty() {
            return Some(git_dir);
        }
        let resolved = resolve_against(&git_dir, Path::new(common));
        if resolved.join("config").exists() || !resolved.join(".git").is_file() {
            return Some(resolved);
        }
        base.clone_from(&resolved);
        pointer_file = resolved.join(".git");
    }
    None
}

/// Join `path` onto `base` unless it is already absolute.
fn resolve_against(base: &Path, path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        base.join(path)
    }
}

/// Extract the target of a `gitdir: <path>` pointer line.
fn gitdir_pointer(text: &str) -> Option<&str> {
    text.lines().find_map(|line| {
        let rest = line.strip_prefix("gitdir:")?;
        let rest = rest.trim();
        (!rest.is_empty()).then_some(rest)
    })
}

/// Parse the org out of the `[remote "origin"]` url in a git config.
///
/// Handles both `git@host:org/repo.git` and `https://host/org/repo.git`. Git
/// treats section names case-insensitively but subsection names (the quoted
/// remote name) case-sensitively, and so does this.
#[must_use]
pub fn origin_org(config: &str) -> Option<OrgName> {
    let mut in_origin = false;
    for line in config.split('\n') {
        let trimmed = line.trim();
        if trimmed.starts_with('[') {
            let squashed: String = trimmed.chars().filter(|c| !c.is_whitespace()).collect();
            in_origin = squashed.to_ascii_lowercase().starts_with("[remote")
                && squashed.get("[remote".len()..) == Some("\"origin\"]");
            continue;
        }
        if !in_origin || !trimmed.starts_with("url") {
            continue;
        }
        let after_key = trimmed.get("url".len()..)?.trim_start();
        let Some(value) = after_key.strip_prefix('=') else {
            continue;
        };
        return org_from_remote_url(value.trim());
    }
    None
}

/// Extract the owning org from a remote url, or `None` when it names none.
///
/// Only *host-based* remotes carry an org: `scheme://host/org/repo` and the
/// scp-style `[user@]host:org/repo`. A local-path remote (`/srv/git/repo.git`,
/// `../sibling`, `file:///…`) has no org at all, and a host-based url with a
/// single path segment (`https://host/repo.git`) names a repo but no owner.
///
/// Both cases yield `None` rather than a best guess. Taking the second-to-last
/// path segment regardless would answer `git` for `/srv/git/repo.git`, `..` for
/// `../sibling`, and the *hostname* for `https://host/repo.git` — a
/// confidently-reported wrong org, the exact failure FR-025 exists to prevent.
#[must_use]
pub fn org_from_remote_url(url: &str) -> Option<OrgName> {
    let path = if let Some((scheme, rest)) = split_scheme(url) {
        // `file://` is a local path dressed as a url — it names no org.
        if scheme.eq_ignore_ascii_case("file") {
            return None;
        }
        let after_host = rest.find('/')?;
        &rest[after_host + 1..]
    } else if url.starts_with('/') || url.starts_with('.') || url.starts_with('~') {
        return None;
    } else {
        // scp-style `[user@]host:org/repo`. No colon at all means a relative
        // local path, which names no org.
        let colon = url.find(':')?;
        &url[colon + 1..]
    };

    let segments: Vec<&str> = path
        .trim_end_matches('/')
        .split('/')
        .filter(|s| !s.is_empty())
        .collect();
    if segments.len() < 2 {
        return None;
    }
    // The repo is the last segment; its owner is the one before it. Nested
    // namespaces (`org/subgroup/repo`) therefore qualify by the innermost
    // group, matching filament-ide-rs's repo_identity so both layers name a
    // repo alike.
    segments
        .get(segments.len() - 2)
        .and_then(|s| OrgName::parse_opt(s))
}

/// Split `scheme://rest`, if `url` has that shape.
fn split_scheme(url: &str) -> Option<(&str, &str)> {
    let idx = url.find("://")?;
    let scheme = &url[..idx];
    let mut chars = scheme.chars();
    let first = chars.next()?;
    if !first.is_ascii_alphabetic()
        || !chars.all(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '.' | '-'))
    {
        return None;
    }
    Some((scheme, &url[idx + 3..]))
}
