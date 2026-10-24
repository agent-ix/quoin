// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Install and launch the published specification workflows (FR-020, FR-021).
//! Runtime assets belong to the effective configuration root; a native release
//! must not resolve through its build machine's source checkout.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command as ProcessCommand;

use clap::{Arg, ArgAction, ArgMatches, Command};
use quoin_core::protocol::Response;

use crate::invocation;

/// `(quoin subcommand, @agent-ix/ix-spec-workflows `spec/` directory name)`.
const FLOWS: [(&str, &str); 2] = [("review", "review"), ("to-plan", "to-plan")];

pub(crate) fn run(name: &str, arguments: &ArgMatches) -> Result<Response, String> {
    let (_, runtime_skill) = FLOWS
        .iter()
        .find(|(candidate, _)| *candidate == name)
        .ok_or_else(|| "an unknown flow command reached dispatch".to_owned())?;
    let path = skill_path(runtime_skill)?;
    let home = ix_home();
    let mut command = ProcessCommand::new("ix-flow");
    command
        .arg("run")
        .arg(name)
        .arg("--path")
        .arg(path)
        .arg("--config-root")
        .arg(&home)
        .arg("--state-dir")
        .arg(home.join("flows"));
    if let Some(id) = arguments.get_one::<String>("id") {
        command.arg("--id").arg(id);
    }
    if arguments.get_flag("json") {
        command.arg("--json");
    }
    if let Some(targets) = arguments.get_many::<String>("target") {
        for target in targets {
            command.arg("--target").arg(target);
        }
    }
    let output = command
        .output()
        .map_err(|error| format!("could not start ix-flow: {error}"))?;
    if !output.status.success() {
        let detail = String::from_utf8_lossy(&output.stderr).trim().to_owned();
        return Err(if detail.is_empty() {
            format!("ix-flow {name} failed")
        } else {
            detail
        });
    }
    let rendered = String::from_utf8(output.stdout)
        .map_err(|error| format!("ix-flow wrote non-UTF-8 output: {error}"))?;
    Ok(Response::ok(
        serde_json::json!({ "rendered": rendered.trim_end() }),
    ))
}

/// The shared grammar for one bundled workflow launcher.
pub(crate) fn command(name: &'static str) -> Command {
    Command::new(name)
        .about("Start the bundled specification workflow")
        .arg(Arg::new("target").long("target").action(ArgAction::Append))
        .arg(Arg::new("json").long("json").action(ArgAction::SetTrue))
        .arg(Arg::new("id").long("id"))
}

fn skill_path(runtime_skill: &str) -> Result<PathBuf, String> {
    let home = ix_home();
    let cwd = std::env::current_dir().map_err(|error| error.to_string())?;
    let explicit = std::env::var_os("IX_SPEC_WORKFLOWS_ROOT").map(PathBuf::from);
    resolve_skill(runtime_skill, explicit.as_deref(), &home, &cwd)
}

fn resolve_skill(
    skill: &str,
    explicit: Option<&Path>,
    home: &Path,
    cwd: &Path,
) -> Result<PathBuf, String> {
    if let Some(root) = explicit {
        return package_skill(root, skill).ok_or_else(|| format!(
            "IX_SPEC_WORKFLOWS_ROOT={} does not contain a complete {skill} workflow; use the package root, spec/, or skills/ directory", root.display()
        )).and_then(|path| check_skill(path, skill));
    }
    for root in [
        home.join("plugins/ix-spec-workflows"),
        home.join("workflow-runtime/node_modules/@agent-ix/ix-spec-workflows"),
        cwd.join("node_modules/@agent-ix/ix-spec-workflows"),
    ] {
        if let Some(path) = package_skill(&root, skill) {
            return check_skill(path, skill);
        }
    }
    Err(format!(
        "could not find the {skill} workflow; run `quoin workflows install` or `quoin workflows install --source <built-ix-spec-workflows-package>`; assets are installed under {}",
        home.display()
    ))
}

fn package_skill(root: &Path, skill: &str) -> Option<PathBuf> {
    [
        root.join(skill),
        root.join("skills").join(skill),
        root.join("spec").join(skill),
    ]
    .into_iter()
    .find(|path| {
        path.join("SKILL.md").is_file()
            && path
                .join("workflows")
                .join(skill)
                .join("def.yaml")
                .is_file()
            && path.join("scripts/invariants.js").is_file()
            && path
                .parent()
                .and_then(Path::parent)
                .is_some_and(|package| package.join("dist/index.js").is_file())
    })
}

fn check_skill(path: PathBuf, skill: &str) -> Result<PathBuf, String> {
    if skill == "review" {
        let definition = fs::read_to_string(path.join("workflows/review/def.yaml"))
            .map_err(|error| error.to_string())?;
        let definition = quoin_yaml::from_str(&definition).map_err(|error| error.to_string())?;
        let enforced = definition
            .get("transitions")
            .and_then(serde_json::Value::as_array)
            .is_some_and(|transitions| {
                transitions.iter().any(|transition| {
                    transition.get("to").and_then(serde_json::Value::as_str) == Some("accepted")
                        && transition
                            .get("invariants")
                            .and_then(serde_json::Value::as_array)
                            .is_some_and(|invariants| {
                                invariants.iter().any(|value| {
                                    value.as_str() == Some("selected_analyses_covered")
                                })
                            })
                })
            });
        if !enforced {
            return Err(format!(
                "review workflow at {} lacks selected_analyses_covered; install a workflow package with enforced per-analysis validation; direct reviews do not complete this run",
                path.display()
            ));
        }
    }
    Ok(path)
}

pub(crate) fn workflows_command() -> Command {
    Command::new("workflows")
        .about("Install specification workflow runtime assets")
        .subcommand_required(true)
        .subcommand(
            Command::new("install")
                .arg(Arg::new("json").long("json").action(ArgAction::SetTrue))
                .arg(
                    Arg::new("source").long("source").help(
                        "Built workflow package directory to install without registry access",
                    ),
                )
                .arg(
                    Arg::new("registry")
                        .long("registry")
                        .help("Registry for the @agent-ix scope"),
                ),
        )
}

pub(crate) fn install(arguments: &ArgMatches) -> Result<Response, String> {
    let (_, arguments) = arguments.subcommand().ok_or("expected workflows install")?;
    let home = ix_home();
    let source = if let Some(source) = arguments.get_one::<String>("source") {
        PathBuf::from(source)
            .canonicalize()
            .map_err(|error| error.to_string())?
    } else {
        let prefix = home.join("workflow-runtime");
        let mut npm = ProcessCommand::new(if cfg!(windows) { "npm.cmd" } else { "npm" });
        npm.arg("install").arg("--prefix").arg(&prefix).args([
            "--ignore-scripts",
            "--no-audit",
            "--no-fund",
            "@agent-ix/ix-spec-workflows@latest",
        ]);
        if let Some(registry) = arguments.get_one::<String>("registry") {
            npm.arg(format!("--@agent-ix:registry={registry}"));
        }
        let output = npm
            .output()
            .map_err(|error| format!("could not start npm: {error}"))?;
        if !output.status.success() {
            return Err("workflow package installation failed; check scoped registry authentication, or install a built package with --source".to_owned());
        }
        prefix.join("node_modules/@agent-ix/ix-spec-workflows")
    };
    if !source.join("package.json").is_file() || !source.join("dist/index.js").is_file() {
        return Err(
            "--source must name the built package root containing package.json and dist/index.js"
                .to_owned(),
        );
    }
    let manifest: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(source.join("package.json")).map_err(|error| error.to_string())?,
    )
    .map_err(|error| error.to_string())?;
    if manifest.get("name").and_then(serde_json::Value::as_str)
        != Some("@agent-ix/ix-spec-workflows")
    {
        return Err("source is not an @agent-ix/ix-spec-workflows package".to_owned());
    }
    let version = manifest
        .get("version")
        .and_then(serde_json::Value::as_str)
        .ok_or("workflow package has no version")?;
    semver::Version::parse(version)
        .map_err(|error| format!("invalid workflow package version: {error}"))?;
    let review = package_skill(&source, "review")
        .ok_or("source package is incomplete; build ix-spec-workflows first")?;
    check_skill(review, "review")?;
    package_skill(&source, "to-plan")
        .ok_or("source package lacks the complete to-plan workflow")?;
    let parent = home.join("plugins");
    fs::create_dir_all(&parent).map_err(|error| error.to_string())?;
    let stage = parent.join(format!(".ix-spec-workflows-install-{}", std::process::id()));
    fs::create_dir(&stage).map_err(|error| error.to_string())?;
    let result = install_local_package(&source, &stage, &parent);
    if stage.exists() {
        let _ = fs::remove_dir_all(&stage);
    }
    let root = result?;
    let rendered = if arguments.get_flag("json") {
        serde_json::to_string_pretty(
            &serde_json::json!({"installed": root, "version": version, "workflows": ["review", "to-plan"]}),
        )
        .map_err(|error| error.to_string())?
    } else {
        format!("installed workflow package {version} at {}", root.display())
    };
    Ok(Response::ok(serde_json::json!({"rendered": rendered})))
}

fn install_local_package(source: &Path, stage: &Path, parent: &Path) -> Result<PathBuf, String> {
    if stage.starts_with(source) {
        return Err(
            "the source package cannot contain the installation staging directory".to_owned(),
        );
    }
    for name in ["package.json", "dist", "skills", "spec"] {
        let path = source.join(name);
        if path.exists() {
            copy_assets(&path, &stage.join(name))?;
        }
    }
    let target = parent.join("ix-spec-workflows");
    let backup = parent.join(format!(".ix-spec-workflows-backup-{}", std::process::id()));
    if target.exists() {
        fs::rename(&target, &backup).map_err(|error| error.to_string())?;
    }
    if let Err(error) = fs::rename(stage, &target) {
        if backup.exists() {
            let _ = fs::rename(&backup, &target);
        }
        return Err(error.to_string());
    }
    if backup.exists() {
        fs::remove_dir_all(backup).map_err(|error| error.to_string())?;
    }
    Ok(target)
}

fn copy_assets(source: &Path, target: &Path) -> Result<(), String> {
    let metadata = fs::symlink_metadata(source).map_err(|error| error.to_string())?;
    if metadata.is_dir() {
        fs::create_dir_all(target).map_err(|error| error.to_string())?;
        for entry in fs::read_dir(source).map_err(|error| error.to_string())? {
            let entry = entry.map_err(|error| error.to_string())?;
            copy_assets(&entry.path(), &target.join(entry.file_name()))?;
        }
    } else if metadata.is_file() {
        fs::copy(source, target).map_err(|error| error.to_string())?;
    } else {
        return Err(format!(
            "workflow assets must be regular files: {}",
            source.display()
        ));
    }
    Ok(())
}

fn ix_home() -> PathBuf {
    if let Some(root) = invocation::current().config_root() {
        return root.clone();
    }
    std::env::var_os("IX_HOME")
        .filter(|home| !home.is_empty())
        .map_or_else(
            || {
                std::env::var_os("HOME").map_or_else(
                    || PathBuf::from(".ix"),
                    |home| PathBuf::from(home).join(".ix"),
                )
            },
            PathBuf::from,
        )
}

#[cfg(test)]
#[allow(clippy::expect_used, reason = "test fixtures may panic")]
mod tests {
    use super::{FLOWS, command, install_local_package, resolve_skill, run};
    use std::fs;
    use std::path::Path;

    /// `matrix` is a computed view now, not a workflow: the flow table no
    /// longer names it, and the launcher refuses it before any process starts.
    ///
    /// Trace: FR-115-AC-13, FR-020-AC-1, TC-1976, TC-065
    #[test]
    fn tc_1080_300_matrix_is_not_a_flow() {
        let names: Vec<&str> = FLOWS.iter().map(|(name, _)| *name).collect();
        assert_eq!(names, ["review", "to-plan"]);
        let matches = command("matrix")
            .try_get_matches_from(["matrix"])
            .expect("grammar parses");
        assert_eq!(
            run("matrix", &matches).err().as_deref(),
            Some("an unknown flow command reached dispatch")
        );
    }

    /// Trace: FR-062
    #[test]
    fn tc_373_742_flow_launchers_keep_the_shared_grammar() {
        let matches = command("review")
            .try_get_matches_from([
                "review", "--target", "spec", "--target", "other", "--id", "run-1", "--json",
            ])
            .expect("grammar parses");
        assert_eq!(
            matches
                .get_many::<String>("target")
                .expect("targets")
                .map(String::as_str)
                .collect::<Vec<_>>(),
            ["spec", "other"]
        );
        assert_eq!(
            matches.get_one::<String>("id").map(String::as_str),
            Some("run-1")
        );
        assert!(matches.get_flag("json"));
    }
    fn package(root: &Path, layout: &str, enforced: bool) {
        let skill = root.join(layout).join("review");
        fs::create_dir_all(skill.join("workflows/review")).expect("workflow directory");
        fs::create_dir_all(skill.join("scripts")).expect("scripts directory");
        fs::create_dir_all(root.join("dist")).expect("bundle directory");
        fs::write(
            root.join("dist/index.js"),
            "export const specInvariants = {};",
        )
        .expect("bundle");
        fs::write(skill.join("SKILL.md"), "review").expect("skill");
        fs::write(
            skill.join("scripts/invariants.js"),
            "export const invariants = {};",
        )
        .expect("shim");
        let gate = if enforced {
            "[selected_analyses_covered]"
        } else {
            "[]"
        };
        fs::write(
            skill.join("workflows/review/def.yaml"),
            format!("transitions:\n  - to: accepted\n    invariants: {gate}\n"),
        )
        .expect("definition");
    }

    /// Trace: FR-020-AC-4, FR-020-AC-5
    #[test]
    fn tc_workflow_assets_resolve_from_config_root_without_project_dependencies() {
        let home = tempfile::tempdir().expect("config root");
        let cwd = tempfile::tempdir().expect("Rust project");
        let root = home.path().join("plugins/ix-spec-workflows");
        package(&root, "skills", true);
        let resolved =
            resolve_skill("review", None, home.path(), cwd.path()).expect("installed workflow");
        assert_eq!(resolved, root.join("skills/review"));
        for explicit in [&root, &root.join("skills")] {
            assert_eq!(
                resolve_skill("review", Some(explicit), home.path(), cwd.path())
                    .expect("explicit root"),
                resolved
            );
        }
    }

    /// Trace: FR-020-AC-5, FR-020-AC-6
    #[test]
    fn tc_legacy_review_and_incomplete_explicit_overrides_are_refused() {
        let home = tempfile::tempdir().expect("config root");
        let cwd = tempfile::tempdir().expect("project");
        package(home.path(), "spec", false);
        let legacy = resolve_skill("review", Some(home.path()), home.path(), cwd.path())
            .expect_err("unenforced workflow");
        assert!(legacy.contains("lacks selected_analyses_covered"));
        fs::remove_file(home.path().join("dist/index.js")).expect("remove missing bundle");
        let incomplete = resolve_skill("review", Some(home.path()), home.path(), cwd.path())
            .expect_err("incomplete workflow");
        assert!(incomplete.contains("does not contain a complete"));
    }

    /// Trace: FR-020-AC-4
    #[test]
    fn tc_local_install_replaces_assets_and_keeps_the_complete_runtime() {
        let source = tempfile::tempdir().expect("source");
        let home = tempfile::tempdir().expect("home");
        package(source.path(), "skills", true);
        let stage = home.path().join("stage");
        fs::create_dir(&stage).expect("stage");
        let installed = install_local_package(source.path(), &stage, home.path()).expect("install");
        assert!(installed.join("dist/index.js").is_file());
        assert!(resolve_skill("review", Some(&installed), home.path(), home.path()).is_ok());
        fs::create_dir(&stage).expect("second stage");
        install_local_package(source.path(), &stage, home.path()).expect("replace");
        assert!(!stage.exists());
    }
}
