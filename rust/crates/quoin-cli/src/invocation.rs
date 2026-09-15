// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Invocation-wide shell options shared by command adapters.
//!
//! Oclif's former base command supplied these once for every command. Keeping
//! the parsed form in one scoped, thread-local value makes the Clap shell do
//! the same without leaking a process-global environment mutation between tests
//! or future in-process callers.

use std::cell::RefCell;
use std::path::PathBuf;

use clap::ArgMatches;

#[derive(Clone, Default)]
pub(crate) struct Invocation {
    config_root: Option<PathBuf>,
    no_project_config: bool,
}

thread_local! {
    static CURRENT: RefCell<Invocation> = RefCell::new(Invocation::default());
}

impl Invocation {
    pub(crate) fn from_matches(matches: &ArgMatches) -> Self {
        Self {
            config_root: matches.get_one::<String>("config_root").map(PathBuf::from),
            no_project_config: matches.get_flag("no_project_config"),
        }
    }

    pub(crate) fn config_root(&self) -> Option<&PathBuf> {
        self.config_root.as_ref()
    }

    pub(crate) const fn no_project_config(&self) -> bool {
        self.no_project_config
    }
}

pub(crate) fn with_current<T>(invocation: Invocation, run: impl FnOnce() -> T) -> T {
    CURRENT.with(|current| {
        let previous = current.replace(invocation);
        let result = run();
        current.replace(previous);
        result
    })
}

pub(crate) fn current() -> Invocation {
    CURRENT.with(|current| current.borrow().clone())
}

#[cfg(test)]
#[allow(clippy::expect_used, reason = "test assertions report failures")]
mod tests {
    use std::path::PathBuf;

    use super::{Invocation, current, with_current};
    use clap::{Arg, ArgAction, Command};

    /// Trace: FR-016, FR-062
    #[test]
    fn tc_373_invocation_context_is_scoped_to_one_parsed_command() {
        let matches = Command::new("quoin")
            .arg(Arg::new("config_root").long("config-root"))
            .arg(
                Arg::new("no_project_config")
                    .long("no-project-config")
                    .action(ArgAction::SetTrue),
            )
            .try_get_matches_from(["quoin", "--config-root", "/tmp/ix", "--no-project-config"])
            .expect("global grammar parses");
        let invocation = Invocation::from_matches(&matches);
        with_current(invocation, || {
            assert_eq!(current().config_root(), Some(&PathBuf::from("/tmp/ix")));
            assert!(current().no_project_config());
        });
        assert!(
            current().config_root().is_none(),
            "context is restored after dispatch"
        );
    }
}
