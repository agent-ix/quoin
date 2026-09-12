// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Plugin schema registration (quoin#381, FR-027-AC-7/AC-8).
//!
//! Reimplements ix-cli-core's `registerPluginSchema`: the in-process table a
//! host's init hook fills by walking its loaded plugins' `ixSchema` exports, so
//! `quoin config set org <name>` and `ix config set quoin org <name>` resolve
//! against the same schema and the same file.
//!
//! Two oracle behaviours are load-bearing and reproduced deliberately:
//!
//! * **A non-strict schema is refused at registration.** Strictness is what
//!   makes `config set` reject an unknown key instead of writing it.
//! * **A duplicate registration is a non-throwing failure that preserves the
//!   first entry.** quoin registers itself once per `config` subcommand when
//!   running standalone (no host init hook), so a second attempt must be
//!   harmless. Note the ix-cli-core `.d.ts` advertises an `"idempotent"`
//!   outcome that v0.12.0 never returns; this port follows the *implementation*,
//!   reporting [`RegistrationOutcome::Duplicate`].

use std::collections::BTreeMap;

use crate::error::ConfigError;
use crate::ids::{PackageName, PluginId};
use crate::schema::{EnvBinding, IxSchema, PluginConfigSchema};

/// What a registration attempt did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegistrationOutcome {
    /// The entry was added.
    Registered,
    /// An entry for this package (or its plugin id) already existed and was
    /// kept; nothing changed.
    Duplicate,
}

/// One registered plugin schema.
#[derive(Debug, Clone)]
pub struct RegisteredPluginSchema {
    /// The package the schema was registered for.
    pub package: PackageName,
    /// The namespace it owns.
    pub plugin_id: PluginId,
    /// Its env bindings.
    pub env_bindings: &'static [EnvBinding],
    /// Its JSON Schema.
    pub config: schemars::Schema,
}

/// The in-process schema table.
///
/// Owned and passed explicitly rather than being a process-global `static`: the
/// subprocess boundary runs one operation per process, and an owned registry
/// makes registration order visible at the call site and keeps tests hermetic.
#[derive(Debug, Default)]
pub struct PluginSchemaRegistry {
    by_package: BTreeMap<String, RegisteredPluginSchema>,
    by_plugin_id: BTreeMap<String, String>,
}

impl PluginSchemaRegistry {
    /// An empty registry.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Register `schema` for `package`.
    ///
    /// # Errors
    /// [`ConfigError::InvalidPackageName`] for an empty package name, and
    /// [`ConfigError::SchemaNotStrict`] when a non-strict schema is offered —
    /// strictness is a registration precondition, not a warning.
    pub fn register(
        &mut self,
        package: &PackageName,
        schema: IxSchema,
    ) -> Result<RegistrationOutcome, ConfigError> {
        if !schema.strict {
            return Err(ConfigError::SchemaNotStrict {
                id: schema.id.to_string(),
            });
        }
        if self.by_package.contains_key(package.as_str())
            || self.by_plugin_id.contains_key(schema.id.as_str())
        {
            return Ok(RegistrationOutcome::Duplicate);
        }
        self.by_plugin_id
            .insert(schema.id.to_string(), package.as_str().to_owned());
        self.by_package.insert(
            package.as_str().to_owned(),
            RegisteredPluginSchema {
                package: package.clone(),
                plugin_id: schema.id,
                env_bindings: schema.env,
                config: schema.config,
            },
        );
        Ok(RegistrationOutcome::Registered)
    }

    /// Register the schema type `S` for `package`, deriving nothing.
    ///
    /// # Errors
    /// As [`PluginSchemaRegistry::register`].
    pub fn register_schema<S: PluginConfigSchema>(
        &mut self,
        package: &PackageName,
    ) -> Result<RegistrationOutcome, ConfigError> {
        self.register(package, IxSchema::of::<S>())
    }

    /// Look up a registered schema by its namespace.
    ///
    /// # Errors
    /// [`ConfigError::UnknownPlugin`], carrying every registered id — the
    /// failure a standalone `quoin config` would hit if it forgot to register
    /// itself first.
    pub fn resolve(&self, id: &PluginId) -> Result<&RegisteredPluginSchema, ConfigError> {
        self.by_plugin_id
            .get(id.as_str())
            .and_then(|package| self.by_package.get(package))
            .ok_or_else(|| ConfigError::UnknownPlugin {
                id: id.to_string(),
                registered: self.registered_ids(),
            })
    }

    /// Every registered namespace, sorted.
    #[must_use]
    pub fn registered_ids(&self) -> Vec<String> {
        self.by_plugin_id.keys().cloned().collect()
    }

    /// Whether anything is registered.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.by_package.is_empty()
    }
}

/// Register quoin's own schema, the way `registerQuoinSchema()` does.
///
/// Under a host, the init hook registers every loaded plugin's `ixSchema`.
/// Standalone there is no host, so `quoin config` registers itself before
/// delegating — otherwise the shared handlers raise
/// [`ConfigError::UnknownPlugin`] for an id nothing has declared. Registration
/// is idempotent in effect: a duplicate is a non-error that keeps the first
/// entry.
///
/// # Errors
/// As [`PluginSchemaRegistry::register`]; in practice infallible for quoin's
/// own strict schema, but the result is returned rather than swallowed so a
/// future non-strict edit fails loudly at its first test.
pub fn register_quoin_schema(
    registry: &mut PluginSchemaRegistry,
) -> Result<RegistrationOutcome, ConfigError> {
    let package = PackageName::new(crate::schema::QUOIN_PACKAGE_NAME)?;
    registry.register_schema::<crate::schema::QuoinConfig>(&package)
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
    use crate::schema::QuoinConfig;

    /// Trace: FR-027-AC-7
    #[test]
    fn tc_381_030_registers_quoin_under_its_package_name() {
        let mut registry = PluginSchemaRegistry::new();
        assert_eq!(
            register_quoin_schema(&mut registry).expect("quoin's schema is strict"),
            RegistrationOutcome::Registered
        );
        let id = QuoinConfig::plugin_id();
        let entry = registry.resolve(&id).expect("just registered");
        assert_eq!(entry.package.as_str(), "@agent-ix/quoin");
        assert_eq!(entry.plugin_id.as_str(), "quoin");
        assert_eq!(entry.env_bindings[0].var_name, "QUOIN_ORG");
    }

    /// Trace: FR-027-AC-8
    #[test]
    fn tc_381_031_duplicate_registration_keeps_the_first_entry() {
        let mut registry = PluginSchemaRegistry::new();
        register_quoin_schema(&mut registry).expect("first registration");
        assert_eq!(
            register_quoin_schema(&mut registry).expect("second does not error"),
            RegistrationOutcome::Duplicate
        );
        assert_eq!(registry.registered_ids(), vec!["quoin".to_owned()]);
    }

    /// Trace: FR-027-AC-8
    #[test]
    fn tc_381_032_unknown_plugin_names_every_registered_id() {
        let registry = PluginSchemaRegistry::new();
        let err = registry
            .resolve(&QuoinConfig::plugin_id())
            .expect_err("nothing is registered");
        assert!(
            err.to_string().contains("<none>"),
            "message must name the empty registry, got: {err}"
        );
        let mut registry = PluginSchemaRegistry::new();
        register_quoin_schema(&mut registry).expect("registers");
        let other = PluginId::new("other").expect("legal id");
        let err = registry.resolve(&other).expect_err("not registered");
        assert!(err.to_string().contains("quoin"), "got: {err}");
    }

    /// Trace: FR-027-AC-6
    #[test]
    fn tc_381_033_non_strict_schema_is_refused() {
        let mut registry = PluginSchemaRegistry::new();
        let mut schema = IxSchema::of::<QuoinConfig>();
        schema.strict = false;
        let package = PackageName::new("@agent-ix/loose").expect("non-empty");
        assert!(registry.register(&package, schema).is_err());
        assert!(registry.is_empty());
    }
}
