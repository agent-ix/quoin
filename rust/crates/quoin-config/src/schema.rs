// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Quoin's persistent configuration schema (quoin#381, FR-027).
//!
//! The TypeScript oracle is `src/config-schema.ts`: a *strict* zod object
//! `{ org?: string (min 1) }`. Strictness is not decoration — it is what makes
//! `config set` reject an unknown key instead of silently writing it, and what
//! `registerPluginSchema` requires at registration.
//!
//! Every leaf is optional rather than defaulted: an absent `org` must stay
//! absent, because "nobody said" is a distinct outcome from any value quoin
//! could pick (FR-025).

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_yaml_ng::Value as Yaml;

use crate::error::ConfigIssue;
use crate::ids::{OrgName, PluginId};

/// Config/secrets namespace quoin registers under.
pub const QUOIN_PLUGIN_ID: &str = "quoin";

/// The npm package quoin's schema is registered for.
pub const QUOIN_PACKAGE_NAME: &str = "@agent-ix/quoin";

/// Map of config key → environment variable, applied as a layer over the file.
///
/// Keeping `QUOIN_ORG` here rather than reading it directly means one
/// precedence rule lives in one place.
pub const QUOIN_ENV_BINDINGS: &[EnvBinding] = &[EnvBinding {
    key_path: "org",
    var_name: "QUOIN_ORG",
}];

/// One `config key → environment variable` binding.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct EnvBinding {
    /// Dotted path into the config object.
    pub key_path: &'static str,
    /// The environment variable whose raw string layers over that path.
    pub var_name: &'static str,
}

/// How `config set` must interpret a raw command-line value for a key.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyKind {
    /// A string/number/boolean/enum leaf: the raw argument is the value.
    Scalar,
    /// An array/object/record/tuple: the raw argument is parsed as JSON.
    Complex,
    /// The schema declares no such key.
    Unknown,
}

/// A plugin's configuration contract.
///
/// Implemented once per plugin namespace. `quoin-config` ships quoin's own
/// implementation; the trait exists so the `ConfigService` machinery — layering,
/// incident recording, atomic writes, doctor — is written once rather than per
/// plugin, which is exactly what ix-cli-core's generic `ConfigService` bought.
pub trait PluginConfigSchema: Sized {
    /// The namespace this schema owns.
    fn plugin_id() -> PluginId;

    /// The `key → env var` bindings applied as the highest layer.
    fn env_bindings() -> &'static [EnvBinding];

    /// Validate a merged YAML mapping into the typed config.
    ///
    /// # Errors
    /// The issue list the merged document violated. Callers treat a failure as
    /// "schema defaults plus a recorded incident", never as a hard stop.
    fn validate(value: &Yaml) -> Result<Self, Vec<ConfigIssue>>;

    /// How `config set` should interpret a raw value for `key_path`.
    fn key_kind(key_path: &str) -> KeyKind;

    /// The JSON Schema for this config, for the generated TypeScript surface.
    fn json_schema() -> schemars::Schema;
}

/// Quoin's persistent configuration.
///
/// `deny_unknown_fields` is the `.strict()` of the zod oracle. `Option<OrgName>`
/// is its `.optional()`, and [`OrgName`]'s deserializer is its `.min(1)`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct QuoinConfig {
    /// The authoring organization, absent when nobody has stated one.
    #[serde(
        default,
        deserialize_with = "deserialize_optional_org",
        skip_serializing_if = "Option::is_none"
    )]
    pub org: Option<OrgName>,
}

/// Deserialize a *present* `org` key.
///
/// zod's `.optional()` accepts `undefined` (an absent key) but refuses an
/// explicit `null`, where serde's `Option<T>` accepts both. The oracle rejects
/// `{ org: null }`, and so must this — `org:` with no value in YAML is a
/// malformed config, not an absent org. Absence is handled by `#[serde(default)]`,
/// so this function only ever sees a key that was written.
fn deserialize_optional_org<'de, D>(d: D) -> Result<Option<OrgName>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    OrgName::deserialize(d).map(Some)
}

impl PluginConfigSchema for QuoinConfig {
    #[allow(
        clippy::expect_used,
        reason = "QUOIN_PLUGIN_ID is a compile-time literal this crate owns, \
                  and `tc_381_020` fails if it ever stops being a legal id; the \
                  alternative is a fallible accessor at every call site for a \
                  condition no caller can cause or repair"
    )]
    fn plugin_id() -> PluginId {
        // The literal is checked by `tc_381_020`; constructing it here keeps
        // the id's grammar enforced rather than assumed.
        PluginId::new(QUOIN_PLUGIN_ID).expect("QUOIN_PLUGIN_ID is a legal plugin id")
    }

    fn env_bindings() -> &'static [EnvBinding] {
        QUOIN_ENV_BINDINGS
    }

    fn validate(value: &Yaml) -> Result<Self, Vec<ConfigIssue>> {
        // Unknown keys are collected before serde sees the document. serde's
        // `deny_unknown_fields` stops at the first one, so a config with three
        // typos would have `config doctor` name one and reveal the next only
        // after the author fixed it. The zod oracle's `.strict()` reports every
        // unrecognised key at once, and an author who has to run the command
        // three times to learn three facts has been told the truth three times
        // too slowly.
        let unknown: Vec<ConfigIssue> = value
            .as_mapping()
            .into_iter()
            .flatten()
            .filter_map(|(key, _)| key.as_str())
            .filter(|key| Self::key_kind(key) == KeyKind::Unknown)
            .map(|key| ConfigIssue {
                key_path: key.to_owned(),
                expected: "a key the quoin config schema declares".to_owned(),
                message: format!("unrecognized key {key:?}"),
            })
            .collect();
        if !unknown.is_empty() {
            return Err(unknown);
        }

        Self::deserialize(value.clone()).map_err(|e| {
            vec![ConfigIssue {
                key_path: String::new(),
                expected: "quoin config object".to_owned(),
                message: e.to_string(),
            }]
        })
    }

    fn key_kind(key_path: &str) -> KeyKind {
        match key_path {
            "org" => KeyKind::Scalar,
            _ => KeyKind::Unknown,
        }
    }

    fn json_schema() -> schemars::Schema {
        schemars::schema_for!(Self)
    }
}

/// The `ixSchema` convention (ix-cli-core FR-014) as a value rather than a
/// named module export: what a host's init hook would read off a loaded plugin
/// and hand to the schema registry.
#[derive(Debug, Clone)]
pub struct IxSchema {
    /// The namespace.
    pub id: PluginId,
    /// The env bindings.
    pub env: &'static [EnvBinding],
    /// The plugin's JSON Schema.
    pub config: schemars::Schema,
    /// Whether the schema refuses unknown keys. Registration requires `true`.
    pub strict: bool,
}

impl IxSchema {
    /// Build the declaration for a schema type.
    #[must_use]
    pub fn of<S: PluginConfigSchema>() -> Self {
        Self {
            id: S::plugin_id(),
            env: S::env_bindings(),
            config: S::json_schema(),
            strict: schema_is_strict(&S::json_schema()),
        }
    }
}

/// Whether a JSON Schema refuses unknown properties.
///
/// The zod oracle tests `_def.catchall.type === "never"`. `schemars` emits the
/// same fact as `additionalProperties: false`, which is what
/// `#[serde(deny_unknown_fields)]` produces.
#[must_use]
pub fn schema_is_strict(schema: &schemars::Schema) -> bool {
    schema
        .as_object()
        .and_then(|o| o.get("additionalProperties"))
        .and_then(serde_json::Value::as_bool)
        .is_some_and(|allowed| !allowed)
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

    fn yaml(s: &str) -> Yaml {
        serde_yaml_ng::from_str(s).expect("test yaml parses")
    }

    /// Trace: FR-027-AC-7
    #[test]
    fn tc_381_020_ix_schema_declares_id_and_env_binding() {
        let decl = IxSchema::of::<QuoinConfig>();
        assert_eq!(decl.id.as_str(), "quoin");
        assert_eq!(decl.env.len(), 1);
        assert_eq!(decl.env[0].key_path, "org");
        assert_eq!(decl.env[0].var_name, "QUOIN_ORG");
    }

    /// Trace: FR-027-AC-6
    #[test]
    fn tc_381_021_schema_is_strict() {
        assert!(
            IxSchema::of::<QuoinConfig>().strict,
            "a non-strict schema is refused at registration and would let \
             `config set` write an unknown key"
        );
    }

    /// Trace: FR-027-AC-6
    #[test]
    fn tc_381_022_absent_org_stays_absent() {
        let cfg = QuoinConfig::validate(&yaml("{}")).expect("empty object is valid");
        assert_eq!(cfg.org, None);
        // and it does not round-trip back as an explicit null
        assert_eq!(serde_yaml_ng::to_string(&cfg).expect("serializes"), "{}\n");
    }
}
