// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Validated identity newtypes (quoin#381).
//!
//! Plugin ids, package names and organization names are all `String`-shaped and
//! are routinely passed adjacent to one another. Newtypes make the call sites
//! un-swappable and put each identifier's validation rule in exactly one place.

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

use crate::error::ConfigError;

/// Maximum byte length of a plugin id, matching ix-cli-core.
const MAX_PLUGIN_ID_LEN: usize = 64;

/// A config/secrets namespace id: `^[a-z][a-z0-9-]*$`, at most 64 bytes.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(transparent)]
pub struct PluginId(String);

impl PluginId {
    /// Validate and wrap a plugin id.
    ///
    /// # Errors
    /// [`ConfigError::InvalidPluginId`] when the id is empty, too long, or
    /// contains anything but lowercase ASCII letters, digits and `-`, or does
    /// not start with a lowercase ASCII letter.
    pub fn new(id: impl Into<String>) -> Result<Self, ConfigError> {
        let id = id.into();
        if Self::is_legal(&id) {
            Ok(Self(id))
        } else {
            Err(ConfigError::InvalidPluginId { id })
        }
    }

    /// Whether `id` satisfies the id grammar. Pure, so the rule has one home.
    #[must_use]
    pub fn is_legal(id: &str) -> bool {
        if id.is_empty() || id.len() > MAX_PLUGIN_ID_LEN {
            return false;
        }
        let mut chars = id.chars();
        let Some(first) = chars.next() else {
            return false;
        };
        first.is_ascii_lowercase()
            && chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
    }

    /// The id as a string slice.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for PluginId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl FromStr for PluginId {
    type Err = ConfigError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::new(s)
    }
}

impl<'de> Deserialize<'de> for PluginId {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let raw = String::deserialize(d)?;
        Self::new(raw).map_err(serde::de::Error::custom)
    }
}

/// An npm package name, e.g. `@agent-ix/quoin`.
///
/// The key a plugin schema is registered under. Distinct from [`PluginId`],
/// which is the *config namespace* derived from it.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(transparent)]
pub struct PackageName(String);

impl PackageName {
    /// Wrap a package name.
    ///
    /// # Errors
    /// [`ConfigError::InvalidPackageName`] when the name is empty.
    pub fn new(name: impl Into<String>) -> Result<Self, ConfigError> {
        let name = name.into();
        if name.is_empty() {
            return Err(ConfigError::InvalidPackageName);
        }
        Ok(Self(name))
    }

    /// The package name as a string slice.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Derive the default config namespace id from this package name.
    ///
    /// Mirrors ix-cli-core's derivation exactly: drop an npm scope, drop a
    /// leading `ix-cli-`, map the historical `workflow-cli-plugin` alias, then
    /// replace every character outside `[A-Za-z0-9-]` with `-` and lowercase.
    ///
    /// # Errors
    /// [`ConfigError::UnderivablePluginId`] when the result is not a legal
    /// [`PluginId`].
    pub fn derive_plugin_id(&self) -> Result<PluginId, ConfigError> {
        let mut base = self.0.as_str();
        if let Some(rest) = base.strip_prefix('@') {
            // `@scope/name` -> `name`; a scope with no `/` keeps the remainder.
            base = rest.split_once('/').map_or(rest, |(_, name)| name);
        }
        base = base.strip_prefix("ix-cli-").unwrap_or(base);
        if base == "workflow-cli-plugin" {
            base = "workflow";
        }
        let derived: String = base
            .chars()
            .map(|c| {
                if c.is_ascii_alphanumeric() || c == '-' {
                    c.to_ascii_lowercase()
                } else {
                    '-'
                }
            })
            .collect();
        PluginId::new(derived.clone()).map_err(|_| ConfigError::UnderivablePluginId {
            package: self.0.clone(),
            derived,
        })
    }
}

impl fmt::Display for PackageName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// A non-empty authoring organization name.
///
/// Constructed only through [`OrgName::parse`], which trims and refuses an
/// empty result — the no-substitution rule of FR-025 lives here so no caller
/// can construct a blank org.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(transparent)]
pub struct OrgName(String);

impl OrgName {
    /// Trim `raw` and wrap it.
    ///
    /// # Errors
    /// [`ConfigError::InvalidOrgName`] when the trimmed value is empty.
    pub fn parse(raw: &str) -> Result<Self, ConfigError> {
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            return Err(ConfigError::InvalidOrgName);
        }
        Ok(Self(trimmed.to_owned()))
    }

    /// Trim `raw`, yielding `None` rather than an error when it is blank.
    ///
    /// This is the shape every resolution source wants: "nobody said" is an
    /// outcome, not a failure (FR-025).
    #[must_use]
    pub fn parse_opt(raw: &str) -> Option<Self> {
        Self::parse(raw).ok()
    }

    /// The organization as a string slice.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for OrgName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for OrgName {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        // Deliberately does NOT trim: the zod oracle is `z.string().min(1)`, so
        // `org: "   "` is a *valid* stored value that later trims to nothing at
        // the resolution step. Trimming here would silently change which
        // configs validate.
        let raw = String::deserialize(d)?;
        if raw.is_empty() {
            return Err(serde::de::Error::custom(
                "expected a string with at least 1 character",
            ));
        }
        Ok(Self(raw))
    }
}

impl schemars::JsonSchema for OrgName {
    fn schema_name() -> std::borrow::Cow<'static, str> {
        "OrgName".into()
    }

    fn json_schema(_g: &mut schemars::SchemaGenerator) -> schemars::Schema {
        schemars::json_schema!({ "type": "string", "minLength": 1 })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Trace: FR-027-AC-7
    #[test]
    fn tc_381_001_plugin_id_grammar() {
        assert!(PluginId::new("quoin").is_ok());
        assert!(PluginId::new("a-1").is_ok());
        assert!(PluginId::new("").is_err());
        assert!(PluginId::new("Quoin").is_err());
        assert!(PluginId::new("1quoin").is_err());
        assert!(PluginId::new("-quoin").is_err());
        assert!(PluginId::new("quoin_x").is_err());
        assert!(PluginId::new("a".repeat(64)).is_ok());
        assert!(PluginId::new("a".repeat(65)).is_err());
    }

    /// Trace: FR-027-AC-7
    #[test]
    fn tc_381_002_package_name_derives_plugin_id() {
        let cases = [
            ("@agent-ix/quoin", "quoin"),
            ("quoin", "quoin"),
            ("ix-cli-widgets", "widgets"),
            ("workflow-cli-plugin", "workflow"),
            ("@scope/Some.Pkg", "some-pkg"),
        ];
        for (package, expected) in cases {
            let id = PackageName::new(package)
                .expect("package name is non-empty")
                .derive_plugin_id()
                .expect("derivation yields a legal id");
            assert_eq!(id.as_str(), expected, "package {package}");
        }
        assert!(PackageName::new("@scope/")
            .expect("non-empty")
            .derive_plugin_id()
            .is_err());
    }

    /// Trace: FR-025-AC-5
    #[test]
    fn tc_381_003_org_name_refuses_blank() {
        assert_eq!(OrgName::parse("  acme ").expect("trims").as_str(), "acme");
        assert!(OrgName::parse("   ").is_err());
        assert!(OrgName::parse("").is_err());
        assert_eq!(OrgName::parse_opt("  "), None);
    }
}
