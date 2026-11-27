// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Opt-in conformance of the emitted XMI against the external OMG model.

#[cfg(feature = "e2e-sacm-metamodel")]
use std::collections::BTreeMap;
use std::path::PathBuf;

use ix_trace_rs::trace;

#[cfg(feature = "e2e-sacm-metamodel")]
use super::{Element, parse_xml, signed_shape_export};

fn required_model_path(value: Option<std::ffi::OsString>) -> Result<PathBuf, &'static str> {
    value
        .filter(|path| !path.is_empty())
        .map(PathBuf::from)
        .ok_or("QUOIN_SACM_METAMODEL must name the external OMG SACM 2.3 metamodel XML")
}

#[trace("FR-226-AC-7")]
#[test]
fn sacm_metamodel_check_rejects_an_unset_or_empty_model_path() {
    let error = required_model_path(None).expect_err("an unset model path is rejected");
    assert!(error.contains("QUOIN_SACM_METAMODEL"));
    let error = required_model_path(Some(std::ffi::OsString::new()))
        .expect_err("an empty model path is rejected");
    assert!(error.contains("QUOIN_SACM_METAMODEL"));
}

#[trace("FR-226-AC-11")]
#[test]
#[cfg(feature = "e2e-sacm-metamodel")]
fn sacm_uses_normative_metamodel() {
    let path = required_model_path(std::env::var_os("QUOIN_SACM_METAMODEL"))
        .expect("QUOIN_SACM_METAMODEL must name the external OMG SACM 2.3 metamodel XML");
    let bytes = std::fs::read(path).expect("read the supplied SACM metamodel");
    let normative = parse_xml(&bytes);
    let mut items = Vec::new();
    normative.walk(&mut items);
    let uml = Metamodel {
        by_id: identified(&items),
        classes: items
            .iter()
            .copied()
            .filter(|item| item.attr("xmi:type") == "uml:Class")
            .map(|item| (item.attr("name"), item))
            .collect(),
    };
    let exported = signed_shape_export();
    let package = parse_xml(&exported.xmi);
    let mut emitted = Vec::new();
    package.walk(&mut emitted);
    let document_ids = identified(&emitted);
    let root_class = uml.class(package.attr("xmi:type"));
    uml.check(&package, root_class, &document_ids);

    let text = String::from_utf8(exported.xmi).expect("UTF-8 package");
    let invalid = text.replacen(
        "xmi:type=\"sacm:Resource\"",
        "xmi:type=\"sacm:Resource\" location=\"unsupported\"",
        1,
    );
    assert_ne!(text, invalid, "the fixture contains a Resource");
    let invalid = parse_xml(invalid.as_bytes());
    let mut elements = Vec::new();
    invalid.walk(&mut elements);
    let ids = identified(&elements);
    let failure = std::panic::catch_unwind(|| uml.check(&invalid, root_class, &ids))
        .expect_err("Resource.location is absent from published XML");
    let diagnostic = failure
        .downcast_ref::<String>()
        .expect("conformance diagnostic");
    assert!(
        diagnostic.contains("Resource has no SACM property location"),
        "{diagnostic}"
    );
}

#[cfg(feature = "e2e-sacm-metamodel")]
type Ids<'a> = BTreeMap<&'a str, &'a Element>;

#[cfg(feature = "e2e-sacm-metamodel")]
fn identified<'a>(items: &[&'a Element]) -> Ids<'a> {
    items
        .iter()
        .copied()
        .filter_map(|item| item.attrs.get("xmi:id").map(|id| (id.as_str(), item)))
        .collect()
}

#[cfg(feature = "e2e-sacm-metamodel")]
struct Metamodel<'a> {
    by_id: Ids<'a>,
    classes: Ids<'a>,
}

#[cfg(feature = "e2e-sacm-metamodel")]
impl<'a> Metamodel<'a> {
    fn class(&self, xmi_type: &str) -> &'a Element {
        let name = xmi_type
            .strip_prefix("sacm:")
            .expect("explicit type in SACM namespace");
        self.classes
            .get(name)
            .copied()
            .unwrap_or_else(|| panic!("missing class {name}"))
    }

    fn base_classes(&self, class: &'a Element) -> impl Iterator<Item = &'a Element> + '_ {
        class
            .children
            .iter()
            .filter(|item| item.role == "generalization")
            .map(|item| self.by_id[item.attr("general")])
    }

    fn property_name(&self, property: &'a Element) -> &'a str {
        if let Some(name) = property.attrs.get("name") {
            name
        } else {
            self.by_id
                .get(property.attr("association"))
                .map_or("", |link| link.attr("name"))
        }
    }

    fn property(&self, class: &'a Element, name: &str) -> Option<&'a Element> {
        class
            .children
            .iter()
            .find(|item| item.role == "ownedAttribute" && self.property_name(item) == name)
            .or_else(|| {
                self.base_classes(class)
                    .find_map(|base| self.property(base, name))
            })
    }

    fn is_a(&self, actual: &'a Element, declared: &'a Element) -> bool {
        actual.attr("xmi:id") == declared.attr("xmi:id")
            || self
                .base_classes(actual)
                .any(|base| self.is_a(base, declared))
    }

    fn check_required(&self, element: &Element, class: &'a Element) {
        for property in class
            .children
            .iter()
            .filter(|item| item.role == "ownedAttribute")
        {
            let lower_bound = property
                .children
                .iter()
                .find(|child| child.role == "lowerValue")
                .map_or(1, |bound| {
                    bound.attr("value").parse::<usize>().unwrap_or_default()
                });
            if lower_bound == 0 {
                continue;
            }
            let name = self.property_name(property);
            let count = if property.attr("aggregation") == "composite" {
                element
                    .children
                    .iter()
                    .filter(|child| child.role == name)
                    .count()
            } else {
                usize::from(element.attrs.contains_key(name))
            };
            assert!(
                count >= lower_bound,
                "{} is missing required property {name}",
                class.attr("name")
            );
        }
        for base in self.base_classes(class) {
            self.check_required(element, base);
        }
    }

    fn check(&self, element: &Element, class: &'a Element, document: &Ids<'_>) {
        self.check_required(element, class);
        for (name, value) in &element.attrs {
            if name.starts_with("xmi:") || name.starts_with("xmlns:") {
                continue;
            }
            let property = self
                .property(class, name)
                .unwrap_or_else(|| panic!("{} has no SACM property {name}", class.attr("name")));
            let Some(property_type) = self.by_id.get(property.attr("type")) else {
                continue;
            };
            match property_type.attr("xmi:type") {
                "uml:Enumeration" => assert!(
                    property_type
                        .children
                        .iter()
                        .any(|literal| literal.role == "ownedLiteral"
                            && literal.attr("name") == value),
                    "{value} is not in the SACM enumeration {}",
                    property_type.attr("name"),
                ),
                "uml:Class" => {
                    for reference in value.split_ascii_whitespace() {
                        let target = document.get(reference).expect("internal XMI reference");
                        assert!(
                            self.is_a(self.class(target.attr("xmi:type")), property_type),
                            "{}.{name} -> {} is not a {}",
                            class.attr("name"),
                            target.attr("xmi:type"),
                            property_type.attr("name"),
                        );
                    }
                }
                _ => {}
            }
        }
        for child in &element.children {
            let property = self.property(class, &child.role).unwrap_or_else(|| {
                panic!("{} has no SACM property {}", class.attr("name"), child.role)
            });
            assert_eq!(property.attr("aggregation"), "composite");
            let expected = self.by_id[property.attr("type")];
            let actual = child
                .attrs
                .get("xmi:type")
                .map_or(expected, |name| self.class(name));
            assert!(
                self.is_a(actual, expected),
                "{} cannot contain {} as {}",
                class.attr("name"),
                actual.attr("name"),
                child.role
            );
            self.check(child, actual, document);
        }
    }
}
