// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Source profiles and the selection files that choose one (FR-180, FR-182).
//!
//! A profile is a module data file (spec-objects-security FR-020). This module
//! reads it, reads the project and user selection files, and merges them into
//! the effective profile. It holds no opinion about a clause: applicability is
//! the engine's.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use quoin_quire_types::ClauseSetKey;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// The one accepted `schemaVersion` of a profile file.
const PROFILE_SCHEMA: &str = "source-profile-v1";

/// A module whose root the catalog located.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ModuleRef {
    /// The manifest name.
    pub(crate) name: String,
    /// The resolved module root.
    pub(crate) root: PathBuf,
}

/// Why a profile or a selection file was refused.
#[derive(Debug, thiserror::Error)]
pub(crate) enum ProfileError {
    /// The selection names a profile no module ships.
    #[error("profile `{name}` is not shipped by any installed module")]
    NotFound {
        /// The profile name.
        name: String,
    },
    /// Two modules ship the same profile name.
    #[error("profile `{name}` is shipped by both `{first}` and `{second}`")]
    Duplicate {
        /// The profile name.
        name: String,
        /// The first module.
        first: String,
        /// The second module.
        second: String,
    },
    /// A profile file breaks a rule of spec-objects-security FR-020.
    #[error("{}: {rule}", file.display())]
    BadProfile {
        /// The profile file.
        file: PathBuf,
        /// The rule it breaks.
        rule: String,
    },
    /// A selection file breaks a rule of FR-180 or FR-182.
    #[error("{}: {rule}", file.display())]
    BadSelection {
        /// The selection file.
        file: PathBuf,
        /// The rule it breaks.
        rule: String,
    },
    /// A file could not be read.
    #[error("{}: {reason}", file.display())]
    Unreadable {
        /// The file.
        file: PathBuf,
        /// The I/O failure.
        reason: String,
    },
}

/// Which file contributed a clause set to the effective profile.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum Layer {
    /// The selected profile.
    Profile,
    /// The project's selection file.
    Project,
    /// The user's selection file.
    User,
}

/// A validated profile file.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub(crate) struct Profile {
    schema_version: String,
    name: String,
    title: String,
    description: String,
    clause_sets: Vec<ClauseSetKey>,
    #[serde(default)]
    context: BTreeMap<String, String>,
}

/// A project selection file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Selection {
    /// The chosen profile name.
    pub(crate) profile: String,
    /// Context values that replace or extend the profile's.
    pub(crate) context: BTreeMap<String, String>,
    /// Extra clause-set keys.
    pub(crate) add: Vec<ClauseSetKey>,
}

/// The profile a run evaluates.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct EffectiveProfile {
    /// The profile name.
    pub(crate) name: String,
    /// The context sent to the engine for every set.
    pub(crate) context: BTreeMap<String, String>,
    /// The sets, in evaluation order, each with the layer that listed it.
    pub(crate) sets: Vec<(ClauseSetKey, Layer)>,
}

fn key_text(key: &ClauseSetKey) -> String {
    format!("{}/{}@{}", key.authority, key.id, key.version)
}

fn check_key(key: &ClauseSetKey) -> Result<(), String> {
    let empty = [&key.authority, &key.id, &key.version]
        .iter()
        .any(|field| field.trim().is_empty());
    if empty {
        return Err("a clause-set key has an empty authority, id or version".to_owned());
    }
    Ok(())
}

fn check_context(context: &BTreeMap<String, String>) -> Result<(), String> {
    for (dimension, value) in context {
        if dimension.trim().is_empty() || value.trim().is_empty() {
            return Err(format!(
                "context dimension `{dimension}` needs a non-empty name and value"
            ));
        }
    }
    Ok(())
}

/// Read and validate a profile file's `text`, naming `file` on a refusal.
///
/// `stem` is the file name without its extension; a profile's `name` must
/// equal it.
pub(crate) fn parse_profile(file: &Path, stem: &str, text: &str) -> Result<Profile, ProfileError> {
    let bad = |rule: String| ProfileError::BadProfile {
        file: file.to_path_buf(),
        rule,
    };
    let value = quoin_yaml::from_str(text).map_err(|e| bad(format!("not valid YAML: {e}")))?;
    let profile: Profile =
        serde_json::from_value(value).map_err(|e| bad(format!("not a source profile: {e}")))?;
    if profile.schema_version != PROFILE_SCHEMA {
        return Err(bad(format!("schemaVersion must be `{PROFILE_SCHEMA}`")));
    }
    if profile.name.trim().is_empty() || profile.name != stem {
        return Err(bad(format!(
            "`name` must be non-empty and equal the file stem `{stem}`"
        )));
    }
    if profile.title.trim().is_empty() || profile.description.trim().is_empty() {
        return Err(bad("`title` and `description` must be non-empty".to_owned()));
    }
    if profile.clause_sets.is_empty() {
        return Err(bad("`clauseSets` must list at least one key".to_owned()));
    }
    for (index, key) in profile.clause_sets.iter().enumerate() {
        check_key(key).map_err(&bad)?;
        if profile.clause_sets.iter().skip(index + 1).any(|k| k == key) {
            return Err(bad(format!("clause-set key {} is repeated", key_text(key))));
        }
    }
    check_context(&profile.context).map_err(bad)?;
    Ok(profile)
}

/// Find and read the profile `name` among the `profiles/` directories of
/// `modules`, and nowhere else.
pub(crate) fn load_profile(name: &str, modules: &[ModuleRef]) -> Result<Profile, ProfileError> {
    let mut found: Vec<(&ModuleRef, PathBuf)> = Vec::new();
    for module in modules {
        let file = module.root.join("profiles").join(format!("{name}.yaml"));
        if file.is_file() {
            found.push((module, file));
        }
    }
    let mut shipped = found.into_iter();
    let (Some((_, file)), second) = (shipped.next(), shipped.next()) else {
        return Err(ProfileError::NotFound {
            name: name.to_owned(),
        });
    };
    if let Some((other, _)) = second {
        let first = modules
            .iter()
            .find(|m| file.starts_with(&m.root))
            .map_or_else(String::new, |m| m.name.clone());
        return Err(ProfileError::Duplicate {
            name: name.to_owned(),
            first,
            second: other.name.clone(),
        });
    }
    let text = std::fs::read_to_string(&file).map_err(|e| ProfileError::Unreadable {
        file: file.clone(),
        reason: e.to_string(),
    })?;
    parse_profile(&file, name, &text)
}

fn selection_value(
    file: &Path,
    text: &str,
) -> Result<serde_json::Map<String, Value>, ProfileError> {
    let bad = |rule: String| ProfileError::BadSelection {
        file: file.to_path_buf(),
        rule,
    };
    match quoin_yaml::from_str(text).map_err(|e| bad(format!("not valid YAML: {e}")))? {
        Value::Object(map) => Ok(map),
        _ => Err(bad("must be a mapping".to_owned())),
    }
}

fn selection_context(
    file: &Path,
    map: &serde_json::Map<String, Value>,
) -> Result<BTreeMap<String, String>, ProfileError> {
    let bad = |rule: String| ProfileError::BadSelection {
        file: file.to_path_buf(),
        rule,
    };
    let mut context = BTreeMap::new();
    match map.get("context") {
        None | Some(Value::Null) => {}
        Some(Value::Object(entries)) => {
            for (dimension, value) in entries {
                let Value::String(value) = value else {
                    return Err(bad(format!(
                        "context dimension `{dimension}` must be a string"
                    )));
                };
                context.insert(dimension.clone(), value.clone());
            }
        }
        Some(_) => return Err(bad("`context` must be a mapping".to_owned())),
    }
    check_context(&context).map_err(bad)?;
    Ok(context)
}

fn selection_add(
    file: &Path,
    map: &serde_json::Map<String, Value>,
) -> Result<Vec<ClauseSetKey>, ProfileError> {
    let bad = |rule: String| ProfileError::BadSelection {
        file: file.to_path_buf(),
        rule,
    };
    let keys: Vec<ClauseSetKey> = match map.get("add") {
        None | Some(Value::Null) => Vec::new(),
        Some(list) => serde_json::from_value(list.clone())
            .map_err(|e| bad(format!("`add` must list clause-set keys: {e}")))?,
    };
    for key in &keys {
        check_key(key).map_err(&bad)?;
    }
    Ok(keys)
}

/// Read a project selection file (FR-180).
pub(crate) fn parse_selection(file: &Path, text: &str) -> Result<Selection, ProfileError> {
    let bad = |rule: String| ProfileError::BadSelection {
        file: file.to_path_buf(),
        rule,
    };
    let map = selection_value(file, text)?;
    if let Some(key) = map
        .keys()
        .find(|key| !["profile", "context", "add"].contains(&key.as_str()))
    {
        return Err(bad(format!("undefined key `{key}`")));
    }
    let profile = match map.get("profile") {
        Some(Value::String(name)) if !name.trim().is_empty() => name.clone(),
        _ => return Err(bad("`profile` must name a profile".to_owned())),
    };
    Ok(Selection {
        profile,
        context: selection_context(file, &map)?,
        add: selection_add(file, &map)?,
    })
}

/// Read the user selection file, which carries `add` only (FR-182).
pub(crate) fn parse_user_add(file: &Path, text: &str) -> Result<Vec<ClauseSetKey>, ProfileError> {
    let map = selection_value(file, text)?;
    if let Some(key) = map.keys().find(|key| key.as_str() != "add") {
        return Err(ProfileError::BadSelection {
            file: file.to_path_buf(),
            rule: format!("the user file carries `add` only, not `{key}`"),
        });
    }
    selection_add(file, &map)
}

/// Merge a profile, the project selection and the user `add` list.
///
/// A repository `context` value replaces the profile's for that dimension. The
/// sets are the profile's, then project `add`, then user `add`, each exact key
/// kept once at its first position.
pub(crate) fn effective(
    profile: &Profile,
    selection: &Selection,
    user_add: &[ClauseSetKey],
) -> EffectiveProfile {
    let mut context = profile.context.clone();
    context.extend(selection.context.clone());
    let mut sets: Vec<(ClauseSetKey, Layer)> = Vec::new();
    let layers = [
        (profile.clause_sets.as_slice(), Layer::Profile),
        (selection.add.as_slice(), Layer::Project),
        (user_add, Layer::User),
    ];
    for (keys, layer) in layers {
        for key in keys {
            if !sets.iter().any(|(seen, _)| seen == key) {
                sets.push((key.clone(), layer));
            }
        }
    }
    EffectiveProfile {
        name: profile.name.clone(),
        context,
        sets,
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]
mod tests;
