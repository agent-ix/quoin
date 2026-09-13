// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! One `ConfigError`, one exit status (quoin#446, scope item 3).
//!
//! The sibling of [`crate::ops::modules::taxonomy`], for the other upstream
//! crate this domain is a shell over. #446 asked for **both** upstream enums to
//! be guarded by a pinned `ALL.len()` count; `ModulesErrorCode` got one and
//! `ConfigErrorCode` did not, which the independent review of quoin#450 (finding
//! 7) recorded as half a deliverable.
//!
//! # Why it exists before an operation needs it
//!
//! No operation wired today produces a [`ConfigError`]: `config.resolve_org`
//! takes its documents by content and answers over them, and a layer that does
//! not parse is a *degradation* reported as [`CoreErrorCode::Degraded`], not a
//! `ConfigError`. Stage 9 wires `config.set` and `config.get`, which produce
//! them by the dozen.
//!
//! Writing the mapping now is the point rather than a cost. The guard #446
//! asked for is a guard against a code being added upstream and landing in a
//! wildcard arm unnoticed; a guard that appears in the same commit as the first
//! operation that needs it has never once been the thing that caught the
//! addition. The count below fails the moment `quoin-config` gains a code,
//! which is when the decision is cheap to make.

use quoin_config::{ConfigError, ConfigErrorCode};

use crate::error::{CoreError, CoreErrorCode};

/// Map a `quoin-config` failure onto the boundary's exit taxonomy.
///
/// The ONE place a [`ConfigError`] becomes an exit status, in the three groups
/// [`crate::ops::modules::taxonomy::map_error`] uses, for the same reasons:
///
/// - **`BadRequest` (3)** — the caller's own words were wrong: an illegal
///   plugin id, an unparsable `config set` value, a key the schema does not
///   declare, an empty org name. Fixing it means editing the request.
/// - **`Refused` (2)** — the request was understood and a stated rule declined
///   it: a config file that does not parse, a top-level value that is not a
///   mapping, a merged document that fails its schema, a write through a
///   symlink, a lock that could not be taken. Fixing it means changing the
///   world, not the request.
/// - **`Io` → Internal (4)** — quoin's own machinery failed: a file that could
///   not be read or written, a schema registered twice, a schema offered that
///   is not strict. None of these is reachable by editing a request.
///
/// [`ConfigErrorCode`] is `#[non_exhaustive]`, so this match needs a `_` arm
/// and the compiler will NOT flag a code added upstream. The count pin in this
/// module is what does.
#[allow(
    dead_code,
    reason = "the mapping Stage 9's config.set/config.get will call; it exists \
              now so the count pin #446 asked for guards the enum from today \
              rather than from the commit that first needs it"
)]
pub(crate) fn map_error(error: &ConfigError, op: &'static str) -> CoreError {
    let code = error.code();
    let mapped = core_code(code);
    let envelope = CoreError::new(mapped.unwrap_or(CoreErrorCode::Io), error.to_string())
        .with_context("op", op)
        .with_context("config_code", code.as_str());
    match mapped {
        Some(_) => envelope,
        // A code this build has no opinion on. It exits Internal (4), and it
        // SAYS it was unmapped rather than posing as a considered answer.
        None => envelope.with_context("mapping", "unrecognised"),
    }
}

/// The exit-taxonomy code one [`ConfigErrorCode`] maps to, or `None` for a code
/// this build does not know.
///
/// `Option` rather than a total function, for the reason
/// [`crate::ops::modules::taxonomy::core_code`] states: "unknown upstream code"
/// and "deliberately Internal" are different facts, and collapsing them makes
/// the wildcard arm indistinguishable from a considered decision.
fn core_code(code: ConfigErrorCode) -> Option<CoreErrorCode> {
    match code {
        ConfigErrorCode::InvalidPluginId
        | ConfigErrorCode::InvalidPackageName
        | ConfigErrorCode::UnderivablePluginId
        | ConfigErrorCode::UnknownPlugin
        | ConfigErrorCode::ConfigSetParse
        | ConfigErrorCode::UnknownConfigKey
        | ConfigErrorCode::InvalidOrgName => Some(CoreErrorCode::BadRequest),
        ConfigErrorCode::ConfigParse
        | ConfigErrorCode::ConfigNotAMapping
        | ConfigErrorCode::ConfigSchema
        | ConfigErrorCode::ConfigSymlinkRefused
        | ConfigErrorCode::ConfigLockTimeout => Some(CoreErrorCode::Refused),
        ConfigErrorCode::ConfigIo
        | ConfigErrorCode::ConfigWrite
        | ConfigErrorCode::DuplicateRegistration
        | ConfigErrorCode::SchemaNotStrict => Some(CoreErrorCode::Io),
        _ => None,
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]
mod tests {
    use super::{ConfigErrorCode, CoreErrorCode, core_code, map_error};

    /// `ConfigErrorCode` is `#[non_exhaustive]`, so `core_code`'s `_` arm means
    /// the compiler cannot catch a code added upstream. This pins the COUNT
    /// instead, and lists every group by name.
    ///
    /// A loop over `all()` is deliberately NOT what this does: such a loop
    /// re-runs the same match and agrees with itself, which is the quoin#443
    /// failure mode and the reason #446 asked for a count in the first place.
    ///
    /// When this fails: a code was added to `quoin-config`. Decide which of the
    /// three groups it belongs to, add it to that arm, and raise the count.
    #[test]
    fn the_error_mapping_covers_every_config_code() {
        assert_eq!(
            ConfigErrorCode::all().len(),
            16,
            "quoin-config gained or lost an error code; map it in core_code deliberately"
        );

        let bad_request = [
            ConfigErrorCode::InvalidPluginId,
            ConfigErrorCode::InvalidPackageName,
            ConfigErrorCode::UnderivablePluginId,
            ConfigErrorCode::UnknownPlugin,
            ConfigErrorCode::ConfigSetParse,
            ConfigErrorCode::UnknownConfigKey,
            ConfigErrorCode::InvalidOrgName,
        ];
        let refused = [
            ConfigErrorCode::ConfigParse,
            ConfigErrorCode::ConfigNotAMapping,
            ConfigErrorCode::ConfigSchema,
            ConfigErrorCode::ConfigSymlinkRefused,
            ConfigErrorCode::ConfigLockTimeout,
        ];
        let internal = [
            ConfigErrorCode::ConfigIo,
            ConfigErrorCode::ConfigWrite,
            ConfigErrorCode::DuplicateRegistration,
            ConfigErrorCode::SchemaNotStrict,
        ];
        assert_eq!(
            bad_request.len() + refused.len() + internal.len(),
            ConfigErrorCode::all().len(),
            "a code is in `all()` but in none of the three groups"
        );

        for code in bad_request {
            assert_eq!(core_code(code), Some(CoreErrorCode::BadRequest), "{code}");
        }
        for code in refused {
            assert_eq!(core_code(code), Some(CoreErrorCode::Refused), "{code}");
        }
        for code in internal {
            assert_eq!(core_code(code), Some(CoreErrorCode::Io), "{code}");
        }
    }

    /// The envelope carries the config code through, so an operator can tell
    /// which rule declined without parsing the sentence.
    #[test]
    fn the_envelope_names_the_config_code_that_declined() {
        let error = quoin_config::ConfigError::InvalidOrgName;
        let envelope = map_error(&error, "config.set");
        assert_eq!(envelope.code, CoreErrorCode::BadRequest);
        assert_eq!(envelope.outcome().code(), 3);
        assert_eq!(envelope.context["op"], "config.set");
        assert_eq!(envelope.context["config_code"], "QC014_INVALID_ORG_NAME");
        assert!(!envelope.context.contains_key("mapping"));
    }
}
