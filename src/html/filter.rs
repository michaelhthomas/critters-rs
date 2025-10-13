use html5ever::{local_name, LocalName};
use selectors::{bloom::BloomFilter, Element};
use smallvec::SmallVec;
use string_cache::Atom;

use crate::html::{ElementData, NodeDataRef};

/// A struct that allows us to fast-reject deep descendant selectors avoiding
/// selector-matching.
///
/// This is implemented using a counting bloom filter, and it's a standard
/// optimization. See Gecko's `AncestorFilter`, and Blink's and WebKit's
/// `SelectorFilter`.
pub struct StyleBloom {
    filter: BloomFilter,

    /// The stack of elements that this bloom filter contains, along with the
    /// number of hashes pushed for each element.
    elements: SmallVec<[PushedElement; 16]>,

    /// Stack of hashes that have been pushed onto this filter.
    pushed_hashes: SmallVec<[u32; 64]>,
}

struct PushedElement {
    /// The element that was pushed.
    element: NodeDataRef<ElementData>,

    /// The number of hashes pushed for this element.
    num_hashes: usize,
}

/// Returns whether the attribute name is excluded from the bloom filter.
///
/// We do this for attributes that are very common but not commonly used in
/// selectors.
#[inline]
pub fn is_attr_name_excluded_from_filter(name: &LocalName) -> bool {
    name == &local_name!("class") || name == &local_name!("id") || name == &local_name!("style")
}

/// Gather all relevant hash for fast-reject filters from an element.
pub fn each_relevant_element_hash<F>(element: &NodeDataRef<ElementData>, mut f: F)
where
    F: FnMut(u32),
{
    f(element.name.local.get_hash());
    f(element.name.ns.get_hash());

    let attrs = element.attributes.borrow();

    if let Some(id) = attrs.get(local_name!("id")) {
        f(Atom::<html5ever::LocalNameStaticSet>::from(id).get_hash());
    }

    attrs
        .class_list
        .iter()
        .for_each(|class| f(class.get_hash()));

    attrs.keys().for_each(|name| {
        if !is_attr_name_excluded_from_filter(name) {
            f(name.get_hash())
        }
    });
}

impl Drop for StyleBloom {
    fn drop(&mut self) {
        // Leave the reusable bloom filter in a zeroed state.
        self.clear();
    }
}

impl StyleBloom {
    pub fn new() -> Self {
        StyleBloom {
            filter: BloomFilter::new(),
            elements: Default::default(),
            pushed_hashes: Default::default(),
        }
    }

    /// Return the bloom filter used properly by the `selectors` crate.
    pub fn filter(&self) -> &BloomFilter {
        &self.filter
    }

    /// Get the current depth of ancestor elements in the filter
    pub fn traversal_depth(&self) -> usize {
        self.elements.len()
    }

    /// Push an element to the bloom filter, knowing that it's a child of the
    /// last element parent.
    pub fn push(&mut self, element: NodeDataRef<ElementData>) {
        if cfg!(debug_assertions) && self.elements.is_empty() {
            assert!(element.parent_element().is_none());
        }
        self.push_internal(element);
    }

    /// Same as `push`, but without asserting, in order to use it from
    /// `rebuild`.
    fn push_internal(&mut self, element: NodeDataRef<ElementData>) {
        let mut num_hashes = 0;
        each_relevant_element_hash(&element, |hash| {
            num_hashes += 1;
            self.filter.insert_hash(hash);
            self.pushed_hashes.push(hash);
        });
        self.elements.push(PushedElement {
            element,
            num_hashes,
        });
    }

    /// Pop the last element in the bloom filter and return it.
    #[inline]
    pub fn pop(&mut self) -> Option<NodeDataRef<ElementData>> {
        let PushedElement {
            element,
            num_hashes,
        } = self.elements.pop()?;

        // Verify that the pushed hashes match the ones we'd get from the element.
        let mut expected_hashes = vec![];
        if cfg!(debug_assertions) {
            each_relevant_element_hash(&element, |hash| expected_hashes.push(hash));
        }

        for _ in 0..num_hashes {
            let hash = self.pushed_hashes.pop().unwrap();
            debug_assert_eq!(expected_hashes.pop().unwrap(), hash);
            self.filter.remove_hash(hash);
        }

        Some(element)
    }

    /// Clears the bloom filter.
    pub fn clear(&mut self) {
        self.elements.clear();
        self.filter.clear();
        self.pushed_hashes.clear();
    }

    /// Rebuilds the bloom filter up to the parent of the given element.
    pub fn rebuild(&mut self, mut element: NodeDataRef<ElementData>) {
        self.clear();

        let mut parents_to_insert = SmallVec::<[NodeDataRef<ElementData>; 16]>::new();
        while let Some(parent) = element.parent_element().clone() {
            parents_to_insert.push(parent.clone());
            element = parent;
        }

        for parent in parents_to_insert.drain(..).rev() {
            self.push(parent);
        }
    }

    /// In debug builds, asserts that all the parents of `element` are in the
    /// bloom filter.
    ///
    /// Goes away in release builds.
    pub fn assert_complete(&self, mut element: NodeDataRef<ElementData>) {
        if cfg!(debug_assertions) {
            let mut checked = 0;
            while let Some(parent) = element.parent_element().clone() {
                assert_eq!(
                    parent,
                    self.elements[self.elements.len() - 1 - checked].element
                );
                element = parent;
                checked += 1;
            }
            assert_eq!(checked, self.elements.len());
        }
    }
}
