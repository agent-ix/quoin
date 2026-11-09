// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! Strict machine-selected bindings for a generic EA campaign run.

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use engineering_assurance::campaign::ProcedureBindings;
use engineering_assurance::producer_execution::{
    CancellationBinding, ContainmentBinding, ContractBinding, ExecutionBudget, ExitCodeBinding,
    OutputBinding, OutputTreeBinding, ProducerDescriptor, StdinBinding,
};
use quoin_measurement::campaign::run::{InputSource, RunMemberBindings, SelectedInput};
use serde::Deserialize;

/// One closed machine configuration; the authored definition is read separately.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct RunSelection {
    pub schema: String,
    pub sources: BTreeMap<String, PathBuf>,
    members: BTreeMap<String, MemberBindingWire>,
}

type SelectedBindings = (
    BTreeMap<String, PathBuf>,
    BTreeMap<String, RunMemberBindings>,
);

impl RunSelection {
    pub(super) fn into_parts(self) -> SelectedBindings {
        let bindings = self
            .members
            .into_iter()
            .map(|(name, member)| (name, member.into_binding()))
            .collect::<BTreeMap<_, _>>();
        (self.sources, bindings)
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct MemberBindingWire {
    producer: ProcedureBindingWire,
    checker: Option<ProcedureBindingWire>,
    #[serde(default)]
    inputs: Vec<SelectedInputWire>,
    timestamp: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ProcedureBindingWire {
    producer: ProducerWire,
    caller: ContractWire,
    #[serde(default)]
    environment: BTreeMap<String, String>,
    #[serde(default)]
    outputs: Vec<OutputWire>,
    #[serde(default)]
    output_trees: Vec<OutputTreeWire>,
    stdin: StdinWire,
    containment: ContainmentWire,
    cancellation: CancellationWire,
    budget: BudgetWire,
    response_protocol: ContractWire,
    response_adapter: ContractWire,
    exit_codes: ExitCodeWire,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ProducerWire {
    name: String,
    version: String,
    executable: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ContractWire {
    kind: String,
    version: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct OutputWire {
    role: String,
    path: String,
    required: bool,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct OutputTreeWire {
    role: String,
    path: String,
    required: bool,
}

#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
enum StdinWire {
    Null,
    InputArtifact { role: String },
}

#[derive(Deserialize)]
#[serde(tag = "profile", rename_all = "kebab-case", deny_unknown_fields)]
enum ContainmentWire {
    ProcessGroupV1 { contract: ContractWire },
}

#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
enum CancellationWire {
    Disabled,
    Event { authority: String, event_id: String },
}

#[derive(Deserialize)]
#[serde(
    tag = "kind",
    content = "codes",
    rename_all = "snake_case",
    deny_unknown_fields
)]
enum ExitCodeWire {
    Any,
    Exact(BTreeSet<i32>),
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct BudgetWire {
    timeout_millis: u64,
    max_stdout_bytes: usize,
    max_stderr_bytes: usize,
    max_input_bytes: u64,
    max_output_artifacts: usize,
    max_output_bytes: u64,
    max_descendants: usize,
    max_concurrency: usize,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct SelectedInputWire {
    role: String,
    path: String,
    #[serde(default)]
    executable: bool,
    source: InputSourceWire,
}

#[derive(Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "snake_case",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
enum InputSourceWire {
    File {
        path: PathBuf,
    },
    SourceFile {
        repository: String,
        path: String,
    },
    Dependency {
        member: String,
        index: i64,
        artifact_role: String,
    },
}

impl ContractWire {
    fn into_binding(self) -> ContractBinding {
        ContractBinding {
            kind: self.kind,
            version: self.version,
        }
    }
}

impl ProducerWire {
    fn into_descriptor(self) -> ProducerDescriptor {
        ProducerDescriptor {
            name: self.name,
            version: self.version,
            executable: self.executable,
        }
    }
}

impl ProcedureBindingWire {
    fn into_binding(self) -> ProcedureBindings {
        let containment = match self.containment {
            ContainmentWire::ProcessGroupV1 { contract } => ContainmentBinding::ProcessGroupV1 {
                contract: contract.into_binding(),
            },
        };
        let cancellation = match self.cancellation {
            CancellationWire::Disabled => CancellationBinding::Disabled,
            CancellationWire::Event {
                authority,
                event_id,
            } => CancellationBinding::Event {
                authority,
                event_id,
            },
        };
        let stdin = match self.stdin {
            StdinWire::Null => StdinBinding::Null,
            StdinWire::InputArtifact { role } => StdinBinding::InputArtifact { role },
        };
        let exit_codes = match self.exit_codes {
            ExitCodeWire::Any => ExitCodeBinding::Any,
            ExitCodeWire::Exact(codes) => ExitCodeBinding::Exact(codes),
        };
        ProcedureBindings {
            producer: self.producer.into_descriptor(),
            caller: self.caller.into_binding(),
            capability_root: String::new(),
            environment: self.environment,
            inputs: Vec::new(),
            input_origins: None,
            source_tree: None,
            outputs: self
                .outputs
                .into_iter()
                .map(|output| OutputBinding {
                    role: output.role,
                    path: output.path,
                    required: output.required,
                })
                .collect(),
            output_trees: self
                .output_trees
                .into_iter()
                .map(|tree| OutputTreeBinding {
                    role: tree.role,
                    path: tree.path,
                    required: tree.required,
                })
                .collect(),
            stdin,
            containment,
            cancellation,
            budget: ExecutionBudget {
                timeout_millis: self.budget.timeout_millis,
                max_stdout_bytes: self.budget.max_stdout_bytes,
                max_stderr_bytes: self.budget.max_stderr_bytes,
                max_input_bytes: self.budget.max_input_bytes,
                max_output_artifacts: self.budget.max_output_artifacts,
                max_output_bytes: self.budget.max_output_bytes,
                max_descendants: self.budget.max_descendants,
                max_concurrency: self.budget.max_concurrency,
            },
            response_protocol: self.response_protocol.into_binding(),
            response_adapter: self.response_adapter.into_binding(),
            exit_codes,
        }
    }
}

impl MemberBindingWire {
    fn into_binding(self) -> RunMemberBindings {
        let inputs = self
            .inputs
            .into_iter()
            .map(|selected| {
                let source = match selected.source {
                    InputSourceWire::File { path } => InputSource::File(path),
                    InputSourceWire::SourceFile { repository, path } => {
                        InputSource::SourceFile { repository, path }
                    }
                    InputSourceWire::Dependency {
                        member,
                        index,
                        artifact_role,
                    } => InputSource::Dependency {
                        member,
                        index,
                        artifact_role,
                    },
                };
                SelectedInput {
                    role: selected.role,
                    path: selected.path,
                    executable: selected.executable,
                    source,
                }
            })
            .collect();
        RunMemberBindings {
            producer: self.producer.into_binding(),
            checker: self.checker.map(ProcedureBindingWire::into_binding),
            inputs,
            timestamp: self.timestamp,
        }
    }
}

#[cfg(test)]
#[allow(
    clippy::expect_used,
    clippy::indexing_slicing,
    reason = "fixture shape and identity assertions must fail loudly when setup changes"
)]
mod tests {
    use super::{InputSource, RunSelection};
    use serde_json::{Value, json};

    fn contract() -> Value {
        json!({
            "kind": "fixture.contract",
            "version": "1",
        })
    }

    fn binding() -> Value {
        json!({
            "producer": {
                "name": "fixture-producer",
                "version": "1",
                "executable": "/usr/bin/true",
            },
            "caller": contract(),
            "environment": {},
            "outputs": [],
            "outputTrees": [],
            "stdin": {"kind": "null"},
            "containment": {"profile": "process-group-v1", "contract": contract()},
            "cancellation": {"kind": "disabled"},
            "budget": {
                "timeoutMillis": 1000,
                "maxStdoutBytes": 1024,
                "maxStderrBytes": 1024,
                "maxInputBytes": 4096,
                "maxOutputArtifacts": 1,
                "maxOutputBytes": 4096,
                "maxDescendants": 1,
                "maxConcurrency": 1
            },
            "responseProtocol": contract(),
            "responseAdapter": contract(),
            "exitCodes": {"kind": "any"}
        })
    }

    fn config() -> Value {
        json!({
            "schema": "quoin.campaign-run-config/v1",
            "sources": {"fixture": "/tmp/fixture-checkout"},
            "members": {
                "fixture.member": {
                    "producer": binding(),
                    "checker": null,
                    "inputs": [{
                        "role": "source-case",
                        "path": "case.txt",
                        "source": {
                            "kind": "source_file",
                            "repository": "fixture",
                            "path": "case.txt"
                        }
                    }],
                    "timestamp": "2026-01-01T00:00:00Z"
                }
            }
        })
    }

    /// Trace: FR-114-AC-1, TC-1942
    #[test]
    fn tc_1942_run_selection_builds_exact_typed_bindings() {
        let selected: RunSelection = serde_json::from_value(config()).expect("typed config");
        let (sources, members) = selected.into_parts();
        assert_eq!(sources.len(), 1);
        let member = members.get("fixture.member").expect("named member");
        assert_eq!(member.producer.producer.executable, "/usr/bin/true");
        assert!(member.checker.is_none());
        assert!(matches!(
            &member.inputs.first().expect("one selected input").source,
            InputSource::SourceFile { repository, path }
                if repository == "fixture" && path == "case.txt"
        ));
    }

    /// Trace: FR-114-AC-4, TC-1942
    #[test]
    fn tc_1942_run_selection_refuses_unknown_fields() {
        let mut unknown = config();
        unknown["members"]["fixture.member"]["producer"]["unbound"] = json!(true);
        assert!(serde_json::from_value::<RunSelection>(unknown).is_err());
    }
}
