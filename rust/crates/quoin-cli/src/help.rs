// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Native rendering for the retained root help contract.
//!
//! Clap owns argument grammar.  Its default root renderer does not preserve
//! the oclif topic/command catalogue that users see before choosing a command,
//! so that small presentation boundary is explicit here.  The catalogue is
//! deliberately checked against the actual Clap command inventory below: it
//! cannot silently become a second, stale command registry.

#[cfg(test)]
use clap::Command;

const DESCRIPTION: &str = "Spec-driven development for Claude Code — author validated, ISO/IEC/IEEE 29148-aligned specs, then plan and build against them.";

const TOPICS: &[(&str, &str)] = &[
    (
        "assurance",
        "Render the assurance case: claim → argument → evidence, from\n                    the store.",
    ),
    (
        "catalog",
        "Inspect the active artifact/object catalog (list, show,\n                    validate).",
    ),
    (
        "change-assurance",
        "Form, retain, and verify change-assurance evidence from\n                    explicit inputs.",
    ),
    (
        "config",
        "Read and store quoin configuration (get, set, edit, doctor).",
    ),
    (
        "evidence",
        "The evidence store — what actually ran, and against which\n                    statement (record, affirm, gc).",
    ),
    ("graph", "Read-only evidence-graph analysis views."),
    (
        "measurement",
        "Record plan-validated QA measurements as atomic producer\n                    collections.",
    ),
    (
        "module",
        "Install and manage user/community spec modules (list,\n                    install, remove, ensure-defaults).",
    ),
    (
        "semantic",
        "Classify every Markdown artifact's Properties form across\n                    corpus roots (FR-074).",
    ),
];

const COMMANDS: &[(&str, &str)] = &[
    (
        "advise",
        "Recommend a verification method for each obligation, from\n                    the catalog.",
    ),
    (
        "assurance",
        "Render the assurance case: claim → argument → evidence, from\n                    the store.",
    ),
    (
        "catalog",
        "List the active artifact/object catalog modules.",
    ),
    (
        "change-assurance",
        "Form, retain, and verify change-assurance evidence from\n                    explicit inputs.",
    ),
    (
        "completeness",
        "Report which declared vocabulary values no requirement owns,\n                    and judge the excuses.",
    ),
    ("config", "Read a stored quoin config value."),
    (
        "discharge",
        "Partition binding clauses into direct evidence,\n                    dispositions, and open work.",
    ),
    (
        "evidence",
        "The evidence store — what actually ran, and against which\n                    statement.",
    ),
    ("graph", "Read-only evidence-graph analysis views."),
    ("matrix", "Build or update a requirements test matrix."),
    (
        "measurement",
        "Record and inspect versioned QA measurements.",
    ),
    ("module", "List installed spec modules."),
    (
        "report",
        "Render QA plans and measurements from the evidence store.",
    ),
    ("review", "Run a composite spec review workflow."),
    (
        "sync",
        "Synchronize the local plan tree with an external tracker\n                    target.",
    ),
    (
        "to-plan",
        "Convert accepted requirements into an implementation plan.",
    ),
    (
        "update",
        "Check for and install the latest published quoin.",
    ),
    (
        "validate",
        "Validate repository QA gates and report located defects.",
    ),
    (
        "write",
        "Build an authoring pack for spec files an agent is about to\n                    create or edit.",
    ),
];

pub(crate) fn root_usage(version: &str) -> String {
    format!(
        "{DESCRIPTION}\n\nVERSION\n  @agent-ix/quoin/{version}\n\nUSAGE\n  $ quoin [COMMAND]\n\nTOPICS\n{}\n\nCOMMANDS\n{}",
        render_entries(TOPICS),
        render_entries(COMMANDS),
    )
}

fn render_entries(entries: &[(&str, &str)]) -> String {
    entries
        .iter()
        // Oclif reserves a 17-character command field after the two-space
        // indent, then emits one separating space.  Continuation lines in the
        // retained descriptions therefore begin at column 21 as well.
        .map(|(name, description)| format!("  {name:<17} {description}"))
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
pub(crate) fn assert_catalogue_matches(command: &Command) {
    let mut actual = command
        .get_subcommands()
        .filter(|subcommand| !subcommand.is_hide_set())
        .map(Command::get_name)
        .collect::<Vec<_>>();
    actual.sort_unstable();

    let mut documented = COMMANDS
        .iter()
        .map(|(name, _)| *name)
        .chain(["semantic"])
        .collect::<Vec<_>>();
    documented.sort_unstable();
    assert_eq!(
        documented, actual,
        "root help catalogue must cover native commands"
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tc_1650_root_help_preserves_the_retained_sections_and_version_contract() {
        let rendered = root_usage("0.23.1-test");
        assert!(rendered.starts_with(DESCRIPTION));
        assert!(rendered.contains("VERSION\n  @agent-ix/quoin/0.23.1-test"));
        assert!(rendered.contains("USAGE\n  $ quoin [COMMAND]"));
        assert!(rendered.contains("TOPICS\n  assurance"));
        assert!(rendered.contains("COMMANDS\n  advise"));
        assert!(rendered.contains("Synchronize the local plan tree with an external tracker"));
    }
}
