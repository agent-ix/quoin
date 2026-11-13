// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! `quoin profile evaluate` (FR-180, FR-181, FR-182, NFR-037).
//!
//! Reads the project's source-profile selection, resolves the named profile
//! from installed module data, and evaluates each clause set through the Quire
//! engine with the effective context. Writes to stdout and `--out-dir` only.

mod evaluate;
mod model;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use clap::{Arg, ArgMatches, Command};
use ix_cli_kit::exit::Outcome;
use ix_trace_rs::implements;
use quoin_core::protocol::{Diagnostic, Response, canonical_json};
use quoin_modules::IxHome;
use quoin_quire::ModuleRoot;
use quoin_quire::clauses::{self, Context};
use quoin_quire_types::ClauseSetKey;

use self::evaluate::{Engine, Evaluation};
use self::model::ModuleRef;
use crate::core_runtime::invoke;
use crate::invocation;

const SELECTION_FILE: &str = "source-profile.yaml";

/// The `profile` command.
pub(crate) fn command() -> Command {
    Command::new("profile")
        .about("Evaluate the source profile a repository selects")
        .subcommand_required(true)
        .subcommand(
            Command::new("evaluate")
                .about("Evaluate every clause set of the selected source profile")
                .arg(
                    Arg::new("out_dir")
                        .long("out-dir")
                        .value_name("DIR")
                        .help("Also write each engine report to its own file here"),
                ),
        )
}

/// Execute a parsed `profile` command.
pub(crate) fn run(matches: &ArgMatches) -> Result<Response, String> {
    match matches.subcommand() {
        Some(("evaluate", arguments)) => {
            let out_dir = arguments.get_one::<String>("out_dir").map(PathBuf::from);
            profile_evaluate(out_dir.as_deref())
        }
        _ => Err("an unknown profile command reached dispatch".to_owned()),
    }
}

fn rendered(text: &str) -> Response {
    Response::ok(serde_json::json!({ "rendered": text }))
}

/// The module roots the catalog locates (FR-006).
fn module_roots() -> Result<Vec<ModuleRef>, String> {
    let response = invoke("catalog.load", &serde_json::json!({}))?;
    if !response.outcome.carries_payload() {
        return Err("the module catalog could not be loaded".to_owned());
    }
    let listed = response
        .payload
        .get("modules")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| "catalog.load did not return modules".to_owned())?;
    listed
        .iter()
        .map(|module| {
            let field = |name: &str| {
                module
                    .get(name)
                    .and_then(serde_json::Value::as_str)
                    .map(str::to_owned)
                    .ok_or_else(|| format!("a catalog module has no {name}"))
            };
            Ok(ModuleRef {
                name: field("name")?,
                root: PathBuf::from(field("root")?),
            })
        })
        .collect()
}

/// Why a run ended without a report.
enum Stop {
    /// A coded refusal (exit 2).
    Refuse(Box<Response>),
    /// An operational failure (exit 1).
    Fail(String),
}

impl From<String> for Stop {
    fn from(message: String) -> Self {
        Self::Fail(message)
    }
}

/// A refusal: no payload, one diagnostic carrying the stable code.
fn refusal(code: &str, message: String) -> Stop {
    Stop::Refuse(Box::new(Response {
        payload: serde_json::Value::Null,
        diagnostics: vec![Diagnostic {
            code: code.to_owned(),
            message,
            context: BTreeMap::new(),
        }],
        outcome: Outcome::Refused,
    }))
}

impl From<model::ProfileError> for Stop {
    fn from(error: model::ProfileError) -> Self {
        refusal(error.code(), error.to_string())
    }
}

impl From<evaluate::EvaluateError> for Stop {
    fn from(error: evaluate::EvaluateError) -> Self {
        refusal(error.code(), error.to_string())
    }
}

/// The user config root: `--config-root`, else `IX_HOME`, else `~/.ix`.
fn user_root() -> PathBuf {
    invocation::current()
        .config_root()
        .cloned()
        .unwrap_or_else(|| {
            IxHome::resolve(
                std::env::var("IX_HOME").ok().as_deref(),
                std::env::home_dir().as_deref(),
            )
            .as_path()
            .to_path_buf()
        })
}

fn profile_evaluate(out_dir: Option<&Path>) -> Result<Response, String> {
    match evaluate_selected(out_dir) {
        Ok(response) => Ok(response),
        Err(Stop::Refuse(response)) => Ok(*response),
        Err(Stop::Fail(message)) => Err(message),
    }
}

#[implements("FR-180", "FR-181", "FR-182")]
fn evaluate_selected(out_dir: Option<&Path>) -> Result<Response, Stop> {
    let invocation = invocation::current();
    let selection_file = if invocation.no_project_config() {
        None
    } else {
        let cwd = std::env::current_dir().map_err(|e| format!("cannot read cwd: {e}"))?;
        Some(cwd.join(".ix").join(SELECTION_FILE))
    };
    let selection = match &selection_file {
        Some(file) => match model::read_bounded(file)? {
            Some(text) => Some(model::parse_selection(file, &text)?),
            None => None,
        },
        None => None,
    };
    let Some(selection) = selection else {
        return Ok(rendered(
            "no source profile is selected; no clause set was evaluated",
        ));
    };

    let user_file = user_root().join(SELECTION_FILE);
    let user_add = match model::read_bounded(&user_file)? {
        Some(text) => model::parse_user_add(&user_file, &text)?,
        None => Vec::new(),
    };

    let modules = module_roots()?;
    let profile = model::load_profile(&selection.profile, &modules)?;
    let effective = model::effective(&profile, &selection, &user_add);
    let evaluation = evaluate::evaluate(&effective, &modules, &QuireEngine)?;
    if let Some(dir) = out_dir {
        write_reports(dir, &evaluation)?;
    }
    Ok(rendered(
        &canonical_json(&evaluation).map_err(|e| e.to_string())?,
    ))
}

/// One file per evaluated set. A name is a position prefix and the set id with
/// every character outside `[A-Za-z0-9.-]` replaced, so no key text can form a
/// path separator.
fn write_reports(dir: &Path, evaluation: &Evaluation) -> Result<(), String> {
    std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    for (index, entry) in evaluation.sets.iter().enumerate() {
        let Some(report) = &entry.report else {
            continue;
        };
        let safe: String = entry
            .key
            .id
            .chars()
            .map(|c| {
                if c.is_ascii_alphanumeric() || c == '-' || c == '.' {
                    c
                } else {
                    '_'
                }
            })
            .collect();
        let file = dir.join(format!("{:02}-{safe}.json", index + 1));
        let body = canonical_json(report).map_err(|e| e.to_string())?;
        std::fs::write(&file, body).map_err(|e| format!("{}: {e}", file.display()))?;
    }
    Ok(())
}

/// The production engine, over `quoin-quire`.
struct QuireEngine;

fn open(module: &ModuleRef) -> Result<ModuleRoot, String> {
    ModuleRoot::open(&module.root).map_err(|e| e.to_string())
}

impl Engine for QuireEngine {
    fn declared(&self, module: &ModuleRef) -> Result<Vec<ClauseSetKey>, String> {
        let sets = clauses::declared_clause_sets(&open(module)?, &mut Vec::new())
            .map_err(|e| e.to_string())?;
        Ok(sets
            .into_iter()
            .map(|set| ClauseSetKey {
                authority: set.authority.to_string(),
                id: set.id.to_string(),
                version: set.version.to_string(),
            })
            .collect())
    }

    fn is_pack(&self, module: &ModuleRef) -> Result<bool, String> {
        let restricted =
            clauses::restricted_clause_sets(&open(module)?).map_err(|e| e.to_string())?;
        Ok(!restricted.is_empty())
    }

    fn evaluate(
        &self,
        module: &ModuleRef,
        key: &ClauseSetKey,
        context: &std::collections::BTreeMap<String, String>,
    ) -> Result<serde_json::Value, String> {
        let reference = quoin_quire::ClauseSetRef {
            authority: quoin_quire::ClauseSetAuthority::new(key.authority.as_str())
                .map_err(|e| e.to_string())?,
            id: quoin_quire::ClauseSetId::new(key.id.as_str()).map_err(|e| e.to_string())?,
            version: quoin_quire::ClauseSetVersion::new(key.version.as_str())
                .map_err(|e| e.to_string())?,
        };
        let mut engine_context = Context::new();
        for (dimension, value) in context {
            engine_context
                .insert(dimension.as_str(), value.as_str())
                .map_err(|e| e.to_string())?;
        }
        let report =
            clauses::evaluate(&open(module)?, &reference, &engine_context, &mut Vec::new())
                .map_err(|e| e.to_string())?;
        serde_json::to_value(&report).map_err(|e| e.to_string())
    }
}
