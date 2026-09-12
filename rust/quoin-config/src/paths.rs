// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Where configuration lives (quoin#381, FR-027).
//!
//! Reimplements ix-cli-core's `configRoot` / `configPathFor` / `cacheRoot`
//! layout. The one fact — *a plugin's config is one YAML file per plugin under
//! `<configRoot>/config.d/`, except the reserved `core` id which is
//! `<configRoot>/config.yaml`* — is written here once and nowhere else.
//!
//! Ambient process state (`$XDG_CONFIG_HOME`, `$HOME`) is read only through
//! [`Environment`], so tests are hermetic without mutating the process
//! environment and racing each other.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::ids::PluginId;

/// The reserved plugin id whose config is the root `config.yaml`.
pub const CORE_PLUGIN_ID: &str = "core";

/// Default namespace directory under the XDG config root.
pub const DEFAULT_CONFIG_NAMESPACE: &str = "ix";

/// A read-only view of the ambient environment.
///
/// A trait rather than direct `std::env` reads so every test can supply a
/// hermetic environment; `std::env::set_var` is process-global and racy under
/// `cargo test`'s thread-per-test model.
pub trait Environment: Send + Sync {
    /// The value of an environment variable, if set (an empty value counts as
    /// set, matching Node's `process.env`).
    fn var(&self, name: &str) -> Option<String>;

    /// The user's home directory.
    fn home_dir(&self) -> Option<PathBuf>;
}

/// Reads the real process environment.
#[derive(Debug, Clone, Copy, Default)]
pub struct ProcessEnvironment;

impl Environment for ProcessEnvironment {
    fn var(&self, name: &str) -> Option<String> {
        std::env::var(name).ok()
    }

    fn home_dir(&self) -> Option<PathBuf> {
        // `HOME` is what Node's `os.homedir()` resolves to on the platforms
        // quoin ships for; keeping it explicit avoids pulling in a crate whose
        // fallbacks would diverge from the oracle.
        self.var("HOME")
            .filter(|h| !h.is_empty())
            .map(PathBuf::from)
    }
}

/// A fixed environment, for tests and for callers that already resolved theirs.
#[derive(Debug, Clone, Default)]
pub struct FixedEnvironment {
    vars: BTreeMap<String, String>,
    home: Option<PathBuf>,
}

impl FixedEnvironment {
    /// An environment with no variables and no home directory.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Set a variable.
    #[must_use]
    pub fn with_var(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.vars.insert(name.into(), value.into());
        self
    }

    /// Set the home directory.
    #[must_use]
    pub fn with_home(mut self, home: impl Into<PathBuf>) -> Self {
        self.home = Some(home.into());
        self
    }
}

impl Environment for FixedEnvironment {
    fn var(&self, name: &str) -> Option<String> {
        self.vars.get(name).cloned()
    }

    fn home_dir(&self) -> Option<PathBuf> {
        self.home.clone()
    }
}

/// The per-invocation context `BaseCommand.init` publishes in the TypeScript
/// host: an explicit `--config-root`, and the project-local `.ix` layer.
#[derive(Debug, Clone, Default)]
pub struct RuntimeContext {
    /// Overrides the XDG-derived config root (`--config-root` / `IX_CONFIG_ROOT`).
    pub config_root: Option<PathBuf>,
    /// The project-local `.ix` directory, normally `<cwd>/.ix`.
    pub project_config_root: Option<PathBuf>,
    /// Whether the project layer applies at all (`--no-project-config` clears it).
    pub project_config_enabled: bool,
}

impl RuntimeContext {
    /// The default context: no root override, no project layer, project layer
    /// *enabled* should one be supplied.
    #[must_use]
    pub fn new() -> Self {
        Self {
            config_root: None,
            project_config_root: None,
            project_config_enabled: true,
        }
    }

    /// The context `BaseCommand.init` would publish for `cwd`.
    #[must_use]
    pub fn for_cwd(cwd: &Path, no_project_config: bool) -> Self {
        Self {
            config_root: None,
            project_config_root: if no_project_config {
                None
            } else {
                Some(cwd.join(".ix"))
            },
            project_config_enabled: !no_project_config,
        }
    }

    /// Set the config-root override.
    #[must_use]
    pub fn with_config_root(mut self, root: impl Into<PathBuf>) -> Self {
        self.config_root = Some(root.into());
        self
    }

    /// Set the project-local `.ix` directory.
    #[must_use]
    pub fn with_project_config_root(mut self, root: impl Into<PathBuf>) -> Self {
        self.project_config_root = Some(root.into());
        self
    }

    /// Enable or disable the project layer.
    #[must_use]
    pub fn with_project_config_enabled(mut self, enabled: bool) -> Self {
        self.project_config_enabled = enabled;
        self
    }

    /// The project config root, or `None` when the layer is disabled.
    #[must_use]
    pub fn effective_project_config_root(&self) -> Option<&Path> {
        if self.project_config_enabled {
            self.project_config_root.as_deref()
        } else {
            None
        }
    }
}

/// Resolve the config root: the override if any, else `$XDG_CONFIG_HOME/ix`,
/// else `$HOME/.config/ix`, else `./.config/ix` when no home is discoverable.
#[must_use]
pub fn config_root(env: &dyn Environment, ctx: &RuntimeContext) -> PathBuf {
    if let Some(root) = &ctx.config_root {
        return root.clone();
    }
    let base = env
        .var("XDG_CONFIG_HOME")
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
        .or_else(|| env.home_dir().map(|h| h.join(".config")))
        .unwrap_or_else(|| PathBuf::from(".config"));
    base.join(DEFAULT_CONFIG_NAMESPACE)
}

/// Resolve the cache root: `<configRoot>/cache` when the config root is
/// overridden, else `$XDG_CACHE_HOME/ix`, else `$HOME/.cache/ix`.
#[must_use]
pub fn cache_root(env: &dyn Environment, ctx: &RuntimeContext) -> PathBuf {
    if let Some(root) = &ctx.config_root {
        return root.join("cache");
    }
    let base = env
        .var("XDG_CACHE_HOME")
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
        .or_else(|| env.home_dir().map(|h| h.join(".cache")))
        .unwrap_or_else(|| PathBuf::from(".cache"));
    base.join(DEFAULT_CONFIG_NAMESPACE)
}

/// The config file for `id` under an arbitrary root.
///
/// The reserved `core` id maps to `<root>/config.yaml`; every other id maps to
/// `<root>/config.d/<id>.yaml`. Used for both the user root and the project
/// `.ix` root — they share one layout, which is why this takes the root.
#[must_use]
pub fn config_path_for_root(root: &Path, id: &PluginId) -> PathBuf {
    if id.as_str() == CORE_PLUGIN_ID {
        root.join("config.yaml")
    } else {
        root.join("config.d").join(format!("{id}.yaml"))
    }
}

/// The user-level config file for `id`.
#[must_use]
pub fn config_path_for(env: &dyn Environment, ctx: &RuntimeContext, id: &PluginId) -> PathBuf {
    config_path_for_root(&config_root(env, ctx), id)
}

/// The project-level config file for `id`, when a project layer applies.
#[must_use]
pub fn project_config_path_for(ctx: &RuntimeContext, id: &PluginId) -> Option<PathBuf> {
    ctx.effective_project_config_root()
        .map(|root| config_path_for_root(root, id))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(s: &str) -> PluginId {
        PluginId::new(s).expect("test id is legal")
    }

    /// Trace: FR-027-AC-1
    #[test]
    fn tc_381_010_user_config_path_is_xdg_config_d_yaml() {
        let env = FixedEnvironment::new().with_var("XDG_CONFIG_HOME", "/x");
        let ctx = RuntimeContext::new();
        assert_eq!(
            config_path_for(&env, &ctx, &id("quoin")),
            PathBuf::from("/x/ix/config.d/quoin.yaml")
        );
        assert_eq!(
            config_path_for(&env, &ctx, &id("core")),
            PathBuf::from("/x/ix/config.yaml")
        );
    }

    /// Trace: FR-027-AC-1
    #[test]
    fn tc_381_011_home_fallback_and_root_override() {
        let env = FixedEnvironment::new().with_home("/h");
        assert_eq!(
            config_path_for(&env, &RuntimeContext::new(), &id("quoin")),
            PathBuf::from("/h/.config/ix/config.d/quoin.yaml")
        );
        let ctx = RuntimeContext::new().with_config_root("/r");
        assert_eq!(
            config_path_for(&env, &ctx, &id("quoin")),
            PathBuf::from("/r/config.d/quoin.yaml")
        );
        assert_eq!(cache_root(&env, &ctx), PathBuf::from("/r/cache"));
    }

    /// Trace: FR-027-AC-9
    #[test]
    fn tc_381_012_project_layer_path_and_disable() {
        let ctx = RuntimeContext::for_cwd(Path::new("/repo"), false);
        assert_eq!(
            project_config_path_for(&ctx, &id("quoin")),
            Some(PathBuf::from("/repo/.ix/config.d/quoin.yaml"))
        );
        let off = RuntimeContext::for_cwd(Path::new("/repo"), true);
        assert_eq!(project_config_path_for(&off, &id("quoin")), None);
        let disabled = ctx.clone().with_project_config_enabled(false);
        assert_eq!(project_config_path_for(&disabled, &id("quoin")), None);
    }
}
