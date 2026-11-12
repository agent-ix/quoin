// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! The hermetic world every NFR-003 probe runs in: a home seeded from the
//! retained catalog fixture (its registry has every default module settled, so
//! nothing reaches the network), one probe module whose types are the only
//! ones in scope, a working directory of its own, and the command tree the
//! binary describes.

#![allow(
    clippy::expect_used,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]

use std::path::{Path, PathBuf};
use std::process::Output;
use std::sync::OnceLock;

use super::help_tree::{self, CommandHelp};
use super::quoin;

/// The artifact types in scope: those of the probe module and of the quire
/// fixture module that declares a traceability model, so commands that read
/// one reach their own checks.
pub(crate) const PROBE_TYPES: [&str; 3] = ["NfrZeta", "NfrAlpha", "TestMatrix"];

/// Copy the directory tree `source` to `destination`.
pub(crate) fn copy_tree(source: &Path, destination: &Path) {
    std::fs::create_dir_all(destination).expect("the destination is creatable");
    for entry in std::fs::read_dir(source).expect("the source is readable") {
        let entry = entry.expect("the entry is readable");
        let target = destination.join(entry.file_name());
        if entry.file_type().expect("the type is readable").is_dir() {
            copy_tree(&entry.path(), &target);
        } else {
            std::fs::copy(entry.path(), target).expect("the file copies");
        }
    }
}

/// Seed `home` from the retained catalog fixture.
pub(crate) fn seed_home(home: &Path) {
    copy_tree(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/retained-catalog/ix-home"),
        home,
    );
}

/// A module directory whose manifest declares `types` and nothing else.
pub(crate) fn write_module(directory: &Path, name: &str, types: &[&str]) {
    std::fs::create_dir_all(directory).expect("the module directory is creatable");
    let types = types.join("\n  - name: ");
    std::fs::write(
        directory.join("manifest.yaml"),
        format!("name: {name}\nartifact_types:\n  - name: {types}\n"),
    )
    .expect("the manifest is writable");
}

pub(crate) struct Context {
    scratch: tempfile::TempDir,
    home: PathBuf,
    modules: std::ffi::OsString,
    commands: &'static [CommandHelp],
}

/// The command tree, walked once per test binary: the binary's help does not
/// depend on the home it is asked in.
static COMMANDS: OnceLock<Vec<CommandHelp>> = OnceLock::new();

impl Context {
    pub(crate) fn new() -> Self {
        let scratch = tempfile::tempdir().expect("scratch is creatable");
        let home = scratch.path().join("ix-home");
        seed_home(&home);
        let module = scratch.path().join("probe-module");
        write_module(&module, "nfr003-probe", &PROBE_TYPES);
        std::fs::create_dir_all(scratch.path().join("cwd")).expect("the cwd is creatable");
        let traceability =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../quoin-quire/tests/fixtures/repo");
        let modules = std::env::join_paths([&module, &traceability]).expect("paths join");
        let commands = COMMANDS.get_or_init(|| help_tree::walk(&home));
        Self {
            scratch,
            home,
            modules,
            commands,
        }
    }

    /// The root and every command below it.
    pub(crate) fn commands(&self) -> &'static [CommandHelp] {
        self.commands
    }

    /// A path that does not exist, unique to `label`.
    pub(crate) fn missing_path(&self, label: &str) -> PathBuf {
        self.scratch
            .path()
            .join("does-not-exist")
            .join(label.trim_start_matches('-'))
    }

    /// A file under the scratch directory holding `contents`.
    pub(crate) fn file(&self, name: &str, contents: &str) -> PathBuf {
        let path = self.scratch.path().join(name);
        std::fs::write(&path, contents).expect("the file is writable");
        path
    }

    /// `quoin <command path> <extra>` in the seeded home, with only the probe
    /// modules in scope and no stdin.
    pub(crate) fn run(&self, command: &CommandHelp, extra: &[&str]) -> Output {
        self.run_with(command, extra, |_| {})
    }

    pub(crate) fn run_with(
        &self,
        command: &CommandHelp,
        extra: &[&str],
        configure: impl FnOnce(&mut std::process::Command),
    ) -> Output {
        let arguments: Vec<&str> = command
            .path
            .iter()
            .map(String::as_str)
            .chain(extra.iter().copied())
            .collect();
        quoin(&self.home, &arguments, |process| {
            process
                .env("QUOIN_MODULE_PATHS", &self.modules)
                .current_dir(self.scratch.path().join("cwd"));
            configure(process);
        })
    }
}
