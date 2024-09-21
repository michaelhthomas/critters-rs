use kuchikiki::NodeRef;
use lightningcss::rules::style::StyleRule;

/// Implementation of the Szudzik pairing function.
///
/// A pairing function is a bijection from N x N -> N. Szudizik's algorithm
/// has the most efficient possible value packing, with the maximum pair size
/// being (sqrt(MAX_INTEGER), sqrt(MAX_INTEGER)).
///
/// [Reference](http://www.szudzik.com/ElegantPairing.pdf)
pub fn szudzik_pair(x: impl Into<u64>, y: impl Into<u64>) -> u64 {
    let x: u64 = x.into();
    let y: u64 = y.into();

    if x >= y {
        (x * x) + x + y
    } else {
        (y * y) + x
    }
}

pub trait StyleRuleExt {
    /// Generates a unique identifier that can be used to identify the rule in later passes of the AST.
    fn id(&self) -> u64;
}
impl StyleRuleExt for StyleRule<'_> {
    fn id(&self) -> u64 {
        szudzik_pair(
            self.loc.source_index,
            szudzik_pair(self.loc.line, self.loc.column),
        )
    }
}

pub trait NodeRefExt {
    /// Creates a new HTML element with the given name and attributes.
    fn new_html_element(name: &str, attributes: Vec<(&str, &str)>) -> NodeRef;
}
impl NodeRefExt for NodeRef {
    fn new_html_element(name: &str, attributes: Vec<(&str, &str)>) -> NodeRef {
        use kuchikiki::{Attribute, ExpandedName};
        use markup5ever::{namespace_url, ns, LocalName, QualName};

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
