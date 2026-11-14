// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! The command tree the built binary describes about itself, read from its
//! `--help` text. Every NFR-003 population is enumerated from this tree, so a
//! command, flag or argument added to the binary joins the population without
//! an edit here.

#![allow(
    clippy::expect_used,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]

use std::path::Path;

use super::{quoin, text};

/// A flag a command's help lists under `FLAGS`.
pub(crate) struct Flag {
    /// The long name with its dashes, for example `--repo`.
    pub(crate) name: String,
    pub(crate) takes_value: bool,
    pub(crate) required: bool,
    pub(crate) description: String,
}

/// A positional argument a command's help lists under `ARGUMENTS`.
pub(crate) struct Argument {
    /// The name as the help writes it, without brackets.
    pub(crate) name: String,
    pub(crate) required: bool,
    pub(crate) description: String,
}

/// What one command's help says about it.
pub(crate) struct CommandHelp {
    /// The words that select it, empty for the root.
    pub(crate) path: Vec<String>,
    /// The words of each subcommand its `COMMANDS` block lists.
    /// The first line of its help.
    pub(crate) summary: String,
    pub(crate) subcommands: Vec<String>,
    pub(crate) flags: Vec<Flag>,
    pub(crate) arguments: Vec<Argument>,
}

impl CommandHelp {
    /// The path as typed, or `quoin` for the root.
    pub(crate) fn id(&self) -> String {
        if self.path.is_empty() {
            "quoin".to_owned()
        } else {
            self.path.join(" ")
        }
    }

    /// The flags the help marks `(required)` and the positionals it does not
    /// bracket: what a bare invocation must name when it is refused.
    pub(crate) fn required_names(&self) -> Vec<String> {
        let flags = self
            .flags
            .iter()
            .filter(|flag| flag.required)
            .map(|flag| flag.name.trim_start_matches('-').to_owned());
        let arguments = self
            .arguments
            .iter()
            .filter(|argument| argument.required)
            .map(|argument| argument.name.clone());
        flags.chain(arguments).collect()
    }
}

/// One value a command takes that a probe replaces: a flag, or the positional.
pub(crate) struct Target {
    /// What the id calls it: the flag with its dashes, or the lowercased
    /// positional name.
    pub(crate) label: String,
    flag: Option<String>,
}

impl CommandHelp {
    /// The value-taking flags and the positionals `wanted` selects.
    pub(crate) fn targets(
        &self,
        flag: impl Fn(&Flag) -> bool,
        argument: impl Fn(&Argument) -> bool,
    ) -> Vec<Target> {
        let flags = self
            .flags
            .iter()
            .filter(|f| f.takes_value && flag(f))
            .map(|f| Target {
                label: f.name.clone(),
                flag: Some(f.name.clone()),
            });
        let arguments = self
            .arguments
            .iter()
            .filter(|a| argument(a))
            .map(|a| Target {
                label: a.name.to_lowercase(),
                flag: None,
            });
        flags.chain(arguments).collect()
    }

    /// Whether \`target\` is the only thing the help marks required, so that
    /// giving it a value needs no placeholder for anything else.
    pub(crate) fn requires_only(&self, target: &Target) -> bool {
        self.flags
            .iter()
            .all(|f| !f.required || Some(&f.name) == target.flag.as_ref())
    }

    /// The arguments that give \`target\` \`value\`. A positional target is the
    /// only positional; beside a flag target every positional gets \`.\`.
    pub(crate) fn line(&self, target: &Target, value: &str) -> Vec<String> {
        match &target.flag {
            Some(flag) => [flag.clone(), value.to_owned()]
                .into_iter()
                .chain(self.arguments.iter().map(|_| ".".to_owned()))
                .collect(),
            None => vec![value.to_owned()],
        }
    }
}

/// The root and every command reachable from it through `COMMANDS` blocks,
/// parents before children, siblings in the order the help lists them.
pub(crate) fn walk(home: &Path) -> Vec<CommandHelp> {
    let mut found = Vec::new();
    let mut pending = vec![Vec::<String>::new()];
    while let Some(path) = pending.pop() {
        let mut arguments: Vec<&str> = path.iter().map(String::as_str).collect();
        arguments.push("--help");
        let help = text(&quoin(home, &arguments, |_| {}).stdout);
        let command = parse(path, &help);
        pending.extend(
            command
                .subcommands
                .iter()
                .rev()
                .map(|name| [command.path.clone(), vec![name.clone()]].concat()),
        );
        found.push(command);
    }
    found
}

fn parse(path: Vec<String>, help: &str) -> CommandHelp {
    let mut command = CommandHelp {
        path,
        summary: help.lines().next().unwrap_or_default().to_owned(),
        subcommands: Vec::new(),
        flags: Vec::new(),
        arguments: Vec::new(),
    };
    let mut section = "";
    let mut commands_list_basename = false;
    for line in help.lines() {
        if !line.starts_with(' ') {
            let heading = line.trim().trim_end_matches(':');
            commands_list_basename =
                heading.eq_ignore_ascii_case("COMMANDS") && heading != "COMMANDS";
            section = if heading.eq_ignore_ascii_case("COMMANDS") {
                "COMMANDS"
            } else {
                heading
            };
            continue;
        }
        let entry = line.starts_with("  ") && !line.starts_with("   ");
        match section {
            "COMMANDS" if entry => {
                let words: Vec<&str> = line
                    .trim_start()
                    .split("  ")
                    .next()
                    .unwrap_or_default()
                    .split_whitespace()
                    .collect();
                let full_path = words.len() == command.path.len() + 1
                    && words.iter().zip(&command.path).all(|(a, b)| a == b);
                let basename =
                    commands_list_basename && !command.path.is_empty() && words.len() == 1;
                if full_path || basename {
                    command
                        .subcommands
                        .extend(words.last().map(|word| (*word).to_owned()));
                }
            }
            "ARGUMENTS" if entry => {
                let (token, description) = split_entry(line);
                command.arguments.push(Argument {
                    name: token.trim_matches(|c| "[].".contains(c)).to_owned(),
                    required: !token.starts_with('['),
                    description,
                });
            }
            "FLAGS" if entry => {
                let (token, description) = split_entry(line);
                let name = token.split('=').next().unwrap_or_default().to_owned();
                command.flags.push(Flag {
                    name,
                    takes_value: token.contains('='),
                    required: description.contains("(required)"),
                    description,
                });
            }
            "FLAGS" => {
                if let Some(flag) = command.flags.last_mut() {
                    flag.description.push(' ');
                    flag.description.push_str(line.trim());
                    flag.required |= flag.description.contains("(required)");
                }
            }
            _ => {}
        }
    }
    command
}

/// An entry line split into its first token and the rest of the line. A flag
/// whose description starts on the next line has an empty rest.
fn split_entry(line: &str) -> (&str, String) {
    let line = line.trim_start();
    let line = line
        .strip_prefix('-')
        .filter(|rest| !rest.starts_with('-'))
        .and_then(|rest| rest.split_once(", "))
        .map_or(line, |(_, long)| long);
    line.split_once(char::is_whitespace)
        .map_or((line, String::new()), |(token, rest)| {
            (token, rest.trim().to_owned())
        })
}

#[cfg(test)]
mod tests {
    use super::parse;

    #[test]
    fn tc_3003_title_case_clap_commands_section_keeps_nested_routes() {
        let parent = parse(
            vec!["class".to_owned()],
            "Report activities\n\nUsage: quoin class <COMMAND>\n\nCommands:\n  obligations  Report activities and evidence\n\nOptions:\n  -h, --help  Print help\n",
        );
        assert_eq!(parent.subcommands, ["obligations"]);
    }
}
