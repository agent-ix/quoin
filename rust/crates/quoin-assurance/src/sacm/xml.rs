// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! XMI serialization of SACM elements and inherited text properties.

use std::collections::BTreeSet;

use quick_xml::Writer;
use quick_xml::events::{BytesDecl, BytesEnd, BytesStart, Event};

use super::ExportError;

pub(super) struct Xml {
    writer: Writer<Vec<u8>>,
    ids: BTreeSet<String>,
}

impl Xml {
    pub(super) fn new() -> Result<Self, ExportError> {
        let mut writer = Writer::new_with_indent(Vec::new(), b' ', 2);
        writer.write_event(Event::Decl(BytesDecl::new("1.0", Some("UTF-8"), None)))?;
        Ok(Self {
            writer,
            ids: BTreeSet::new(),
        })
    }

    pub(super) fn start(&mut self, role: &str, attrs: &[(&str, &str)]) -> Result<(), ExportError> {
        let element = self.element(role, attrs)?;
        self.writer.write_event(Event::Start(element))?;
        Ok(())
    }

    pub(super) fn end(&mut self, role: &str) -> Result<(), ExportError> {
        self.writer.write_event(Event::End(BytesEnd::new(role)))?;
        Ok(())
    }

    pub(super) fn empty(&mut self, role: &str, attrs: &[(&str, &str)]) -> Result<(), ExportError> {
        let element = self.element(role, attrs)?;
        self.writer.write_event(Event::Empty(element))?;
        Ok(())
    }

    pub(super) fn name(&mut self, name: &str) -> Result<(), ExportError> {
        self.empty("name", &[("content", name)])
    }

    pub(super) fn description(&mut self, text: &str) -> Result<(), ExportError> {
        self.start("description", &[])?;
        self.start("content", &[])?;
        self.empty("value", &[("content", text)])?;
        self.end("content")?;
        self.end("description")
    }

    pub(super) fn finish(self) -> Vec<u8> {
        self.writer.into_inner()
    }

    fn element<'a>(
        &mut self,
        role: &'a str,
        attrs: &[(&str, &str)],
    ) -> Result<BytesStart<'a>, ExportError> {
        let mut element = BytesStart::new(role);
        for &(key, value) in attrs {
            if key == "xmi:id" {
                let mut bytes = value.bytes();
                if !bytes
                    .next()
                    .is_some_and(|b| b.is_ascii_alphabetic() || b == b'_')
                    || !bytes.all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-' | b'.'))
                {
                    return Err(ExportError::InvalidId {
                        id: value.to_owned(),
                    });
                }
                if !self.ids.insert(value.to_owned()) {
                    return Err(ExportError::DuplicateId {
                        id: value.to_owned(),
                    });
                }
            }
            if value.chars().any(|ch| {
            !matches!(ch, '\t' | '\n' | '\r' | '\u{20}'..='\u{d7ff}' | '\u{e000}'..='\u{fffd}' | '\u{10000}'..='\u{10ffff}')
        }) {
            return Err(ExportError::InvalidXmlText { property: key.to_owned() });
        }
            element.push_attribute((key, value));
        }
        Ok(element)
    }
}
