use fastbloom::BloomFilter;
use html5ever::{local_name, namespace_url, ns, LocalName, Namespace, Prefix};
use indexmap::{map::Entry, IndexMap};
use selectors::attr::{CaseSensitivity, SELECTOR_WHITESPACE};

#[derive(Debug, Clone)]
pub(crate) enum ClassCache {
    /// In CSS selector matching, checking an element's class is frequent. Given that classes are
    /// often specific, most elements won't have the checked class. Leveraging this, we use a Bloom
    /// filter for a quick initial check. If positive, we do an actual check. This two-tier
    /// approach ensures fewer actual checks on class attributes.
    Bloom(BloomFilter),
    /// Element has a single class.
    Single,
}

impl ClassCache {
    fn new(value: &str) -> Self {
        if !value.trim().contains(SELECTOR_WHITESPACE) {
            // We just have a single class and a Bloom filter is not needed.
            ClassCache::Single
        } else {
            // Build a Bloom filter for all element's classes
            let classes: Vec<&[u8]> = value
                .split(SELECTOR_WHITESPACE)
                .filter(|s| !s.is_empty())
                .map(|s| s.as_bytes())
                .collect();
            ClassCache::Bloom(BloomFilter::with_num_bits(64).items(classes))
        }
    }
}

/// Convenience wrapper around a indexmap that adds method for attributes in the null namespace.
#[derive(Debug, Clone)]
pub struct Attributes {
    /// A map of attributes whose name can have namespaces.
    pub(crate) map: IndexMap<ExpandedName, Attribute>,
    /// The 'class' attribute value is separated for performance reasons.
    pub(crate) class_cache: Option<ClassCache>,
}

impl Attributes {
    pub(crate) fn new<I>(attributes: I) -> Attributes
    where
        I: IntoIterator<Item = (ExpandedName, Attribute)>,
    {
        let mut class_cache = None;
        let map: IndexMap<ExpandedName, Attribute> = attributes.into_iter().collect();
        if let Some(attr) = map.get(&ExpandedName::new(ns!(), local_name!("class"))) {
            class_cache = Some(ClassCache::new(&attr.value));
        }
        Attributes { map, class_cache }
    }

    /// Manually check whether the class attribute value contains the given class.
    #[inline]
    fn has_class_impl(&self, name: &[u8], case_sensitivity: CaseSensitivity) -> bool {
        let class_list = match self.get(local_name!("class")) {
            Some(class_list) => class_list,
            None => return false,
        };
        for class in class_list.split(SELECTOR_WHITESPACE) {
            if case_sensitivity.eq(class.as_bytes(), name) {
                return true;
            }
        }
        false
    }

    #[inline]
    pub(crate) fn has_class(&self, name: &[u8], case_sensitivity: CaseSensitivity) -> bool {
        match (&self.class_cache, case_sensitivity) {
            (Some(ClassCache::Single), case_sensitivity) => self
                .get(local_name!("class"))
                .is_some_and(|class| case_sensitivity.eq(class.as_bytes(), name)),
            (Some(ClassCache::Bloom(bloom_filter)), CaseSensitivity::CaseSensitive) => {
                if bloom_filter.contains(name) {
                    self.has_class_impl(name, case_sensitivity)
                } else {
                    // Class is not in the Bloom filter, hence this `class` value does not
                    // contain the given class
                    false
                }
            }
            (Some(ClassCache::Bloom(_)), CaseSensitivity::AsciiCaseInsensitive) => {
                self.has_class_impl(name, case_sensitivity)
            }
            (None, case_sensitivity) => self.has_class_impl(name, case_sensitivity),
        }
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
}
