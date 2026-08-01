use html5ever::serialize::TraversalScope::*;
use html5ever::serialize::{AttrRef, Serialize, Serializer, TraversalScope};
use html5ever::{local_name, namespace_url, ns, LocalName, QualName};
use std::fmt;
use std::fs::File;
use std::io;
use std::io::Write;
use std::path::Path;

use crate::html::tree::{NodeData, NodeRef};

/// Per-element bookkeeping kept on the serializer stack, mirroring html5ever's
/// `ElemInfo`: the element's local name (only when in the HTML namespace, used
/// to decide raw-text escaping) and whether its children/closing tag are
/// suppressed (void elements).
#[derive(Default)]
struct ElemInfo {
    html_name: Option<LocalName>,
    ignore_children: bool,
}

/// A drop-in replacement for html5ever's `HtmlSerializer` that produces
/// byte-for-byte identical output but escapes text and attribute values with a
/// run-based scanner (bulk-copying the bytes between special characters) rather
/// than dispatching every character through `core::fmt`.
struct HtmlSerializer<W: Write> {
    writer: W,
    stack: Vec<ElemInfo>,
}

fn tagname(name: &QualName) -> LocalName {
    name.local.clone()
}

impl<W: Write> HtmlSerializer<W> {
    fn new(writer: W) -> Self {
        // Matches html5ever's `new()` for a top-level `IncludeNode` traversal:
        // a single default parent frame with no HTML name.
        HtmlSerializer {
            writer,
            stack: vec![ElemInfo::default()],
        }
    }

    fn parent(&mut self) -> &mut ElemInfo {
        self.stack.last_mut().expect("no parent ElemInfo")
    }

    /// Escape `text`, writing runs of ordinary bytes in bulk. Equivalent to
    /// html5ever's per-`char` escaper: `&` -> `&amp;`, U+00A0 -> `&nbsp;`,
    /// `"` -> `&quot;` (attribute mode only), and `<`/`>` -> `&lt;`/`&gt;`
    /// (text mode only); every other byte passes through unchanged.
    fn write_escaped(&mut self, text: &str, attr_mode: bool) -> io::Result<()> {
        let bytes = text.as_bytes();
        let (mut last, mut i) = (0, 0);
        while i < bytes.len() {
            let (entity, len): (&[u8], usize) = match bytes[i] {
                b'&' => (b"&amp;", 1),
                b'"' if attr_mode => (b"&quot;", 1),
                b'<' if !attr_mode => (b"&lt;", 1),
                b'>' if !attr_mode => (b"&gt;", 1),
                // U+00A0 non-breaking space is 0xC2 0xA0 in UTF-8.
                0xC2 if bytes.get(i + 1) == Some(&0xA0) => (b"&nbsp;", 2),
                _ => {
                    i += 1;
                    continue;
                }
            };
            self.writer.write_all(&bytes[last..i])?;
            self.writer.write_all(entity)?;
            i += len;
            last = i;
        }
        self.writer.write_all(&bytes[last..])
    }
}

impl<W: Write> Serializer for HtmlSerializer<W> {
    fn start_elem<'a, AttrIter>(&mut self, name: QualName, attrs: AttrIter) -> io::Result<()>
    where
        AttrIter: Iterator<Item = AttrRef<'a>>,
    {
        let html_name = match name.ns {
            ns!(html) => Some(name.local.clone()),
            _ => None,
        };

        if self.parent().ignore_children {
            self.stack.push(ElemInfo {
                html_name,
                ignore_children: true,
            });
            return Ok(());
        }

        self.writer.write_all(b"<")?;
        self.writer.write_all(tagname(&name).as_bytes())?;
        for (name, value) in attrs {
            self.writer.write_all(b" ")?;

            match name.ns {
                ns!() => (),
                ns!(xml) => self.writer.write_all(b"xml:")?,
                ns!(xmlns) => {
                    if name.local != local_name!("xmlns") {
                        self.writer.write_all(b"xmlns:")?;
                    }
                }
                ns!(xlink) => self.writer.write_all(b"xlink:")?,
                _ => self.writer.write_all(b"unknown_namespace:")?,
            }

            self.writer.write_all(name.local.as_bytes())?;
            self.writer.write_all(b"=\"")?;
            self.write_escaped(value, true)?;
            self.writer.write_all(b"\"")?;
        }
        self.writer.write_all(b">")?;

        let ignore_children = name.ns == ns!(html)
            && matches!(
                name.local,
                local_name!("area")
                    | local_name!("base")
                    | local_name!("basefont")
                    | local_name!("bgsound")
                    | local_name!("br")
                    | local_name!("col")
                    | local_name!("embed")
                    | local_name!("frame")
                    | local_name!("hr")
                    | local_name!("img")
                    | local_name!("input")
                    | local_name!("keygen")
                    | local_name!("link")
                    | local_name!("meta")
                    | local_name!("param")
                    | local_name!("source")
                    | local_name!("track")
                    | local_name!("wbr")
            );

        self.stack.push(ElemInfo {
            html_name,
            ignore_children,
        });

        Ok(())
    }

    fn end_elem(&mut self, name: QualName) -> io::Result<()> {
        let info = self.stack.pop().expect("no ElemInfo");
        if info.ignore_children {
            return Ok(());
        }

        self.writer.write_all(b"</")?;
        self.writer.write_all(tagname(&name).as_bytes())?;
        self.writer.write_all(b">")
    }

    fn write_text(&mut self, text: &str) -> io::Result<()> {
        // Raw-text elements are emitted without escaping. `<noscript>` is raw
        // when scripting is enabled (html5ever's default, which we mirror): the
        // parser stores its contents as raw text, so it must be written back
        // verbatim.
        let escape = !matches!(
            self.parent().html_name,
            Some(local_name!("style"))
                | Some(local_name!("script"))
                | Some(local_name!("xmp"))
                | Some(local_name!("iframe"))
                | Some(local_name!("noembed"))
                | Some(local_name!("noframes"))
                | Some(local_name!("plaintext"))
                | Some(local_name!("noscript"))
        );

        if escape {
            self.write_escaped(text, false)
        } else {
            self.writer.write_all(text.as_bytes())
        }
    }

    fn write_comment(&mut self, text: &str) -> io::Result<()> {
        self.writer.write_all(b"<!--")?;
        self.writer.write_all(text.as_bytes())?;
        self.writer.write_all(b"-->")
    }

    fn write_doctype(&mut self, name: &str) -> io::Result<()> {
        self.writer.write_all(b"<!DOCTYPE ")?;
        self.writer.write_all(name.as_bytes())?;
        self.writer.write_all(b">")
    }

    fn write_processing_instruction(&mut self, target: &str, data: &str) -> io::Result<()> {
        self.writer.write_all(b"<?")?;
        self.writer.write_all(target.as_bytes())?;
        self.writer.write_all(b" ")?;
        self.writer.write_all(data.as_bytes())?;
        self.writer.write_all(b">")
    }
}

impl Serialize for NodeRef {
    fn serialize<S: Serializer>(
        &self,
        serializer: &mut S,
        traversal_scope: TraversalScope,
    ) -> io::Result<()> {
        match (traversal_scope, self.data()) {
            (ref scope, NodeData::Element(element)) => {
                if *scope == IncludeNode {
                    let attrs = element.attributes.borrow();

                    // Unfortunately we need to allocate something to hold these &'a QualName
                    let attrs = attrs
                        .map
                        .iter()
                        .map(|(name, attr)| {
                            (
                                QualName::new(
                                    attr.prefix.clone(),
                                    name.ns.clone(),
                                    name.local.clone(),
                                ),
                                &attr.value,
                            )
                        })
                        .collect::<Vec<_>>();

                    serializer.start_elem(
                        element.name.clone(),
                        attrs.iter().map(|&(ref name, value)| (name, &**value)),
                    )?
                }

                let children = match element.template_contents.as_ref() {
                    Some(template_root) => template_root.children(),
                    None => self.children(),
                };

                for child in children {
                    Serialize::serialize(&child, serializer, IncludeNode)?
                }

                if *scope == IncludeNode {
                    serializer.end_elem(element.name.clone())?
                }
                Ok(())
            }

            (_, &NodeData::DocumentFragment) | (_, &NodeData::Document(_)) => {
                for child in self.children() {
                    Serialize::serialize(&child, serializer, IncludeNode)?
                }
                Ok(())
            }

            (ChildrenOnly(_), _) => Ok(()),

            (IncludeNode, NodeData::Doctype(doctype)) => serializer.write_doctype(&doctype.name),
            (IncludeNode, NodeData::Text(text)) => serializer.write_text(&text.borrow()),
            (IncludeNode, NodeData::Comment(text)) => serializer.write_comment(&text.borrow()),
            (IncludeNode, NodeData::ProcessingInstruction(contents)) => {
                let contents = contents.borrow();
                serializer.write_processing_instruction(&contents.0, &contents.1)
            }
        }
    }
}

impl fmt::Display for NodeRef {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // Call the html serializer for the node (sub)tree.
        let mut bytes = Vec::new();
        self.serialize(&mut bytes).or(Err(fmt::Error))?;
        let html = String::from_utf8(bytes).or(Err(fmt::Error))?;
        f.write_str(&html)
    }
}

impl NodeRef {
    /// Serialize this node and its descendants in HTML syntax to the given stream.
    #[inline]
    pub fn serialize<W: Write>(&self, writer: &mut W) -> io::Result<()> {
        let mut ser = HtmlSerializer::new(writer);
        Serialize::serialize(self, &mut ser, IncludeNode)
    }

    /// Serialize this node and its descendants in HTML syntax to a new file at the given path.
    #[inline]
    pub fn serialize_to_file<P: AsRef<Path>>(&self, path: P) -> io::Result<()> {
        let mut file = File::create(&path)?;
        self.serialize(&mut file)
    }
}
