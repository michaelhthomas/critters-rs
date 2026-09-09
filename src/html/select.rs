use crate::html::attributes::ExpandedName;
use crate::html::iter::{NodeIterator, Select};
use crate::html::node_data_ref::NodeDataRef;
use crate::html::tree::{ElementData, Node, NodeData, NodeRef};
use html5ever::{local_name, namespace_url, ns, LocalName, Namespace};
use lightningcss::selector::{Component, PseudoClass, PseudoElement};
use lightningcss::traits::ParseWithOptions;
use lightningcss::values::ident::Ident;
use lightningcss::values::string::{CSSString, CowArcStr};
use parcel_selectors::attr::{AttrSelectorOperation, CaseSensitivity, NamespaceConstraint};
use parcel_selectors::bloom::BLOOM_HASH_MASK;
use parcel_selectors::context::QuirksMode;
use parcel_selectors::matching::{self, MatchingContext, MatchingMode};
use parcel_selectors::parser::{AncestorHashes, Combinator, NthType, SelectorImpl, SelectorIter};
use parcel_selectors::{Element, OpaqueElement};
use std::fmt;

/// Recovers the [`SelectorImpl`] that `lightningcss` parses its selectors with.
///
/// `lightningcss` declares that type as `mod private { pub struct Selectors; }`, so it cannot be
/// named from outside the crate even though it leaks through the public `Selector`, `SelectorList`
/// and `Component` aliases. Projecting it back out through a trait of our own is the only way to
/// write `impl Element for _` against it, which in turn is what lets selectors travel between the
/// CSS and HTML backends without a serialize/reparse round trip.
pub trait ImplOf<'i> {
    /// The `SelectorImpl` the selector was parsed with.
    type Impl: SelectorImpl<'i>;
}

impl<'i, I: SelectorImpl<'i>> ImplOf<'i> for parcel_selectors::parser::Selector<'i, I> {
    type Impl = I;
}

/// The `SelectorImpl` shared by both backends.
pub type LightningCss<'i> = <lightningcss::selector::Selector<'i> as ImplOf<'i>>::Impl;

/// A parsed CSS selector, borrowed from the stylesheet it was parsed from.
pub type Selector<'i> = lightningcss::selector::Selector<'i>;

/// A parsed list of comma-separated CSS selectors.
pub type Selectors<'i> = lightningcss::selector::SelectorList<'i>;

impl<'i> Element<'i> for NodeDataRef<ElementData> {
    type Impl = LightningCss<'i>;

    #[inline]
    fn opaque(&self) -> OpaqueElement {
        let node: &Node = self.as_node();
        OpaqueElement::new(node)
    }

    #[inline]
    fn is_html_slot_element(&self) -> bool {
        false
    }
    #[inline]
    fn parent_node_is_shadow_root(&self) -> bool {
        false
    }
    #[inline]
    fn containing_shadow_host(&self) -> Option<Self> {
        None
    }

    #[inline]
    fn parent_element(&self) -> Option<Self> {
        self.as_node().parent().and_then(NodeRef::into_element_ref)
    }
    #[inline]
    fn prev_sibling_element(&self) -> Option<Self> {
        self.as_node().preceding_siblings().elements().next()
    }
    #[inline]
    fn next_sibling_element(&self) -> Option<Self> {
        self.as_node().following_siblings().elements().next()
    }
    #[inline]
    fn is_empty(&self) -> bool {
        self.as_node().children().all(|child| match *child.data() {
            NodeData::Element(_) => false,
            NodeData::Text(ref text) => text.borrow().is_empty(),
            _ => true,
        })
    }
    #[inline]
    fn is_root(&self) -> bool {
        match self.as_node().parent() {
            None => false,
            Some(parent) => matches!(*parent.data(), NodeData::Document(_)),
        }
    }

    #[inline]
    fn is_html_element_in_html_document(&self) -> bool {
        // FIXME: Have a notion of HTML document v.s. XML document?
        self.name.ns == ns!(html)
    }

    #[inline]
    fn has_local_name(&self, name: &Ident<'i>) -> bool {
        *self.name.local == *name.0
    }
    #[inline]
    fn has_namespace(&self, namespace: &CowArcStr<'i>) -> bool {
        *self.name.ns == **namespace
    }

    #[inline]
    fn is_part(&self, _name: &Ident<'i>) -> bool {
        false
    }

    #[inline]
    fn imported_part(&self, _: &Ident<'i>) -> Option<Ident<'i>> {
        None
    }

    #[inline]
    fn is_pseudo_element(&self) -> bool {
        false
    }

    #[inline]
    fn is_same_type(&self, other: &Self) -> bool {
        self.name == other.name
    }

    #[inline]
    fn is_link(&self) -> bool {
        self.name.ns == ns!(html)
            && matches!(
                self.name.local,
                local_name!("a") | local_name!("area") | local_name!("link")
            )
            && self
                .attributes
                .borrow()
                .map
                .contains_key(&ExpandedName::new(ns!(), local_name!("href")))
    }

    #[inline]
    fn has_id(&self, id: &Ident<'i>, case_sensitivity: CaseSensitivity) -> bool {
        self.attributes
            .borrow()
            .get(local_name!("id"))
            .is_some_and(|id_attr| case_sensitivity.eq(id.0.as_bytes(), id_attr.as_bytes()))
    }

    #[inline]
    fn has_class(&self, name: &Ident<'i>, case_sensitivity: CaseSensitivity) -> bool {
        let name = name.0.as_bytes();
        !name.is_empty() && self.attributes.borrow().has_class(name, case_sensitivity)
    }

    #[inline]
    fn attr_matches(
        &self,
        ns: &NamespaceConstraint<&CowArcStr<'i>>,
        local_name: &Ident<'i>,
        operation: &AttrSelectorOperation<&CSSString<'i>>,
    ) -> bool {
        // Scanning beats a map lookup here: the selector's names are plain strings, so keying
        // into the map would mean interning an atom for every attribute selector we test, and
        // elements carry few enough attributes for the scan to win.
        let attrs = self.attributes.borrow();
        attrs.map.iter().any(|(name, attr)| {
            *name.local == *local_name.0
                && match *ns {
                    NamespaceConstraint::Any => true,
                    NamespaceConstraint::Specific(ns_url) => *name.ns == **ns_url,
                }
                && operation.eval_str(&attr.value)
        })
    }

    /// Pseudo-elements are transparent: whether the rule is reachable is decided by the originating
    /// element, so `.foo::before` is kept exactly when `.foo` is present.
    fn match_pseudo_element(
        &self,
        _pseudo: &PseudoElement<'i>,
        _context: &mut MatchingContext<'_, 'i, LightningCss<'i>>,
    ) -> bool {
        true
    }

    /// `parcel_selectors` splits a compound selector at its pseudo-element with a synthetic
    /// [`Combinator::PseudoElement`], then asks for the element the pseudo-element originates
    /// from before matching the rest. Since we match pseudo-elements transparently against real
    /// elements rather than against a wrapper standing in for the pseudo-element, the originating
    /// element is this element itself. The default implementation would return the parent, which
    /// would have `.foo::before` look for `.foo` one level too high.
    ///
    /// [`Combinator::PseudoElement`]: parcel_selectors::parser::Combinator::PseudoElement
    #[inline]
    fn pseudo_element_originating_element(&self) -> Option<Self> {
        Some(self.clone())
    }

    /// Non-tree-structural pseudo-classes describe state that a static document cannot tell us
    /// about, so they match. Dropping the rule instead would strip styles the page needs the
    /// moment the user interacts with it, whereas keeping it only costs bytes.
    ///
    /// The link pseudo-classes are the exception, since being a link *is* statically observable.
    fn match_non_ts_pseudo_class<F>(
        &self,
        pseudo: &PseudoClass<'i>,
        _context: &mut MatchingContext<'_, 'i, LightningCss<'i>>,
        _flags_setter: &mut F,
    ) -> bool
    where
        F: FnMut(&Self, matching::ElementSelectorFlags),
    {
        use PseudoClass::*;
        match pseudo {
            AnyLink(_) | Link | LocalLink | Visited => self.is_link(),
            _ => true,
        }
    }
}

/// Extension methods for a compiled selector.
pub trait SelectorExt<'i> {
    /// Returns whether the given element matches this selector.
    fn matches(&self, element: &NodeDataRef<ElementData>) -> bool;

    /// Returns whether the given element matches this selector, reusing the given matching context.
    /// Passing `hashes` lets the context's bloom filter fast-reject the selector.
    fn matches_with_context(
        &self,
        element: &NodeDataRef<ElementData>,
        hashes: Option<&AncestorHashes>,
        context: &mut MatchingContext<'_, 'i, LightningCss<'i>>,
    ) -> bool;

    /// Computes the ancestor hashes used to fast-reject this selector against a bloom filter of
    /// the ancestors of the element being matched.
    fn ancestor_hashes(&self) -> AncestorHashes;

    /// Returns whether this selector can be handed to the matching engine at all.
    ///
    /// `parcel_selectors` panics rather than returning `false` for the few component types it has
    /// not implemented, so callers must screen selectors with this first and decide for themselves
    /// what an unmatchable selector means.
    fn is_matchable(&self) -> bool;
}

impl<'i> SelectorExt<'i> for Selector<'i> {
    #[inline]
    fn matches(&self, element: &NodeDataRef<ElementData>) -> bool {
        let mut context =
            MatchingContext::new(MatchingMode::Normal, None, None, QuirksMode::NoQuirks);
        self.matches_with_context(element, None, &mut context)
    }

    #[inline]
    fn matches_with_context(
        &self,
        element: &NodeDataRef<ElementData>,
        hashes: Option<&AncestorHashes>,
        context: &mut MatchingContext<'_, 'i, LightningCss<'i>>,
    ) -> bool {
        matching::matches_selector(self, 0, hashes, element, context, &mut |_, _| {})
    }

    fn ancestor_hashes(&self) -> AncestorHashes {
        let mut hashes = [0u32; 4];
        let mut len = 0;
        collect_ancestor_hashes(self.iter(), &mut hashes, &mut len);
        debug_assert!(len <= 4);

        // Pack the fourth hash, if there is one, into the upper byte of each of the other three.
        if len == 4 {
            let fourth = hashes[3];
            hashes[0] |= (fourth & 0x0000_00ff) << 24;
            hashes[1] |= (fourth & 0x0000_ff00) << 16;
            hashes[2] |= (fourth & 0x00ff_0000) << 8;
        }

        AncestorHashes {
            packed_hashes: [hashes[0], hashes[1], hashes[2]],
        }
    }

    fn is_matchable(&self) -> bool {
        self.iter_raw_match_order().all(is_component_matchable)
    }
}

fn is_component_matchable(component: &Component<'_>) -> bool {
    match component {
        // The matching engine reaches an `unreachable!()`/`todo!()` for these rather than
        // returning false, so they must never get there.
        Component::Has(_) | Component::Nesting | Component::NthOf(_) => false,
        Component::Nth(nth) => !matches!(nth.ty, NthType::Col | NthType::LastCol),
        // Nested selector lists have to be screened too, since a `:has()` inside an `:is()` panics
        // just the same. `Component::visit` skips `Has` and `Any` sublists, hence the manual walk.
        Component::Negation(list)
        | Component::Is(list)
        | Component::Where(list)
        | Component::Any(_, list) => list.iter().all(|selector| selector.is_matchable()),
        Component::Slotted(selector) => selector.is_matchable(),
        Component::Host(selector) => selector.as_ref().is_none_or(|s| s.is_matchable()),
        _ => true,
    }
}

/// Hashes a selector component the same way [`crate::html::filter`] hashes the corresponding part
/// of an element: by interning it into the matching `html5ever` atom set and reading the atom's
/// precomputed hash. Both sides have to agree, or the bloom filter would reject valid matches.
#[inline]
fn local_name_hash(name: &str) -> u32 {
    LocalName::from(name).get_hash()
}

#[inline]
fn namespace_hash(url: &str) -> u32 {
    Namespace::from(url).get_hash()
}

/// Port of `parcel_selectors`' private `AncestorIter`. Yields the components of every ancestor
/// compound selector, skipping the rightmost one and any sequence joined by a sibling combinator.
struct AncestorIter<'a, 'i>(SelectorIter<'a, 'i, LightningCss<'i>>);

impl<'a, 'i> AncestorIter<'a, 'i> {
    fn new(inner: SelectorIter<'a, 'i, LightningCss<'i>>) -> Self {
        let mut iter = AncestorIter(inner);
        iter.skip_until_ancestor();
        iter
    }

    /// Skips a sequence of simple selectors and all subsequent sequences until an ancestor
    /// combinator is reached.
    fn skip_until_ancestor(&mut self) {
        loop {
            while self.0.next().is_some() {}
            if self
                .0
                .next_sequence()
                .is_none_or(|c| matches!(c, Combinator::Child | Combinator::Descendant))
            {
                break;
            }
        }
    }
}

impl<'a, 'i> Iterator for AncestorIter<'a, 'i> {
    type Item = &'a Component<'i>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(next) = self.0.next() {
            return Some(next);
        }

        if let Some(combinator) = self.0.next_sequence() {
            if !matches!(combinator, Combinator::Child | Combinator::Descendant) {
                self.skip_until_ancestor();
            }
        }

        self.0.next()
    }
}

/// Port of `parcel_selectors`' private `collect_ancestor_hashes`. We cannot call the crate's own
/// `AncestorHashes::new`, because it requires `PrecomputedHash` on the `SelectorImpl`'s string
/// types and `lightningcss`' `Ident`/`CowArcStr` neither implement it nor can we implement it for
/// them.
fn collect_ancestor_hashes(
    iter: SelectorIter<'_, '_, LightningCss<'_>>,
    hashes: &mut [u32; 4],
    len: &mut usize,
) -> bool {
    for component in AncestorIter::new(iter) {
        let hash = match component {
            Component::LocalName(name) => {
                // Only insert the local name if it is all lowercase, otherwise we would have to
                // test both hashes and our data structures are not set up for that.
                if name.name != name.lower_name {
                    continue;
                }
                local_name_hash(&name.name.0)
            }
            Component::DefaultNamespace(url) | Component::Namespace(_, url) => namespace_hash(url),
            Component::ID(id) => local_name_hash(&id.0),
            Component::Class(class) => local_name_hash(&class.0),
            Component::Is(list) | Component::Where(list) => {
                // `:is` and `:where` OR their selectors together, so nothing can go into the
                // filter unless there is exactly one, as that would exclude elements matching one
                // of the others.
                if list.len() == 1 && !collect_ancestor_hashes(list[0].iter(), hashes, len) {
                    return false;
                }
                continue;
            }
            _ => continue,
        };

        hashes[*len] = hash & BLOOM_HASH_MASK;
        *len += 1;
        if *len == hashes.len() {
            return false;
        }
    }

    true
}

/// Extension methods for a compiled selector list.
pub trait SelectorsExt<'i>: Sized {
    /// Compile a list of selectors. This may fail on syntax errors.
    fn compile(s: &'i str) -> Result<Self, SelectorParseError>;

    /// Returns whether the given element matches any selector in this list.
    fn matches(&self, element: &NodeDataRef<ElementData>) -> bool;

    /// Filter an element iterator, yielding those matching this list of selectors.
    fn filter<I>(self, iter: I) -> Select<'i, I>
    where
        I: Iterator<Item = NodeDataRef<ElementData>>;
}

impl<'i> SelectorsExt<'i> for Selectors<'i> {
    #[inline]
    fn compile(s: &'i str) -> Result<Self, SelectorParseError> {
        Selectors::parse_string_with_options(s, Default::default())
            .map_err(|err| SelectorParseError(format!("{err:?}")))
    }

    #[inline]
    fn matches(&self, element: &NodeDataRef<ElementData>) -> bool {
        let mut context =
            MatchingContext::new(MatchingMode::Normal, None, None, QuirksMode::NoQuirks);
        matching::matches_selector_list(self, element, &mut context)
    }

    #[inline]
    fn filter<I>(self, iter: I) -> Select<'i, I>
    where
        I: Iterator<Item = NodeDataRef<ElementData>>,
    {
        Select {
            iter,
            selectors: self,
        }
    }
}

/// The error returned when a selector list fails to parse.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelectorParseError(String);

impl fmt::Display for SelectorParseError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for SelectorParseError {}
