// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! Which adapter a caller asked for.
//!
//! Ports `GRAPH_ADAPTER_NAMES`, `GraphAdapterName` and `selectGraphAdapter`
//! (`src/measurement/graph-adapters.ts:18-22, 428-434`).

use std::fmt;

use crate::error::{GraphAdapterError, GraphAdapterErrorCode, Result};

/// The versioned adapters this crate offers, in the retained declaration order.
///
/// The order is the order `selectGraphAdapter`'s refusal lists them in, so it
/// is part of the message a caller reads.
pub const GRAPH_ADAPTER_NAMES: [&str; 2] = ["quire-assurance-v1", "quire-code-graph-quality-v1"];

/// One adapter, named.
///
/// The retained `GraphAdapterName` is a string union, so `selectGraphAdapter`
/// returns a *narrowed string*. Here it is an enum: a caller that holds one
/// cannot hold a name outside the union, and a new adapter is a new variant
/// rather than a new string every match must remember.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum GraphAdapterName {
    /// Quire's authoritative assurance export, admitted without translation.
    QuireAssuranceV1,
    /// `quire-code-rs`'s graph-quality observation, transcribed.
    QuireCodeGraphQualityV1,
}

impl GraphAdapterName {
    /// Every adapter, in the order [`GRAPH_ADAPTER_NAMES`] states them.
    pub const ALL: [Self; 2] = [Self::QuireAssuranceV1, Self::QuireCodeGraphQualityV1];

    /// The stable wire spelling.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::QuireAssuranceV1 => GRAPH_ADAPTER_NAMES[0],
            Self::QuireCodeGraphQualityV1 => GRAPH_ADAPTER_NAMES[1],
        }
    }
}

impl fmt::Display for GraphAdapterName {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// Read an adapter name.
///
/// # Errors
///
/// [`GraphAdapterErrorCode::UnknownAdapter`], listing what is available.
pub fn select_graph_adapter(name: &str) -> Result<GraphAdapterName> {
    GraphAdapterName::ALL
        .into_iter()
        .find(|known| known.as_str() == name)
        .ok_or_else(|| {
            GraphAdapterError::new(
                GraphAdapterErrorCode::UnknownAdapter,
                format!(
                    "unknown adapter `{name}`; available: {}",
                    GRAPH_ADAPTER_NAMES.join(", ")
                ),
            )
        })
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]
mod tests {
    use super::{GRAPH_ADAPTER_NAMES, GraphAdapterName, select_graph_adapter};
    use crate::error::GraphAdapterErrorCode;

    /// Every declared name selects, and the enum and the list agree.
    #[test]
    fn tc_475_020_every_declared_name_selects() {
        assert_eq!(GraphAdapterName::ALL.len(), GRAPH_ADAPTER_NAMES.len());
        for (name, expected) in GRAPH_ADAPTER_NAMES.into_iter().zip(GraphAdapterName::ALL) {
            assert_eq!(select_graph_adapter(name).unwrap(), expected);
            assert_eq!(expected.as_str(), name);
        }
    }

    /// An undeclared name is refused, and the refusal names what is available.
    #[test]
    fn tc_475_021_an_undeclared_name_is_refused() {
        let error = select_graph_adapter("quire-assurance-v2").unwrap_err();
        assert_eq!(error.code(), GraphAdapterErrorCode::UnknownAdapter);
        assert_eq!(
            error.message(),
            "unknown adapter `quire-assurance-v2`; available: quire-assurance-v1, \
             quire-code-graph-quality-v1"
        );
    }
}
