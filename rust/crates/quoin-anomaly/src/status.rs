// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The status vocabulary and the legal moves between its members.

use serde::Serialize;

/// An anomaly's lifecycle status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Status {
    /// Reported, not yet assessed.
    Identified,
    /// Safety impact assessed.
    Assessed,
    /// A fix has been made.
    Resolved,
    /// Final.
    Closed,
}

/// The single legal moves (spec-artifacts-process FR-014).
const MOVES: &[(Status, Status)] = &[
    (Status::Identified, Status::Assessed),
    (Status::Assessed, Status::Resolved),
    (Status::Resolved, Status::Closed),
    (Status::Resolved, Status::Assessed),
    (Status::Identified, Status::Closed),
    (Status::Assessed, Status::Closed),
];

impl Status {
    /// Parse an authored status word; an unknown word is `None`.
    #[must_use]
    pub fn parse(word: &str) -> Option<Self> {
        match word {
            "identified" => Some(Self::Identified),
            "assessed" => Some(Self::Assessed),
            "resolved" => Some(Self::Resolved),
            "closed" => Some(Self::Closed),
            _ => None,
        }
    }

    /// The authored word.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Identified => "identified",
            Self::Assessed => "assessed",
            Self::Resolved => "resolved",
            Self::Closed => "closed",
        }
    }

    /// Whether `to` is reachable from `self` through one or more legal moves.
    ///
    /// Squash-merged history can hide intermediate states, so a change between
    /// two recorded states is legal when the later is reachable from the
    /// earlier. `identified` is never re-entered and `closed` is final.
    #[must_use]
    pub fn reaches(self, to: Self) -> bool {
        let mut seen = vec![self];
        let mut frontier = vec![self];
        let mut reached = false;
        while let Some(from) = frontier.pop() {
            for &(a, b) in MOVES {
                if a != from {
                    continue;
                }
                if b == to {
                    reached = true;
                }
                if !seen.contains(&b) {
                    seen.push(b);
                    frontier.push(b);
                }
            }
        }
        reached
    }
}
