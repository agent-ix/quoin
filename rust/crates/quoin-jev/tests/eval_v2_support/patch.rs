// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Applying a mutation's unified diff to one file's text (PLAT-1027).
//!
//! An external corpus row (PLAT-1026) carries no body: it names a file in a
//! sibling checkout and a content digest, and its mutation is a `git diff` of
//! that one file. This module turns the pristine file plus the patch into the
//! mutated file, so the harness can cut the mutated symbol out of it. In-repo
//! rows never pass through here: their bodies are already the mutated text
//! and their `patch` is provenance only.
//!
//! Strict by design. Context and removed lines must match exactly; a hunk
//! that does not match at its stated line is searched for once, forward of
//! the previous hunk, and must match in exactly one place. Anything else is an
//! error, never a partial application.

/// One parsed hunk: the old range it claims (1-based start, line count), and
/// its lines with their `' '`/`-`/`+` tag.
struct Hunk {
    old_start: usize,
    old_count: usize,
    lines: Vec<(char, String)>,
}

impl Hunk {
    /// The 0-based index the hunk's old lines start at. A hunk with an empty
    /// old range (`-a,0`, a pure insertion with no context) inserts AFTER
    /// line `a`, so its index is `a`, not `a - 1`.
    const fn stated_index(&self) -> usize {
        if self.old_count == 0 {
            self.old_start
        } else {
            self.old_start.saturating_sub(1)
        }
    }
}

/// Parses `@@ -a,b +c,d @@` into `(a, b)`; `b` defaults to 1 when omitted.
fn old_range(header: &str) -> Option<(usize, usize)> {
    let rest = header.strip_prefix("@@ -")?;
    let range = rest.split_whitespace().next()?;
    let mut parts = range.split(',');
    let start = parts.next()?.parse().ok()?;
    let count = parts.next().map_or(Some(1), |count| count.parse().ok())?;
    Some((start, count))
}

fn parse(patch: &str) -> Result<Vec<Hunk>, String> {
    let mut hunks: Vec<Hunk> = Vec::new();
    let mut files = 0usize;
    for line in patch.lines() {
        if line.starts_with("+++ ") {
            files += 1;
            continue;
        }
        if let Some(hunk) = hunks.last_mut().filter(|_| !line.starts_with("@@")) {
            match line.chars().next() {
                Some(tag @ (' ' | '-' | '+')) => {
                    hunk.lines
                        .push((tag, line.get(1..).unwrap_or_default().to_owned()));
                }
                // An empty line inside a hunk is an empty context line whose
                // leading space an editor stripped.
                None => hunk.lines.push((' ', String::new())),
                Some('\\') => {}
                Some(_) => return Err(format!("unrecognized line in a hunk: {line:?}")),
            }
            continue;
        }
        if line.starts_with("@@") {
            let (old_start, old_count) =
                old_range(line).ok_or_else(|| format!("bad hunk header {line:?}"))?;
            hunks.push(Hunk {
                old_start,
                old_count,
                lines: Vec::new(),
            });
        }
    }
    if files > 1 {
        return Err(format!("the patch touches {files} files; one is supported"));
    }
    if hunks.is_empty() {
        return Err("the patch has no hunks".to_owned());
    }
    Ok(hunks)
}

/// Applies `patch` to `source`.
///
/// # Errors
/// When the patch has no hunk, spans more than one file, or a hunk's context
/// does not match `source` in exactly one place.
pub(crate) fn apply_unified_patch(source: &str, patch: &str) -> Result<String, String> {
    let hunks = parse(patch)?;
    let lines: Vec<&str> = source.lines().collect();
    let mut out: Vec<String> = Vec::new();
    let mut cursor = 0usize;
    for (index, hunk) in hunks.iter().enumerate() {
        let old: Vec<&str> = hunk
            .lines
            .iter()
            .filter(|(tag, _)| *tag != '+')
            .map(|(_, text)| text.as_str())
            .collect();
        let matches_at = |at: usize| lines.get(at..at + old.len()) == Some(old.as_slice());
        let stated = hunk.stated_index();
        let at = if stated >= cursor && matches_at(stated) {
            stated
        } else {
            let found: Vec<usize> = (cursor..=lines.len())
                .filter(|at| matches_at(*at))
                .collect();
            match found.as_slice() {
                [only] => *only,
                [] => return Err(format!("hunk {} does not match the source", index + 1)),
                _ => {
                    return Err(format!(
                        "hunk {} matches the source in {} places",
                        index + 1,
                        found.len()
                    ));
                }
            }
        };
        out.extend(
            lines
                .get(cursor..at)
                .unwrap_or_default()
                .iter()
                .map(|line| (*line).to_owned()),
        );
        out.extend(
            hunk.lines
                .iter()
                .filter(|(tag, _)| *tag != '-')
                .map(|(_, text)| text.clone()),
        );
        cursor = at + old.len();
    }
    out.extend(
        lines
            .get(cursor..)
            .unwrap_or_default()
            .iter()
            .map(|line| (*line).to_owned()),
    );
    let mut text = out.join("\n");
    if source.ends_with('\n') {
        text.push('\n');
    }
    Ok(text)
}
