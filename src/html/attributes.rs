use html5ever::{local_name, namespace_url, ns, LocalName, Namespace, Prefix};
use indexmap::{map::Entry, IndexMap};
use selectors::attr::{CaseSensitivity, SELECTOR_WHITESPACE};

/// Convenience wrapper around a indexmap that adds method for attributes in the null namespace.
#[derive(Debug, Clone)]
pub struct Attributes {
    /// The list of CSS classes for the element
    pub class_list: Vec<LocalName>,
    /// A map of attributes whose name can have namespaces.
    pub(crate) map: IndexMap<ExpandedName, Attribute>,
}

impl Attributes {
    pub(crate) fn new<I>(attributes: I) -> Attributes
    where
        I: IntoIterator<Item = (ExpandedName, Attribute)>,
    {
        let map: IndexMap<ExpandedName, Attribute> = attributes.into_iter().collect();
        let class_list = map
            .get(&ExpandedName {
                ns: ns!(),
                local: local_name!("class"),
            })
            .map(|a| {
                a.value
                    .split(SELECTOR_WHITESPACE)
                    .map(LocalName::from)
                    .collect()
            })
            .unwrap_or_else(|| Vec::new());

        Attributes { map, class_list }
    }

    #[inline]
    pub(crate) fn has_class(&self, name: &[u8], case_sensitivity: CaseSensitivity) -> bool {
        for class in &self.class_list {
            if case_sensitivity.eq(class.as_bytes(), name) {
                return true;
            }
        }
        false
    }
}
impl PartialEq for Attributes {
    fn eq(&self, other: &Self) -> bool {
        self.map == other.map
    }
}

/// <https://www.w3.org/TR/REC-xml-names/#dt-expname>
#[derive(Debug, PartialEq, Eq, Hash, Clone, PartialOrd, Ord)]
pub struct ExpandedName {
    /// Namespace URL
    pub ns: Namespace,
    /// "Local" part of the name
    pub local: LocalName,
}

impl ExpandedName {
    /// Trivial constructor
    pub fn new<N: Into<Namespace>, L: Into<LocalName>>(ns: N, local: L) -> Self {
        ExpandedName {
            ns: ns.into(),
            local: local.into(),
        }
    }
}

/// The non-identifying parts of an attribute
#[derive(Debug, PartialEq, Clone)]
pub struct Attribute {
    /// The namespace prefix, if any
    pub prefix: Option<Prefix>,
    /// The attribute value
    pub value: String,
}

impl Attributes {
    /// Like IndexMap::contains
    pub fn contains<A: Into<LocalName>>(&self, local_name: A) -> bool {
        self.map.contains_key(&ExpandedName::new(ns!(), local_name))
    }

    /// Like IndexMap::get
    pub fn get<A: Into<LocalName>>(&self, local_name: A) -> Option<&str> {
        self.map
            .get(&ExpandedName::new(ns!(), local_name))
            .map(|attr| &*attr.value)
    }

    /// Like IndexMap::get_mut
    pub fn get_mut<A: Into<LocalName>>(&mut self, local_name: A) -> Option<&mut String> {
        self.map
            .get_mut(&ExpandedName::new(ns!(), local_name))
            .map(|attr| &mut attr.value)
    }

    /// Like IndexMap::entry
    pub fn entry<A: Into<LocalName>>(
        &mut self,
        local_name: A,
    ) -> Entry<'_, ExpandedName, Attribute> {
        self.map.entry(ExpandedName::new(ns!(), local_name))
    }

    /// Like IndexMap::insert
    pub fn insert<A: Into<LocalName>>(
        &mut self,
        local_name: A,
        value: String,
    ) -> Option<Attribute> {
        self.map.insert(
            ExpandedName::new(ns!(), local_name),
            Attribute {
                prefix: None,
                value,
            },
        )
    }

    /// Like IndexMap::remove
    pub fn remove<A: Into<LocalName>>(&mut self, local_name: A) -> Option<Attribute> {
        self.map.swap_remove(&ExpandedName::new(ns!(), local_name))
    }

    /// Like IndexMap::keys
    pub fn keys(&self) -> impl Iterator<Item = &LocalName> {
        self.map.keys().map(|expanded_name| &expanded_name.local)
    }
}
