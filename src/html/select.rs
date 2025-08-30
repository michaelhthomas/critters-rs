use crate::html::attributes::ExpandedName;
use crate::html::iter::{NodeIterator, Select};
use crate::html::node_data_ref::NodeDataRef;
use crate::html::tree::{ElementData, Node, NodeData, NodeRef};
use cssparser::{self, CowRcStr, ParseError, SourceLocation, ToCss};
use html5ever::{local_name, namespace_url, ns, LocalName, Namespace};
use selectors::attr::{AttrSelectorOperation, CaseSensitivity, NamespaceConstraint};
use selectors::context::QuirksMode;
use selectors::parser::{
    AncestorHashes, NonTSPseudoClass, Parser, Selector as GenericSelector, SelectorImpl,
    SelectorList,
};
use selectors::parser::{SelectorIter, SelectorParseErrorKind};
use selectors::{self, matching, OpaqueElement};
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KuchikiSelectors;

impl SelectorImpl for KuchikiSelectors {
    type AttrValue = String;
    type Identifier = LocalName;
    type ClassName = LocalName;
    type LocalName = LocalName;
    type PartName = LocalName;
    type NamespacePrefix = LocalName;
    type NamespaceUrl = Namespace;
    type BorrowedNamespaceUrl = Namespace;
    type BorrowedLocalName = LocalName;

    type NonTSPseudoClass = PseudoClass;
    type PseudoElement = PseudoElement;

    type ExtraMatchingData = ();
}

struct KuchikiParser;

impl<'i> Parser<'i> for KuchikiParser {
    type Impl = KuchikiSelectors;
    type Error = SelectorParseErrorKind<'i>;

    fn parse_non_ts_pseudo_class(
        &self,
        location: SourceLocation,
        name: CowRcStr<'i>,
    ) -> Result<PseudoClass, ParseError<'i, SelectorParseErrorKind<'i>>> {
        use self::PseudoClass::*;
        if name.eq_ignore_ascii_case("any-link") {
            Ok(AnyLink)
        } else if name.eq_ignore_ascii_case("link") {
            Ok(Link)
        } else if name.eq_ignore_ascii_case("visited") {
            Ok(Visited)
        } else if name.eq_ignore_ascii_case("active") {
            Ok(Active)
        } else if name.eq_ignore_ascii_case("focus") {
            Ok(Focus)
        } else if name.eq_ignore_ascii_case("hover") {
            Ok(Hover)
        } else if name.eq_ignore_ascii_case("enabled") {
            Ok(Enabled)
        } else if name.eq_ignore_ascii_case("disabled") {
            Ok(Disabled)
        } else if name.eq_ignore_ascii_case("checked") {
            Ok(Checked)
        } else if name.eq_ignore_ascii_case("indeterminate") {
            Ok(Indeterminate)
        } else {
            Err(
                location.new_custom_error(SelectorParseErrorKind::UnsupportedPseudoClassOrElement(
                    name,
                )),
            )
        }
    }
}

#[derive(PartialEq, Eq, Clone, Debug, Hash)]
pub enum PseudoClass {
    AnyLink,
    Link,
    Visited,
    Active,
    Focus,
    Hover,
    Enabled,
    Disabled,
    Checked,
    Indeterminate,
}

impl NonTSPseudoClass for PseudoClass {
    type Impl = KuchikiSelectors;

    fn is_active_or_hover(&self) -> bool {
        matches!(*self, PseudoClass::Active | PseudoClass::Hover)
    }

    fn is_user_action_state(&self) -> bool {
        matches!(
            *self,
            PseudoClass::Active | PseudoClass::Hover | PseudoClass::Focus
        )
    }

    fn has_zero_specificity(&self) -> bool {
        false
    }
}

impl ToCss for PseudoClass {
    fn to_css<W>(&self, dest: &mut W) -> fmt::Result
    where
        W: fmt::Write,
    {
        dest.write_str(match *self {
            PseudoClass::AnyLink => ":any-link",
            PseudoClass::Link => ":link",
            PseudoClass::Visited => ":visited",
            PseudoClass::Active => ":active",
            PseudoClass::Focus => ":focus",
            PseudoClass::Hover => ":hover",
            PseudoClass::Enabled => ":enabled",
            PseudoClass::Disabled => ":disabled",
            PseudoClass::Checked => ":checked",
            PseudoClass::Indeterminate => ":indeterminate",
        })
    }
}

#[derive(PartialEq, Eq, Clone, Debug, Hash)]
pub enum PseudoElement {}

impl ToCss for PseudoElement {
    fn to_css<W>(&self, _dest: &mut W) -> fmt::Result
    where
        W: fmt::Write,
    {
        match *self {}
    }
}

impl selectors::parser::PseudoElement for PseudoElement {
    type Impl = KuchikiSelectors;
}

impl selectors::Element for NodeDataRef<ElementData> {
    type Impl = KuchikiSelectors;

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
    fn has_local_name(&self, name: &LocalName) -> bool {
        self.name.local == *name
    }
    #[inline]
    fn has_namespace(&self, namespace: &Namespace) -> bool {
        self.name.ns == *namespace
    }

    #[inline]
    fn is_part(&self, _name: &LocalName) -> bool {
        false
    }

    #[inline]
    fn exported_part(&self, _: &LocalName) -> Option<LocalName> {
        None
    }

    #[inline]
    fn imported_part(&self, _: &LocalName) -> Option<LocalName> {
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
    fn has_id(&self, id: &LocalName, case_sensitivity: CaseSensitivity) -> bool {
        self.attributes
            .borrow()
            .get(local_name!("id"))
            .is_some_and(|id_attr| case_sensitivity.eq(id.as_bytes(), id_attr.as_bytes()))
    }

    #[inline]
    fn has_class(&self, name: &LocalName, case_sensitivity: CaseSensitivity) -> bool {
        let name = name.as_bytes();
        !name.is_empty() && self.attributes.borrow().has_class(name, case_sensitivity)
    }

    #[inline]
    fn attr_matches(
        &self,
        ns: &NamespaceConstraint<&Namespace>,
        local_name: &LocalName,
        operation: &AttrSelectorOperation<&String>,
    ) -> bool {
        let attrs = self.attributes.borrow();
        match *ns {
            NamespaceConstraint::Any => attrs
                .map
                .iter()
                .any(|(name, attr)| name.local == *local_name && operation.eval_str(&attr.value)),
            NamespaceConstraint::Specific(ns_url) => attrs
                .map
                .get(&ExpandedName::new(ns_url, local_name.clone()))
                .is_some_and(|attr| operation.eval_str(&attr.value)),
        }
    }

    fn match_pseudo_element(
        &self,
        pseudo: &PseudoElement,
        _context: &mut matching::MatchingContext<KuchikiSelectors>,
    ) -> bool {
        match *pseudo {}
    }

    fn match_non_ts_pseudo_class<F>(
        &self,
        pseudo: &PseudoClass,
        _context: &mut matching::MatchingContext<KuchikiSelectors>,
        _flags_setter: &mut F,
    ) -> bool
    where
        F: FnMut(&Self, matching::ElementSelectorFlags),
    {
        use self::PseudoClass::*;
        match *pseudo {
            Active | Focus | Hover | Enabled | Disabled | Checked | Indeterminate | Visited => {
                false
            }
            AnyLink | Link => {
                self.name.ns == ns!(html)
                    && matches!(
                        self.name.local,
                        local_name!("a") | local_name!("area") | local_name!("link")
                    )
                    && self.attributes.borrow().contains(local_name!("href"))
            }
        }
    }
}

/// A pre-compiled list of CSS Selectors.
pub struct Selectors(pub Vec<Selector>);

/// A pre-compiled CSS Selector.
#[derive(Clone, PartialEq, Eq)]
pub struct Selector(GenericSelector<KuchikiSelectors>);

impl Selector {
    /// Returns an iterator over this selector in matching order (right-to-left).
    /// When a combinator is reached, the iterator will return None, and
    /// next_sequence() may be called to continue to the next sequence.
    pub fn iter(&self) -> SelectorIter<'_, KuchikiSelectors> {
        self.0.iter()
    }

    /// Returns whether the given element matches this selector.
    #[inline]
    pub fn matches(&self, element: &NodeDataRef<ElementData>) -> bool {
        let mut context = matching::MatchingContext::new(
            matching::MatchingMode::Normal,
            None,
            None,
            QuirksMode::NoQuirks,
        );
        matching::matches_selector(&self.0, 0, None, element, &mut context, &mut |_, _| {})
    }

    /// Returns whether the given element matches this selector using the given matching context.
    #[inline]
    pub fn matches_with_context(
        &self,
        element: &NodeDataRef<ElementData>,
        hashes: Option<&AncestorHashes>,
        context: &mut matching::MatchingContext<KuchikiSelectors>,
    ) -> bool {
        matching::matches_selector(&self.0, 0, hashes, element, context, &mut |_, _| {})
    }

    /// Return the specificity of this selector.
    pub fn specificity(&self) -> Specificity {
        Specificity(self.0.specificity())
    }

    pub(crate) fn ancestor_hashes(&self) -> AncestorHashes {
        AncestorHashes::new(&self.0, QuirksMode::NoQuirks)
    }
}

/// Implements hashing for CSS selectors. Unfortunately, the `selectors` crate does not provide a
/// direct way to hash a selector, so we have to do some manual labor.
mod hashing {
    use crate::html::select::KuchikiSelectors;
    use selectors::attr::NamespaceConstraint;
    use std::hash::Hash;

    fn hash_component<H: std::hash::Hasher>(
        c: &selectors::parser::Component<KuchikiSelectors>,
        state: &mut H,
    ) {
        std::mem::discriminant(c).hash(state);
        match c {
            selectors::parser::Component::Combinator(combinator) => {
                std::mem::discriminant(combinator).hash(state);
            }
            selectors::parser::Component::DefaultNamespace(ns) => ns.hash(state),
            selectors::parser::Component::Namespace(pre, url) => {
                pre.hash(state);
                url.hash(state);
            }
            selectors::parser::Component::LocalName(local_name) => {
                local_name.name.hash(state);
                local_name.lower_name.hash(state);
            }
            selectors::parser::Component::ID(id) => id.hash(state),
            selectors::parser::Component::Class(class) => class.hash(state),
            selectors::parser::Component::AttributeInNoNamespaceExists {
                local_name,
                local_name_lower,
            } => {
                local_name.hash(state);
                local_name_lower.hash(state);
            }
            selectors::parser::Component::AttributeInNoNamespace {
                local_name,
                operator,
                value,
                case_sensitivity,
                never_matches,
            } => {
                local_name.hash(state);
                std::mem::discriminant(operator).hash(state);
                value.hash(state);
                std::mem::discriminant(case_sensitivity).hash(state);
                never_matches.hash(state);
            }
            selectors::parser::Component::AttributeOther(attr_selector_with_optional_namespace) => {
                if let Some(namespace) = &attr_selector_with_optional_namespace.namespace {
                    std::mem::discriminant(namespace).hash(state);
                    match &namespace {
                        NamespaceConstraint::Specific(url) => url.hash(state),
                        NamespaceConstraint::Any => {}
                    }
                }
                attr_selector_with_optional_namespace.local_name.hash(state);
                attr_selector_with_optional_namespace
                    .local_name_lower
                    .hash(state);
                std::mem::discriminant(&attr_selector_with_optional_namespace.operation)
                    .hash(state);
                match &attr_selector_with_optional_namespace.operation {
                    selectors::attr::ParsedAttrSelectorOperation::WithValue {
                        operator,
                        case_sensitivity,
                        expected_value,
                    } => {
                        std::mem::discriminant(operator).hash(state);
                        std::mem::discriminant(case_sensitivity).hash(state);
                        expected_value.hash(state);
                    }
                    selectors::attr::ParsedAttrSelectorOperation::Exists => {}
                }
                attr_selector_with_optional_namespace
                    .never_matches
                    .hash(state);
            }
            selectors::parser::Component::Negation(thin_boxed_slice) => {
                thin_boxed_slice
                    .iter()
                    .for_each(|c| hash_component(c, state));
            }
            selectors::parser::Component::NthChild(i, j) => {
                i.hash(state);
                j.hash(state);
            }
            selectors::parser::Component::NthLastChild(i, j) => {
                i.hash(state);
                j.hash(state);
            }
            selectors::parser::Component::NthOfType(i, j) => {
                i.hash(state);
                j.hash(state);
            }
            selectors::parser::Component::NthLastOfType(i, j) => {
                i.hash(state);
                j.hash(state);
            }
            selectors::parser::Component::NonTSPseudoClass(class) => class.hash(state),
            selectors::parser::Component::Slotted(selector) => hash_selector(selector, state),
            selectors::parser::Component::Part(items) => {
                items.iter().for_each(|item| item.hash(state))
            }
            selectors::parser::Component::Host(selector) => {
                if let Some(selector) = selector {
                    hash_selector(selector, state)
                }
            }
            selectors::parser::Component::PseudoElement(el) => el.hash(state),
            selectors::parser::Component::ExplicitAnyNamespace => {}
            selectors::parser::Component::ExplicitNoNamespace => {}
            selectors::parser::Component::ExplicitUniversalType => {}
            selectors::parser::Component::FirstChild => {}
            selectors::parser::Component::LastChild => {}
            selectors::parser::Component::OnlyChild => {}
            selectors::parser::Component::Root => {}
            selectors::parser::Component::Empty => {}
            selectors::parser::Component::Scope => {}
            selectors::parser::Component::FirstOfType => {}
            selectors::parser::Component::LastOfType => {}
            selectors::parser::Component::OnlyOfType => {}
        }
    }

    pub fn hash_selector<H: std::hash::Hasher>(
        selector: &selectors::parser::Selector<KuchikiSelectors>,
        state: &mut H,
    ) {
        selector.iter().for_each(|c| hash_component(c, state));
    }
}
impl std::hash::Hash for Selector {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        hashing::hash_selector(&self.0, state);
    }
}

/// The specificity of a selector.
///
/// Opaque, but ordered.
///
/// Determines precedence in the cascading algorithm.
/// When equal, a rule later in source order takes precedence.
#[derive(Copy, Clone, Hash, Eq, PartialEq, Ord, PartialOrd)]
pub struct Specificity(u32);

impl Selectors {
    /// Compile a list of selectors. This may fail on syntax errors or unsupported selectors.
    #[inline]
    pub fn compile(s: &str) -> Result<Selectors, ParseError<'_, SelectorParseErrorKind<'_>>> {
        let mut input = cssparser::ParserInput::new(s);
        match SelectorList::parse(&KuchikiParser, &mut cssparser::Parser::new(&mut input)) {
            Ok(list) => Ok(Selectors(list.0.into_iter().map(Selector).collect())),
            Err(err) => Err(err),
        }
    }

    /// Returns whether the given element matches this list of selectors.
    #[inline]
    pub fn matches(&self, element: &NodeDataRef<ElementData>) -> bool {
        self.0.iter().any(|s| s.matches(element))
    }

    /// Filter an element iterator, yielding those matching this list of selectors.
    #[inline]
    pub fn filter<I>(&self, iter: I) -> Select<I, &Selectors>
    where
        I: Iterator<Item = NodeDataRef<ElementData>>,
    {
        Select {
            iter,
            selectors: self,
        }
    }
}

impl ::std::str::FromStr for Selectors {
    type Err = ();
    #[inline]
    fn from_str(s: &str) -> Result<Selectors, ()> {
        Selectors::compile(s).map_err(|_| ())
    }
}

impl fmt::Display for Selector {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.0.to_css(f)
    }
}

impl fmt::Display for Selectors {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut iter = self.0.iter();
        let first = iter
            .next()
            .expect("Empty Selectors, should contain at least one selector");
        first.0.to_css(f)?;
        for selector in iter {
            f.write_str(", ")?;
            selector.0.to_css(f)?;
        }
        Ok(())
    }
}

impl fmt::Debug for Selector {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

impl fmt::Debug for Selectors {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}
