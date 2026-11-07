// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The fallback record: what went wrong in a config read, and the in-process
//! log that holds it. Re-exported from [`super`] so the public paths are
//! unchanged.

use std::path::PathBuf;

use crate::error::ConfigIssue;
use crate::ids::PluginId;

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
