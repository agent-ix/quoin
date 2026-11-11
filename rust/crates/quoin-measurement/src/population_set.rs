// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! Checking an observation's stated population against the plan-declared
//! enumeration (PLAT-110, FR-131).
//!
//! The plan names the ids a percentage metric is taken over. An observation
//! lists the ids it `examined` and the ids that `matched`; the check refuses
//! any list that is not exactly the plan's set (examined) or a subset of it
//! (matched). The producer's list is evidence about the set, never the
//! denominator itself.

use std::collections::BTreeSet;

use quoin_store::JsonValue;

use crate::types::plan::PopulationSet;

/// The member of `population` holding the ids the producer examined.
pub const EXAMINED_IDS: &str = "examined_ids";
/// The member of `population` holding the ids that matched.
pub const MATCHED_IDS: &str = "matched_ids";
/// The member of `population` counting the ids examined.
pub const EXAMINED: &str = "examined";
/// The member of `population` counting the ids that matched.
pub const MATCHED: &str = "matched";

/// How many ids a finding names before summarising the rest.
const NAMED_IDS: usize = 5;
/// The longest id a finding spells out in full.
const NAMED_ID_CHARS: usize = 64;

/// A population count as the stored document states it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum StatedCount {
    /// The member is not there.
    Absent,
    /// The member is a number.
    Number(f64),
    /// The member is there and is not a number.
    NotNumber,
}

impl StatedCount {
    /// Read a stored member.
    #[must_use]
    pub fn of(value: Option<&JsonValue>) -> Self {
        match value {
            None => Self::Absent,
            Some(stated) => stated.as_f64().map_or(Self::NotNumber, Self::Number),
        }
    }
}

/// Why a stated population is not the plan's enumeration.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SetFault {
    /// The named id list is absent, so nothing says what was examined.
    Unstated(&'static str),
    /// The named member is not a list of distinct, non-empty strings, or a
    /// stated count disagrees with it.
    Malformed(&'static str, String),
    /// Ids that are not in the plan's set.
    Outside(&'static str, Vec<String>),
    /// Ids of the plan's set that were not examined.
    Partial(Vec<String>),
}

/// The ids `value` lists.
fn ids(member: &'static str, value: &JsonValue) -> Result<BTreeSet<String>, SetFault> {
    let malformed = |why: String| SetFault::Malformed(member, why);
    let JsonValue::Array(items) = value else {
        return Err(malformed("it must be a list of strings".to_owned()));
    };
    let mut seen = BTreeSet::new();
    for item in items {
        let JsonValue::String(id) = item else {
            return Err(malformed("it must be a list of strings".to_owned()));
        };
        if id.is_empty() {
            return Err(malformed("an id must not be empty".to_owned()));
        }
        if !seen.insert(id.clone()) {
            return Err(malformed(format!("id `{id}` is listed twice")));
        }
    }
    Ok(seen)
}

/// Check one observation's stated population against `set`.
///
/// `examined` and `matched` are the stored counts. Both must be stated, and
/// each must equal the length of its list: a count is never trusted over the
/// list.
///
/// # Errors
///
/// The first [`SetFault`], checked in this order: an absent list, a malformed
/// list, an absent or disagreeing count, ids outside the set, then set ids
/// not examined.
pub fn check(
    set: &PopulationSet,
    examined_ids: Option<&JsonValue>,
    matched_ids: Option<&JsonValue>,
    examined: StatedCount,
    matched: StatedCount,
) -> Result<(), SetFault> {
    let examined_list = ids(
        EXAMINED_IDS,
        examined_ids.ok_or(SetFault::Unstated(EXAMINED_IDS))?,
    )?;
    let matched_list = ids(
        MATCHED_IDS,
        matched_ids.ok_or(SetFault::Unstated(MATCHED_IDS))?,
    )?;
    for (member, list, count) in [
        (EXAMINED, &examined_list, examined),
        (MATCHED, &matched_list, matched),
    ] {
        match count {
            StatedCount::Absent => return Err(SetFault::Unstated(member)),
            StatedCount::NotNumber => {
                return Err(SetFault::Malformed(
                    member,
                    "it must be a number".to_owned(),
                ));
            }
            StatedCount::Number(count) if u32::try_from(list.len()).map(f64::from) != Ok(count) => {
                return Err(SetFault::Malformed(
                    member,
                    format!("it lists {} ids but the count states {count}", list.len()),
                ));
            }
            StatedCount::Number(_) => {}
        }
    }
    for (member, list) in [(EXAMINED_IDS, &examined_list), (MATCHED_IDS, &matched_list)] {
        let outside: Vec<String> = list
            .iter()
            .filter(|id| !set.contains(id))
            .cloned()
            .collect();
        if !outside.is_empty() {
            return Err(SetFault::Outside(member, outside));
        }
    }
    let missing: Vec<String> = set
        .iter()
        .filter(|id| !examined_list.contains(*id))
        .map(str::to_owned)
        .collect();
    if missing.is_empty() {
        Ok(())
    } else {
        Err(SetFault::Partial(missing))
    }
}

/// The first few ids, each cut to a bounded length, then a count of the rest,
/// so a finding stays small however large the set or the producer's lists.
fn named(ids: &[String]) -> String {
    let shown: Vec<String> = ids
        .iter()
        .take(NAMED_IDS)
        .map(|id| {
            let mut cut: String = id.chars().take(NAMED_ID_CHARS).collect();
            if cut.len() < id.len() {
                cut.push_str("...");
            }
            cut
        })
        .collect();
    match ids.len().saturating_sub(NAMED_IDS) {
        0 => shown.join(", "),
        more => format!("{} and {more} more", shown.join(", ")),
    }
}

impl SetFault {
    /// A sentence naming the fault, for `metric`.
    #[must_use]
    pub fn describe(&self, metric: &str, plan: &str) -> String {
        match self {
            Self::Unstated(member) => format!(
                "metric `{metric}` states no population.{member}; plan {plan} declares the \
                 population it is taken over"
            ),
            Self::Malformed(member, why) => {
                format!("metric `{metric}` states population.{member} wrongly: {why}")
            }
            Self::Outside(member, ids) => format!(
                "metric `{metric}` lists population.{member} ids outside plan {plan}'s \
                 population: {}",
                named(ids)
            ),
            Self::Partial(ids) => format!(
                "metric `{metric}` did not examine {} of plan {plan}'s population ids: {}",
                ids.len(),
                named(ids)
            ),
        }
    }

    /// The JSON path, inside the observation, of the member the fault concerns.
    #[must_use]
    pub fn path(&self) -> &'static str {
        let member = match self {
            Self::Unstated(member) | Self::Malformed(member, _) | Self::Outside(member, _) => {
                *member
            }
            Self::Partial(_) => EXAMINED_IDS,
        };
        match member {
            MATCHED_IDS => "population.matched_ids",
            EXAMINED => "population.examined",
            MATCHED => "population.matched",
            _ => "population.examined_ids",
        }
    }
}
