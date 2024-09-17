use kuchikiki::NodeRef;
use lightningcss::rules::style::StyleRule;

/// Cantor's pairing function. Generates a unique integer from a pair of two integers.
pub fn cantor(a: u32, b: u32) -> u32 {
    (a + b + 1) * (a + b) / 2 + b
}

pub trait StyleRuleExt {
    /// Generates a unique identifier that can be used to identify the rule in later passes of the AST.
    fn id(&self) -> u32;
}
impl StyleRuleExt for StyleRule<'_> {
    fn id(&self) -> u32 {
        cantor(
            self.loc.source_index,
            cantor(self.loc.line, self.loc.column),
        )
    }
}

pub trait NodeRefExt {
    /// Creates a new HTML element with the given name and attributes.
    fn new_html_element(name: &str, attributes: Vec<(&str, &str)>) -> NodeRef;
}
impl NodeRefExt for NodeRef {
    fn new_html_element(name: &str, attributes: Vec<(&str, &str)>) -> NodeRef
    {
        use kuchikiki::{Attribute, ExpandedName};
        use markup5ever::{ns, namespace_url, LocalName, QualName};

        NodeRef::new_element(
            QualName::new(None, ns!(html), LocalName::from(name)),
            attributes.into_iter().map(|(n, v)| {
                (
                    ExpandedName::new(ns!(), n),
                    Attribute {
                        prefix: None,
                        value: v.to_string(),
                    },
                )
            }),
        )
    }
}
