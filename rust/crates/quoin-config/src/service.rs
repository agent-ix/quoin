// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! `ConfigService` — layered, total configuration reads and validated writes
//! (quoin#381, FR-027).
//!
//! Reimplements the one ix-cli-core behaviour quoin actually depends on:
//!
//! * **Layering**, lowest to highest: schema defaults < user file
//!   (`<configRoot>/config.d/<id>.yaml`) < project file
//!   (`<projectRoot>/config.d/<id>.yaml`) < environment bindings. Layers are
//!   deep-merged **as untyped documents and validated once at the end**, so an
//!   unknown key in *any* layer invalidates the whole merged config — that is
//!   what the strict schema means, and what the TS oracle does.
//! * **Totality of [`ConfigService::get`]**: it never fails. A missing,
//!   unreadable, unparsable or invalid file yields schema defaults plus a
//!   recorded [`ConfigIncident`], because an author must be able to keep
//!   writing specs with a broken config file.
//!
//! Incidents are process-local and never persisted, matching the oracle: they
//! exist so `config doctor` can explain a silent fallback in the same run.
//!
//! **Blocking by design.** Every operation here is a small local file read or
//! an atomic rename. `quoin-core` is a short-lived subprocess running one
//! command-shaped operation, so there is no concurrency to overlap and an async
//! runtime would add a `block_on` bridge at every call site for no throughput.

use std::fs;
use std::io::Write as _;
use std::marker::PhantomData;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde_yaml_ng::{Mapping, Value as Yaml};

use crate::error::{ConfigError, ConfigIssue};
use crate::ids::PluginId;
use crate::paths::{Environment, RuntimeContext, config_path_for, project_config_path_for};
use crate::schema::{KeyKind, PluginConfigSchema};

/// The header every managed config file carries.
///
/// Byte-for-byte ix-cli-core's header, and deliberately so. Both
/// implementations write the same `<configRoot>/config.d/<id>.yaml` during
/// coexistence; a header of our own would be rewritten by the next TypeScript
/// write and rewritten back by the one after, churning the file forever on a
/// line neither implementation reads.
const MANAGED_HEADER: &str = "# Managed by ix — `ix config edit` to modify safely.\n";

/// Upper bound on a config file, in bytes.
///
/// A config file is a handful of scalar keys. The bound exists so a truncated,
/// corrupt or hostile file (a symlink swapped for a multi-gigabyte log, a fifo)
/// is refused as an I/O incident rather than read into memory.
pub const MAX_CONFIG_FILE_BYTES: u64 = 1 << 20;

/// What kind of problem produced a fallback.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum IncidentKind {
    /// The file could not be read.
    Io,
    /// The file was not parsable YAML, or its top level was not a mapping.
    Parse,
    /// The merged document failed schema validation, or a registration clashed.
    Schema,
}

impl IncidentKind {
    /// The stable wire string.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Io => "io",
            Self::Parse => "parse",
            Self::Schema => "schema",
        }
    }
}

impl std::fmt::Display for IncidentKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// A recorded fallback: what went wrong, where, and when.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigIncident {
    /// The namespace whose read fell back.
    pub plugin_id: PluginId,
    /// The file the problem is attributed to.
    pub file_path: PathBuf,
    /// The class of problem.
    pub kind: IncidentKind,
    /// Human-readable detail.
    pub detail: String,
    /// Schema issues, when `kind` is [`IncidentKind::Schema`].
    pub issues: Vec<ConfigIssue>,
    /// Milliseconds since the Unix epoch when the incident was observed.
    pub observed_at_ms: u128,
}

/// An in-process log of configuration fallbacks.
///
/// Owned by the caller and passed in, rather than a process-global: a
/// subprocess boundary that runs one operation per process has no use for
/// hidden global state, and an owned log keeps tests independent.
#[derive(Debug, Clone, Default)]
pub struct IncidentLog {
    incidents: Vec<ConfigIncident>,
}

impl IncidentLog {
    /// An empty log.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Record an incident.
    pub fn record(&mut self, incident: ConfigIncident) {
        self.incidents.push(incident);
    }

    /// Every incident, in the order observed.
    #[must_use]
    pub fn incidents(&self) -> &[ConfigIncident] {
        &self.incidents
    }

    /// Drop every incident recorded for `plugin_id` — called after a successful
    /// write, which is what made the previous fallback stale.
    pub fn clear_for_plugin(&mut self, plugin_id: &PluginId) {
        self.incidents.retain(|i| &i.plugin_id != plugin_id);
    }

    /// Whether anything is recorded.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.incidents.is_empty()
    }
}

/// The outcome of a read: the typed config plus how it was reached.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Resolved<T> {
    /// The validated configuration, or schema defaults on any fallback.
    pub value: T,
    /// Whether a layer fell back rather than contributing its content.
    pub degraded: bool,
}

/// A layered configuration reader/writer for one plugin namespace.
pub struct ConfigService<'a, S: PluginConfigSchema> {
    plugin_id: PluginId,
    user_path: PathBuf,
    project_path: Option<PathBuf>,
    env: &'a dyn Environment,
    schema: PhantomData<S>,
}

impl<'a, S: PluginConfigSchema + Default + serde::Serialize> ConfigService<'a, S> {
    /// Build the service for `S` in the given environment and runtime context.
    #[must_use]
    pub fn for_plugin(env: &'a dyn Environment, ctx: &RuntimeContext) -> Self {
        let plugin_id = S::plugin_id();
        Self {
            user_path: config_path_for(env, ctx, &plugin_id),
            project_path: project_config_path_for(ctx, &plugin_id),
            plugin_id,
            env,
            schema: PhantomData,
        }
    }

    /// The user-level file this service reads and writes.
    #[must_use]
    pub fn file_path(&self) -> &Path {
        &self.user_path
    }

    /// The project-level file, when a project layer applies.
    #[must_use]
    pub fn project_file_path(&self) -> Option<&Path> {
        self.project_path.as_deref()
    }

    /// The namespace this service owns.
    #[must_use]
    pub fn plugin_id(&self) -> &PluginId {
        &self.plugin_id
    }

    /// Read the merged, validated configuration.
    ///
    /// Total: every failure path records an incident on `log` and yields schema
    /// defaults. There is no error return, deliberately — a broken config file
    /// must not stop an author writing specs (FR-027-AC-5).
    pub fn get(&self, log: &mut IncidentLog) -> Resolved<S> {
        let mut degraded = false;
        let mut merged = Mapping::new();

        // Lowest layer first: the user file, then the project file over it.
        for path in [Some(self.user_path.clone()), self.project_path.clone()]
            .into_iter()
            .flatten()
        {
            match read_mapping(&path) {
                Ok(Some(map)) => deep_merge(&mut merged, &map),
                Ok(None) => {}
                Err(err) => {
                    degraded = true;
                    log.record(incident_for(&self.plugin_id, &path, &err));
                }
            }
        }

        apply_env_layer(&mut merged, S::env_bindings(), self.env);

        match S::validate(&Yaml::Mapping(merged)) {
            Ok(value) => Resolved { value, degraded },
            Err(issues) => {
                log.record(ConfigIncident {
                    plugin_id: self.plugin_id.clone(),
                    file_path: self.user_path.clone(),
                    kind: IncidentKind::Schema,
                    detail: format!("schema validation failed ({} issue(s))", issues.len()),
                    issues,
                    observed_at_ms: now_ms(),
                });
                Resolved {
                    value: S::default(),
                    degraded: true,
                }
            }
        }
    }

    /// Read one dotted key out of the merged configuration.
    ///
    /// `Ok(None)` means the key is declared but unset — `config get` reports
    /// that as a successful "unset", not as a failure.
    ///
    /// # Errors
    /// [`ConfigError::UnknownConfigKey`] when the schema declares no such key.
    /// Reading a *declared* key never fails, because [`ConfigService::get`]
    /// never fails.
    pub fn get_key(
        &self,
        key_path: &str,
        log: &mut IncidentLog,
    ) -> Result<Option<Yaml>, ConfigError> {
        if S::key_kind(key_path) == KeyKind::Unknown {
            return Err(ConfigError::UnknownConfigKey {
                plugin_id: self.plugin_id.to_string(),
                key_path: key_path.to_owned(),
            });
        }
        let resolved = self.get(log).value;
        let Yaml::Mapping(map) =
            serde_yaml_ng::to_value(&resolved).map_err(|e| ConfigError::ConfigSchema {
                plugin_id: self.plugin_id.to_string(),
                path: self.user_path.clone(),
                issues: vec![ConfigIssue {
                    key_path: key_path.to_owned(),
                    expected: "serializable config".to_owned(),
                    message: e.to_string(),
                }],
            })?
        else {
            return Ok(None);
        };
        Ok(get_at_path(&map, key_path).cloned())
    }

    /// Merge `partial` into the user-level file and write it atomically.
    ///
    /// # Errors
    /// [`ConfigError::ConfigSchema`] when the result would not validate,
    /// [`ConfigError::ConfigSymlinkRefused`] when the target is a symlink, and
    /// [`ConfigError::ConfigWrite`] / [`ConfigError::ConfigIo`] on I/O failure.
    pub fn set(&self, partial: &Mapping, log: &mut IncidentLog) -> Result<S, ConfigError> {
        // The whole read-merge-write runs under the advisory sentinel
        // ix-cli-core takes for the same file. Without it two interleaved
        // read-merge-write pairs — ours and the TypeScript writer's, during
        // coexistence — silently drop whichever key the loser merged.
        let validated = crate::lock::with_config_lock(&self.user_path, || {
            // An unreadable or unparsable existing file is *replaced*, not
            // reported. This matches the oracle, whose `set` wraps its read in
            // `try { … } catch { t = {} }`: a corrupt config must not make the
            // command that would repair it impossible to run.
            let mut next = read_mapping(&self.user_path)
                .unwrap_or_default()
                .unwrap_or_default();
            deep_merge(&mut next, partial);

            let validated = S::validate(&Yaml::Mapping(next.clone())).map_err(|issues| {
                ConfigError::ConfigSchema {
                    plugin_id: self.plugin_id.to_string(),
                    path: self.user_path.clone(),
                    issues,
                }
            })?;

            write_yaml_atomically(&self.user_path, &next)?;
            Ok(validated)
        })?;
        log.clear_for_plugin(&self.plugin_id);
        Ok(validated)
    }

    /// Set one dotted key from a raw command-line string.
    ///
    /// Unlike the TypeScript oracle this writes **only the named key**, not the
    /// whole resolved object — see the crate README note on
    /// `runConfigSet`. An env- or project-supplied value must not be baked into
    /// the user file as a side effect of setting an unrelated key.
    ///
    /// # Errors
    /// [`ConfigError::UnknownConfigKey`] when the schema declares no such key,
    /// [`ConfigError::ConfigSetParse`] when a complex key's value is not JSON,
    /// plus everything [`ConfigService::set`] can return.
    pub fn set_key(
        &self,
        key_path: &str,
        raw_value: &str,
        log: &mut IncidentLog,
    ) -> Result<S, ConfigError> {
        let value = match S::key_kind(key_path) {
            KeyKind::Scalar => Yaml::String(raw_value.to_owned()),
            KeyKind::Complex => {
                let parsed: serde_json::Value =
                    serde_json::from_str(raw_value).map_err(|e| ConfigError::ConfigSetParse {
                        key_path: key_path.to_owned(),
                        expected: "object or array".to_owned(),
                        detail: e.to_string(),
                    })?;
                serde_yaml_ng::to_value(parsed).map_err(|e| ConfigError::ConfigSetParse {
                    key_path: key_path.to_owned(),
                    expected: "object or array".to_owned(),
                    detail: e.to_string(),
                })?
            }
            KeyKind::Unknown => {
                return Err(ConfigError::UnknownConfigKey {
                    plugin_id: self.plugin_id.to_string(),
                    key_path: key_path.to_owned(),
                });
            }
        };
        let mut partial = Mapping::new();
        set_at_path(&mut partial, key_path, value);
        self.set(&partial, log)
    }

    /// Delete the user-level config file, tolerating its absence.
    ///
    /// # Errors
    /// [`ConfigError::ConfigWrite`] when the file exists but cannot be removed.
    pub fn reset(&self, log: &mut IncidentLog) -> Result<(), ConfigError> {
        match fs::remove_file(&self.user_path) {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(source) => {
                return Err(ConfigError::ConfigWrite {
                    path: self.user_path.clone(),
                    source,
                });
            }
        }
        log.clear_for_plugin(&self.plugin_id);
        Ok(())
    }

    /// Inspect the user-level file the way `config doctor` reports it.
    ///
    /// Only the user layer is inspected, matching the oracle: project `.ix`
    /// files are invisible to doctor.
    #[must_use]
    pub fn doctor(&self) -> DoctorEntry {
        let raw = match read_mapping(&self.user_path) {
            Ok(Some(map)) => map,
            Ok(None) => Mapping::new(),
            Err(err) => {
                return DoctorEntry {
                    plugin_id: self.plugin_id.clone(),
                    file_path: self.user_path.clone(),
                    status: DoctorStatus::Invalid {
                        issues: vec![ConfigIssue {
                            key_path: String::new(),
                            expected: match err.code() {
                                crate::error::ConfigErrorCode::ConfigIo => {
                                    "readable file".to_owned()
                                }
                                _ => "valid YAML object".to_owned(),
                            },
                            message: err.to_string(),
                        }],
                    },
                };
            }
        };
        let key_count = raw.len();
        match S::validate(&Yaml::Mapping(raw)) {
            Ok(_) => DoctorEntry {
                plugin_id: self.plugin_id.clone(),
                file_path: self.user_path.clone(),
                status: DoctorStatus::Valid { key_count },
            },
            Err(issues) => DoctorEntry {
                plugin_id: self.plugin_id.clone(),
                file_path: self.user_path.clone(),
                status: DoctorStatus::Invalid { issues },
            },
        }
    }
}

/// One plugin's line in a `config doctor` report.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DoctorEntry {
    /// The namespace.
    pub plugin_id: PluginId,
    /// The user-level file inspected.
    pub file_path: PathBuf,
    /// The verdict.
    pub status: DoctorStatus,
}

/// A doctor verdict.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DoctorStatus {
    /// The file (or its absence) validates.
    Valid {
        /// Top-level key count in the raw file.
        key_count: usize,
    },
    /// The file does not validate, or could not be read.
    Invalid {
        /// Why.
        issues: Vec<ConfigIssue>,
    },
    /// A `config.d/*.yaml` file exists for a namespace nothing has registered.
    Unregistered,
}

/// Read a config file into a mapping.
///
/// `Ok(None)` means "absent", which is not a problem. Every other failure is an
/// error the caller turns into an incident.
fn read_mapping(path: &Path) -> Result<Option<Mapping>, ConfigError> {
    let meta = match fs::symlink_metadata(path) {
        Ok(m) => m,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(source) => {
            return Err(ConfigError::ConfigIo {
                path: path.to_path_buf(),
                source,
            });
        }
    };
    // A config file is a few scalars. Bounding the read keeps a corrupt or
    // hostile file (or a fifo swapped in for one) from being slurped whole.
    if meta.is_file() && meta.len() > MAX_CONFIG_FILE_BYTES {
        return Err(ConfigError::ConfigIo {
            path: path.to_path_buf(),
            source: std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!(
                    "config file is {} bytes, over the {MAX_CONFIG_FILE_BYTES}-byte limit",
                    meta.len()
                ),
            ),
        });
    }
    let text = match fs::read_to_string(path) {
        Ok(t) => t,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(source) => {
            return Err(ConfigError::ConfigIo {
                path: path.to_path_buf(),
                source,
            });
        }
    };
    let parsed: Yaml = serde_yaml_ng::from_str(&text).map_err(|e| ConfigError::ConfigParse {
        path: path.to_path_buf(),
        detail: e.to_string(),
    })?;
    match parsed {
        Yaml::Null => Ok(None),
        Yaml::Mapping(map) => Ok(Some(map)),
        other => Err(ConfigError::ConfigNotAMapping {
            path: path.to_path_buf(),
            found: yaml_kind(&other),
        }),
    }
}

/// The name of a YAML value's kind, for diagnostics.
const fn yaml_kind(value: &Yaml) -> &'static str {
    match value {
        Yaml::Null => "null",
        Yaml::Bool(_) => "boolean",
        Yaml::Number(_) => "number",
        Yaml::String(_) => "string",
        Yaml::Sequence(_) => "array",
        Yaml::Mapping(_) => "object",
        Yaml::Tagged(_) => "tagged",
    }
}

fn incident_for(plugin_id: &PluginId, path: &Path, err: &ConfigError) -> ConfigIncident {
    use crate::error::ConfigErrorCode as Code;
    let kind = match err.code() {
        Code::ConfigIo => IncidentKind::Io,
        Code::ConfigParse | Code::ConfigNotAMapping => IncidentKind::Parse,
        _ => IncidentKind::Schema,
    };
    ConfigIncident {
        plugin_id: plugin_id.clone(),
        file_path: path.to_path_buf(),
        kind,
        detail: err.to_string(),
        issues: Vec::new(),
        observed_at_ms: now_ms(),
    }
}

fn now_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_millis())
}

/// Recursively merge `overlay` into `base`. Mappings merge key-wise; every
/// other value replaces wholesale, matching the oracle's `deepMerge`.
pub fn deep_merge(base: &mut Mapping, overlay: &Mapping) {
    for (key, value) in overlay {
        match (base.get_mut(key), value) {
            (Some(Yaml::Mapping(existing)), Yaml::Mapping(incoming)) => {
                deep_merge(existing, incoming);
            }
            _ => {
                base.insert(key.clone(), value.clone());
            }
        }
    }
}

/// Write `value` at a dotted path, creating intermediate mappings.
pub fn set_at_path(map: &mut Mapping, key_path: &str, value: Yaml) {
    let mut segments = key_path.split('.').peekable();
    let mut cursor = map;
    while let Some(segment) = segments.next() {
        let key = Yaml::String(segment.to_owned());
        if segments.peek().is_none() {
            cursor.insert(key, value);
            return;
        }
        let entry = cursor
            .entry(key)
            .or_insert_with(|| Yaml::Mapping(Mapping::new()));
        if !entry.is_mapping() {
            *entry = Yaml::Mapping(Mapping::new());
        }
        let Yaml::Mapping(next) = entry else {
            // `entry` was just forced to a mapping above; this arm is
            // unreachable, and returning keeps it panic-free regardless.
            return;
        };
        cursor = next;
    }
}

/// Read a dotted path out of a mapping.
///
/// Yields `None` when any hop is absent or is not a mapping — the same plain
/// descent `config get` performs.
#[must_use]
pub fn get_at_path<'m>(map: &'m Mapping, key_path: &str) -> Option<&'m Yaml> {
    let mut current = map;
    let mut segments = key_path.split('.').peekable();
    while let Some(segment) = segments.next() {
        let value = current.get(Yaml::String(segment.to_owned()))?;
        if segments.peek().is_none() {
            return Some(value);
        }
        let Yaml::Mapping(next) = value else {
            return None;
        };
        current = next;
    }
    None
}

/// Layer every bound environment variable over `merged`.
///
/// A variable that is *set at all* wins, empty string included — matching
/// `process.env[name] !== undefined`. The raw string is written unconverted;
/// the schema is what coerces or refuses it.
fn apply_env_layer(
    merged: &mut Mapping,
    bindings: &[crate::schema::EnvBinding],
    env: &dyn Environment,
) {
    for binding in bindings {
        if let Some(raw) = env.var(binding.var_name) {
            set_at_path(merged, binding.key_path, Yaml::String(raw));
        }
    }
}

/// Write a mapping to `path` atomically: refuse a symlinked target, write a
/// sibling temp file with restrictive permissions, fsync, then rename.
fn write_yaml_atomically(path: &Path, value: &Mapping) -> Result<(), ConfigError> {
    if let Ok(meta) = fs::symlink_metadata(path)
        && meta.file_type().is_symlink()
    {
        return Err(ConfigError::ConfigSymlinkRefused {
            path: path.to_path_buf(),
        });
    }
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(parent).map_err(|source| ConfigError::ConfigWrite {
        path: path.to_path_buf(),
        source,
    })?;

    let body = serde_yaml_ng::to_string(value).map_err(|e| ConfigError::ConfigWrite {
        path: path.to_path_buf(),
        source: std::io::Error::other(e.to_string()),
    })?;
    let contents = format!("{MANAGED_HEADER}{body}");

    let tmp = path.with_extension(format!("tmp.{}", std::process::id()));
    let write = || -> std::io::Result<()> {
        let mut file = fs::File::create(&tmp)?;
        set_owner_only(&file)?;
        file.write_all(contents.as_bytes())?;
        file.sync_all()?;
        drop(file);
        fs::rename(&tmp, path)
    };
    write().map_err(|source| {
        let _ = fs::remove_file(&tmp);
        ConfigError::ConfigWrite {
            path: path.to_path_buf(),
            source,
        }
    })
}

#[cfg(unix)]
fn set_owner_only(file: &fs::File) -> std::io::Result<()> {
    use std::os::unix::fs::PermissionsExt as _;
    file.set_permissions(fs::Permissions::from_mode(0o600))
}

#[cfg(not(unix))]
fn set_owner_only(_file: &fs::File) -> std::io::Result<()> {
    Ok(())
}
