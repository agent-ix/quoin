// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
// Extract one `key = "…"` value out of a TOML inline-table line.
//
// `include!`d by `build.rs`.

/// The value of `key = "…"` inside one inline table line.
///
/// The key must begin at a TOML token boundary — start of line, whitespace,
/// `{` or `,` — and be followed by `=`. A naive `split(key)` matched the same
/// letters anywhere in the line, including inside the dependency's git URL,
/// and was correct only because no current URL happens to contain `rev` or
/// `version`. A repository rename would have mis-parsed the pin silently.
fn field(line: &str, key: &str) -> Option<String> {
    line.match_indices(key)
        .filter(|(index, _)| {
            line.get(..*index)
                .and_then(|before| before.chars().next_back())
                .is_none_or(|char| char.is_whitespace() || char == '{' || char == ',')
        })
        .find_map(|(index, _)| {
            let after_key = line.get(index.saturating_add(key.len())..)?;
            let after_equals = after_key.trim_start().strip_prefix('=')?;
            let rest = after_equals.trim_start().strip_prefix('"')?;
            let closing = rest.find('"')?;
            rest.get(..closing).map(str::to_string)
        })
}
