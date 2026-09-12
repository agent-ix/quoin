// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

#![forbid(unsafe_code)]
#![warn(missing_docs, clippy::pedantic)]

//! Quoin's persistent configuration, plugin-schema registration and
//! authoring-organization resolution (quoin#381, Stage 7 of the EPIC #373
//! Rust burn-down).
//!
//! This crate is the Rust successor to `src/config-schema.ts` and `src/org.ts`,
//! and to the nine-plus `@agent-ix/ix-cli-core` symbols quoin reaches for. It
//! deliberately **does not** port ix-cli-core: quoin uses none of its auth,
//! secrets, marketplace or self-update code, and reimplementing only the
//! behaviours quoin depends on keeps that dependency retirable.
//!
//! # What is reimplemented from ix-cli-core
//!
//! | ix-cli-core symbol | Reimplemented as |
//! |---|---|
//! | `ConfigService.forPlugin` / `.get` / `.set` | [`service::ConfigService`] |
//! | `registerPluginSchema` | [`plugin_registry::PluginSchemaRegistry`] |
//! | `runConfigGet` / `runConfigSet` / `runConfigDoctor` | [`service::ConfigService::get`] / [`service::ConfigService::set_key`] / [`service::ConfigService::doctor`] |
//! | `UnknownPluginError` | [`error::ConfigError::UnknownPlugin`] |
//!
//! `runConfigEdit`, `BaseCommand`, `loadConfig`, `run`, `maybeOfferUpdate`,
//! `runSelfUpdate` and `listCorePlugins` are CLI-shell concerns and belong to
//! Stage 9 (oclif → clap); nothing here depends on them.
//!
//! # Blocking, by design
//!
//! Every operation is a bounded local-file read or an atomic rename. `quoin-core`
//! is a short-lived subprocess running one command-shaped operation, so there is
//! no concurrency to overlap; an async runtime would add a `block_on` bridge at
//! every call site and buy nothing. Resource safety comes from explicit bounds
//! ([`service::MAX_CONFIG_FILE_BYTES`], [`org::MAX_GIT_CONFIG_BYTES`]) rather
//! than from cancellation.

pub mod error;
pub mod ids;
mod lock;
pub mod org;
pub mod paths;
pub mod plugin_registry;
pub mod schema;
pub mod service;

pub use error::{ConfigError, ConfigErrorCode, ConfigIssue};
pub use ids::{OrgName, PackageName, PluginId};
pub use org::{
    OrgOptions, OrgSource, ResolvedOrg, UNRESOLVED_ORG_MESSAGE, org_from_git_config, origin_org,
    resolve_org,
};
pub use paths::{
    Environment, FixedEnvironment, ProcessEnvironment, RuntimeContext, cache_root, config_path_for,
    config_root,
};
pub use plugin_registry::{
    PluginSchemaRegistry, RegisteredPluginSchema, RegistrationOutcome, register_quoin_schema,
};
pub use schema::{
    IxSchema, PluginConfigSchema, QUOIN_ENV_BINDINGS, QUOIN_PACKAGE_NAME, QUOIN_PLUGIN_ID,
    QuoinConfig,
};
pub use service::{ConfigIncident, ConfigService, DoctorEntry, DoctorStatus, IncidentLog};
