// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! Join verification-method characteristics to the facts the advisor can mint.

use std::collections::BTreeMap;

use ix_trace_rs::implements;
use serde::Serialize;

use super::MethodCatalog;

/// A catalog characteristic that no advisor fact can produce, and the methods
/// that declare it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct UnreachableCharacteristic {
    /// The value the active catalog asks the advisor to match.
    pub characteristic: String,
    /// Method identifiers that declare this value.
    pub methods: Vec<String>,
}

/// A characteristics-only method with no producible value and no reason.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct UnjustifiedUnreachableMethod {
    /// The method identifier.
    pub method: String,
}

/// Find characteristic values that the advisor cannot produce.
///
/// A returned value is a catalog defect, not an implicit exemption. Every
/// declaration without a reason remains visible to catalog validation, even
/// if the method has another applicability rule.
#[implements("FR-031-AC-16")]
#[must_use]
pub fn unreachable_characteristics(catalog: &MethodCatalog) -> Vec<UnreachableCharacteristic> {
    let mintable = crate::advise::mintable_characteristics();
    let mut findings: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for method in &catalog.methods {
        let has_reason = method
            .unreachable_reason
            .as_deref()
            .is_some_and(|reason| !reason.trim().is_empty());
        if let Some(values) = method.applicability.get("characteristics") {
            for value in values {
                if !mintable.contains(value) && !has_reason {
                    findings
                        .entry(value.clone())
                        .or_default()
                        .push(method.id.clone());
                }
            }
        }
    }
    findings
        .into_iter()
        .map(|(characteristic, mut methods)| {
            methods.sort();
            methods.dedup();
            UnreachableCharacteristic {
                characteristic,
                methods,
            }
        })
        .collect()
}

/// Find characteristics-only methods that cannot match any fact and have no
/// non-empty `unreachable_reason` declaration.
#[implements("FR-031-AC-13")]
#[must_use]
pub fn unjustified_unreachable_methods(
    catalog: &MethodCatalog,
) -> Vec<UnjustifiedUnreachableMethod> {
    let mintable = crate::advise::mintable_characteristics();
    let mut methods = catalog
        .methods
        .iter()
        .filter_map(|method| {
            if method.applicability.len() != 1 {
                return None;
            }
            let values = method.applicability.get("characteristics")?;
            let unreachable = values.iter().all(|value| !mintable.contains(value));
            let has_reason = method
                .unreachable_reason
                .as_deref()
                .is_some_and(|reason| !reason.trim().is_empty());
            (unreachable && !has_reason).then(|| UnjustifiedUnreachableMethod {
                method: method.id.clone(),
            })
        })
        .collect::<Vec<_>>();
    methods.sort_by(|left, right| left.method.cmp(&right.method));
    methods
}
