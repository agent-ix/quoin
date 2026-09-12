// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Typed module/marketplace source descriptors (quoin#381, FR-019).
//!
//! Wire-compatible with `@agent-ix/ts-plugin-kit`'s `Source` union: the same
//! `type` discriminant and the same field names, because these values are
//! persisted verbatim in `~/.ix/filament/registry.json` and in
//! `default-modules.yaml`, both of which the retained TypeScript still reads
//! during staged coexistence.

use serde::{Deserialize, Serialize};

use crate::error::{ModulesError, SourceFieldRule};

/// Where a module's content comes from.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "kebab-case", deny_unknown_fields)]
pub enum Source {
    /// A GitHub repository whose module root is the repository root.
    Github {
        /// `owner/repo`, or a full clonable url.
        repo: String,
        /// A tag or branch to pin to.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        r#ref: Option<String>,
        /// A commit id to pin to; outranks `ref`.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        sha: Option<String>,
    },
    /// A module living in a subdirectory of a git repository.
    GitSubdir {
        /// `owner/repo`, or a full clonable url.
        url: String,
        /// The subdirectory holding the module root.
        path: String,
        /// A tag or branch to pin to.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        r#ref: Option<String>,
        /// A commit id to pin to; outranks `ref`.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        sha: Option<String>,
    },
    /// A git repository by url.
    Git {
        /// A clonable url.
        url: String,
        /// A tag or branch to pin to.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        r#ref: Option<String>,
        /// A commit id to pin to; outranks `ref`.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        sha: Option<String>,
    },
    /// A plain url. Accepted structurally, not resolvable.
    Url {
        /// The url.
        url: String,
        /// A tag or branch, carried for round-tripping.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        r#ref: Option<String>,
        /// A commit id, carried for round-tripping.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        sha: Option<String>,
    },
    /// A local directory.
    Path {
        /// The directory.
        path: String,
    },
    /// An npm package. Accepted structurally, not resolvable here.
    Npm {
        /// The package name.
        package: String,
        /// An exact version.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        version: Option<String>,
        /// A registry override.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        registry: Option<String>,
    },
}

impl Source {
    /// The `type` discriminant as it appears on the wire.
    #[must_use]
    pub const fn type_name(&self) -> &'static str {
        match self {
            Self::Github { .. } => "github",
            Self::GitSubdir { .. } => "git-subdir",
            Self::Git { .. } => "git",
            Self::Url { .. } => "url",
            Self::Path { .. } => "path",
            Self::Npm { .. } => "npm",
        }
    }

    /// Validate the descriptor's fields.
    ///
    /// Every non-`path` string field must be non-empty and must not begin with
    /// `-`: a value that looks like an option is how a repo name becomes a git
    /// flag. `path` is exempt from the option rule, matching the oracle — a
    /// local directory is never handed to git as a bare argument.
    ///
    /// # Errors
    /// [`ModulesError::InvalidSourceField`] naming the field and the rule.
    pub fn validate(&self) -> Result<(), ModulesError> {
        fn non_empty(value: &str, field: &'static str) -> Result<(), ModulesError> {
            if value.is_empty() {
                return Err(ModulesError::InvalidSourceField {
                    field,
                    rule: SourceFieldRule::NonEmpty,
                });
            }
            Ok(())
        }
        fn safe(value: &str, field: &'static str) -> Result<(), ModulesError> {
            non_empty(value, field)?;
            if value.trim_start().starts_with('-') {
                return Err(ModulesError::InvalidSourceField {
                    field,
                    rule: SourceFieldRule::NotOptionLike,
                });
            }
            Ok(())
        }
        fn safe_opt(value: Option<&String>, field: &'static str) -> Result<(), ModulesError> {
            value.map_or(Ok(()), |v| safe(v, field))
        }

        match self {
            Self::Github { repo, r#ref, sha } => {
                safe(repo, "repo")?;
                safe_opt(r#ref.as_ref(), "ref")?;
                safe_opt(sha.as_ref(), "sha")
            }
            Self::Git { url, r#ref, sha } | Self::Url { url, r#ref, sha } => {
                safe(url, "url")?;
                safe_opt(r#ref.as_ref(), "ref")?;
                safe_opt(sha.as_ref(), "sha")
            }
            Self::GitSubdir {
                url,
                path,
                r#ref,
                sha,
            } => {
                safe(url, "url")?;
                safe(path, "path")?;
                safe_opt(r#ref.as_ref(), "ref")?;
                safe_opt(sha.as_ref(), "sha")
            }
            Self::Path { path } => non_empty(path, "path"),
            Self::Npm { package, .. } => safe(package, "package"),
        }
    }

    /// The git subdirectory this source selects, if any.
    #[must_use]
    pub fn subdir(&self) -> Option<&str> {
        match self {
            Self::GitSubdir { path, .. } => Some(path.as_str()),
            _ => None,
        }
    }

    /// The commit id this source pins to, if any.
    #[must_use]
    pub fn pinned_sha(&self) -> Option<&str> {
        match self {
            Self::Github { sha, .. }
            | Self::GitSubdir { sha, .. }
            | Self::Git { sha, .. }
            | Self::Url { sha, .. } => sha.as_deref(),
            Self::Path { .. } | Self::Npm { .. } => None,
        }
    }

    /// The tag or branch this source requests, if any.
    #[must_use]
    pub fn requested_ref(&self) -> Option<&str> {
        match self {
            Self::Github { r#ref, .. }
            | Self::GitSubdir { r#ref, .. }
            | Self::Git { r#ref, .. }
            | Self::Url { r#ref, .. } => r#ref.as_deref(),
            Self::Path { .. } | Self::Npm { .. } => None,
        }
    }

    /// The clonable git url for this source, when it is a git source.
    #[must_use]
    pub fn git_url(&self) -> Option<String> {
        match self {
            Self::Github { repo, .. } => Some(to_git_url(repo)),
            Self::GitSubdir { url, .. } | Self::Git { url, .. } => Some(to_git_url(url)),
            Self::Url { .. } | Self::Path { .. } | Self::Npm { .. } => None,
        }
    }
}

/// Expand a git reference to a clonable url.
///
/// Full urls (`scheme://…`, `git@…`) pass through; an `owner/repo` shorthand
/// expands to GitHub.
#[must_use]
pub fn to_git_url(raw: &str) -> String {
    let trimmed = raw.trim();
    if trimmed.contains("://") || trimmed.starts_with("git@") {
        return trimmed.to_owned();
    }
    let repo = trimmed.strip_suffix(".git").unwrap_or(trimmed);
    format!("https://github.com/{repo}.git")
}

/// Parse a quoin CLI install argument into a typed [`Source`].
///
/// Supported forms, matching `quoin module install --help`:
///
/// | Argument | Source |
/// |---|---|
/// | `path:<dir>` | [`Source::Path`] |
/// | `github:<owner>/<repo>[@<ref>]` | [`Source::Github`] |
/// | `github:<owner>/<repo>//<subdir>[@<ref>]` | [`Source::GitSubdir`] |
/// | `package:<name>[@<version>]` | [`Source::Npm`] |
/// | anything else | [`Source::Path`] |
///
/// # Errors
/// [`ModulesError::UnparsableSourceArg`] when a prefix is present but its
/// remainder is empty, and whatever [`Source::validate`] refuses.
pub fn parse_source_arg(arg: &str) -> Result<Source, ModulesError> {
    let unparsable = |detail: &'static str| ModulesError::UnparsableSourceArg {
        arg: arg.to_owned(),
        detail,
    };

    let source = if let Some(path) = arg.strip_prefix("path:") {
        Source::Path {
            path: path.to_owned(),
        }
    } else if let Some(spec) = arg.strip_prefix("github:") {
        let (spec, r#ref) = match spec.split_once('@') {
            Some((spec, r#ref)) => (spec, Some(r#ref.to_owned())),
            None => (spec, None),
        };
        // `owner/repo//subdir` installs a module that lives in a monorepo
        // subdirectory — the same git-subdir source the default module set uses.
        if let Some((url, path)) = spec.split_once("//") {
            Source::GitSubdir {
                url: url.to_owned(),
                path: path.to_owned(),
                r#ref,
                sha: None,
            }
        } else {
            Source::Github {
                repo: spec.to_owned(),
                r#ref,
                sha: None,
            }
        }
    } else if let Some(spec) = arg.strip_prefix("package:") {
        if spec.is_empty() {
            return Err(unparsable("`package:` must name a package"));
        }
        // Split on the *last* `@` so a scope (`@scope/foo`) survives.
        match spec.rfind('@').filter(|idx| *idx > 0) {
            Some(idx) => Source::Npm {
                package: spec[..idx].to_owned(),
                version: Some(spec[idx + 1..].to_owned()),
                registry: None,
            },
            None => Source::Npm {
                package: spec.to_owned(),
                version: None,
                registry: None,
            },
        }
    } else {
        Source::Path {
            path: arg.to_owned(),
        }
    };

    source.validate()?;
    Ok(source)
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::panic,
        clippy::indexing_slicing,
        reason = "in a test, a panic IS the failure report; the production lints stand"
    )]

    use super::*;

    /// Trace: FR-019-AC-2
    #[test]
    fn tc_381_210_option_like_fields_are_refused() {
        let evil = Source::Github {
            repo: "--upload-pack=evil".to_owned(),
            r#ref: None,
            sha: None,
        };
        let err = evil.validate().expect_err("an option-like repo is refused");
        assert_eq!(err.code(), crate::error::ModulesErrorCode::InvalidSource);
    }

    /// Trace: FR-019-AC-2
    #[test]
    fn tc_381_211_package_prefix_with_no_package_is_refused() {
        let err = parse_source_arg("package:").expect_err("no package named");
        assert_eq!(
            err.code(),
            crate::error::ModulesErrorCode::UnparsableSourceArg
        );
    }
}
