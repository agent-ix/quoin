// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Package-URL equality as FR-142 defines it: the type compared
//! case-insensitively, the namespace, name and version percent-decoded and
//! compared as written, qualifiers and subpath ignored. No ecosystem-specific
//! normalisation is applied.

/// The identity a purl names.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct PurlIdentity {
    /// The package type, lower-cased.
    pub kind: String,
    /// The percent-decoded namespace segments, joined with `/`; empty when none.
    pub namespace: String,
    /// The percent-decoded name.
    pub name: String,
    /// The percent-decoded version, when the purl states one.
    pub version: Option<String>,
}

impl PurlIdentity {
    /// Parse `pkg:<type>/[<namespace>/]<name>[@<version>][?<qualifiers>][#<subpath>]`.
    /// `None` for anything else.
    #[must_use]
    pub fn parse(purl: &str) -> Option<Self> {
        let rest = purl.strip_prefix("pkg:")?;
        let core = rest.split(['#', '?']).next().unwrap_or_default();
        let (kind, path) = core.split_once('/')?;
        let (namespace, tail) = path.rsplit_once('/').unwrap_or(("", path));
        let (name, version) = match tail.split_once('@') {
            Some((name, version)) => (name, Some(version).filter(|v| !v.is_empty())),
            None => (tail, None),
        };
        if kind.is_empty() || name.is_empty() {
            return None;
        }
        Some(Self {
            kind: kind.to_ascii_lowercase(),
            namespace: percent_decode(namespace),
            name: percent_decode(name),
            version: version.map(percent_decode),
        })
    }

    /// The same package, whatever its version.
    #[must_use]
    pub fn same_package(&self, other: &Self) -> bool {
        self.kind == other.kind && self.namespace == other.namespace && self.name == other.name
    }
}

/// Decode `%XX` escapes. A malformed escape, or bytes that are not UTF-8 once
/// decoded, leave the text as written.
#[must_use]
pub fn percent_decode(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while let Some(&byte) = bytes.get(index) {
        let decoded = (byte == b'%')
            .then(|| bytes.get(index.saturating_add(1)..index.saturating_add(3)))
            .flatten()
            .and_then(|pair| std::str::from_utf8(pair).ok())
            .and_then(|pair| u8::from_str_radix(pair, 16).ok());
        if let Some(value) = decoded {
            out.push(value);
            index = index.saturating_add(3);
        } else {
            out.push(byte);
            index = index.saturating_add(1);
        }
    }
    String::from_utf8(out).unwrap_or_else(|_| text.to_owned())
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]
mod tests {
    use super::PurlIdentity;

    #[test]
    fn type_is_case_insensitive_and_segments_are_decoded() {
        let a = PurlIdentity::parse("pkg:NPM/%40scope/name@1.0.0?arch=x#sub").unwrap();
        let b = PurlIdentity::parse("pkg:npm/@scope/name@1.0.0").unwrap();
        assert_eq!(a, b);
        assert_eq!(a.namespace, "@scope");
        assert!(
            PurlIdentity::parse("pkg:npm/Name").unwrap()
                != PurlIdentity::parse("pkg:npm/name").unwrap()
        );
        assert!(PurlIdentity::parse("not a purl").is_none());
        assert!(PurlIdentity::parse("pkg:npm/").is_none());
    }
}
