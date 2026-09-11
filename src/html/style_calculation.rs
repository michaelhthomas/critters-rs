//! Style calculation implementation using bloom filters and Blink's rule set system.
//!
//! This module provides efficient CSS selector matching optimized for performance
//! using techniques from modern browser engines like Blink and WebKit.

use crate::html::filter::StyleBloom;
use crate::html::select::SelectorExt;
use crate::html::{ElementData, NodeDataRef, Selector};
use html5ever::{local_name, LocalName};
use parcel_selectors::context::{MatchingContext, MatchingMode};
use parcel_selectors::parser::{AncestorHashes, Component};
use smallvec::SmallVec;
use std::collections::{HashMap, HashSet};

/// A CSS rule with its selector and the data needed to match it quickly.
#[derive(Debug, Clone)]
struct Rule<'i> {
    /// The CSS selector for this rule
    pub selector: Selector<'i>,
    /// The ancestor hashes for this rule
    pub hashes: AncestorHashes,
}

impl<'i> Rule<'i> {
    /// Creates a new CSS rule with the given parameters.
    pub fn new(selector: Selector<'i>) -> Self {
        let hashes = selector.ancestor_hashes();
        Self { selector, hashes }
    }
}

/// A reference to a Rule which is hashed by address.
///
/// Since rules are stored in a RuleSet throughout the matching process,
/// their pointers are stable and faster to hash than the whole selector.
#[derive(Debug, Clone, Copy)]
struct RuleRef<'a, 'i>(&'a Rule<'i>);

impl Eq for RuleRef<'_, '_> {}

impl PartialEq for RuleRef<'_, '_> {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self.0, other.0)
    }
}

impl std::hash::Hash for RuleRef<'_, '_> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        state.write_usize(self.0 as *const Rule<'_> as usize);
    }
}

/// A rule set organized for fast lookup using hash-based indexing.
///
/// Rules are partitioned into buckets by their key selector component,
/// enabling O(1) lookup time for most selector types.
#[derive(Debug, Default)]
struct RuleSet<'i> {
    /// Rules indexed by ID selectors (highest priority)
    pub id_rules: HashMap<String, Vec<Rule<'i>>>,
    /// Rules indexed by class selectors
    pub class_rules: HashMap<LocalName, Vec<Rule<'i>>>,
    /// Rules indexed by tag selectors
    pub tag_rules: HashMap<LocalName, Vec<Rule<'i>>>,
    /// Universal rules and other selectors that can't be indexed
    pub universal_rules: Vec<Rule<'i>>,
    /// Total rule count for performance tracking
    pub rule_count: usize,
}

impl<'i> RuleSet<'i> {
    /// Creates a new empty rule set.
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a rule to the appropriate hash bucket based on its key selector.
    pub fn add_rule(&mut self, rule: Rule<'i>) {
        let key_component = Self::extract_key_selector(&rule.selector);

        match key_component {
            KeySelector::Id(id) => {
                self.id_rules.entry(id).or_default().push(rule);
            }
            KeySelector::Class(class) => {
                self.class_rules.entry(class).or_default().push(rule);
            }
            KeySelector::Tag(tag) => {
                self.tag_rules.entry(tag).or_default().push(rule);
            }
            KeySelector::Universal => {
                self.universal_rules.push(rule);
            }
        }

        self.rule_count += 1;
    }

    fn extract_key_selector(selector: &Selector) -> KeySelector {
        // Scan the whole rightmost compound for something indexable, preferring the most
        // selective key.
        let mut key = KeySelector::Universal;
        for component in selector.iter() {
            match component {
                Component::ID(id) => return KeySelector::Id(id.0.to_string()),
                Component::Class(class) => {
                    key = KeySelector::Class(LocalName::from(&*class.0));
                }
                Component::LocalName(name) => {
                    if matches!(key, KeySelector::Universal) {
                        key = KeySelector::Tag(LocalName::from(&*name.lower_name.0));
                    }
                }
                _ => {}
            }
        }

        key
    }

    /// Gets all rules that might match the given element from indexed buckets.
    ///
    /// This performs fast O(1) hash lookups rather than scanning all rules.
    #[inline]
    pub fn get_potential_rules(
        &self,
        element: &NodeDataRef<ElementData>,
    ) -> SmallVec<[&Rule<'i>; 16]> {
        let attributes = element.attributes.borrow();

        // Estimate capacity based on universal rules + typical matches
        // Most elements match: universal rules + 1 tag rule + 2-3 class rules
        let estimated_capacity = self.universal_rules.len() + 8;
        let mut rules = SmallVec::with_capacity(estimated_capacity);

        // Always check universal rules
        rules.extend(self.universal_rules.iter());

        // Check ID rules
        if let Some(id) = attributes.get(local_name!("id")) {
            if let Some(id_rules) = self.id_rules.get(id) {
                rules.extend(id_rules.iter());
            }
        }

        // Check class rules
        for class in &attributes.class_list {
            if let Some(class_rules) = self.class_rules.get(class) {
                rules.extend(class_rules.iter());
            }
        }

        // Check tag rules
        let tag_name = &element.name.local;
        if let Some(tag_rules) = self.tag_rules.get(tag_name) {
            rules.extend(tag_rules.iter());
        }

        rules
    }
}
impl<'i> FromIterator<Rule<'i>> for RuleSet<'i> {
    fn from_iter<I: IntoIterator<Item = Rule<'i>>>(iter: I) -> Self {
        let mut set = RuleSet::new();

        for rule in iter {
            set.add_rule(rule);
        }

        set
    }
}

#[derive(Debug, Clone)]
enum KeySelector {
    Id(String),
    Class(LocalName),
    Tag(LocalName),
    Universal,
}

/// Calculates which rules match the given element and adds them to the provided set.
///
/// Uses indexed lookups and bloom filter optimization for performance,
/// then applies CSS cascade rules (specificity + source order).
#[inline]
fn calculate_matching_rules<'a, 'i>(
    element: &NodeDataRef<ElementData>,
    rule_set: &'a RuleSet<'i>,
    bloom: &mut StyleBloom,
    rules: &mut HashSet<RuleRef<'a, 'i>>,
) {
    if cfg!(debug_assertions) {
        bloom.assert_complete(element.clone());
    }

    // Get potential matching rules from indexed buckets
    let potential_rules = rule_set.get_potential_rules(element);

    // One context serves every candidate rule for this element: the matcher restores the state it
    // mutates (nesting level, negation) as it unwinds.
    let mut context = MatchingContext::new(
        MatchingMode::Normal,
        Some(bloom.filter()),
        None,
        parcel_selectors::context::QuirksMode::NoQuirks,
    );

    // Filter rules by actually matching selectors and insert into the set
    for rule in potential_rules {
        if rule
            .selector
            .matches_with_context(element, Some(&rule.hashes), &mut context)
        {
            rules.insert(RuleRef(rule));
        }
    }
}

/// Calculates matching styles for all elements in a DOM tree.
///
/// Returns a mapping from elements to their matching CSS rules,
/// useful for comprehensive style analysis.
pub fn calculate_styles_for_tree<'i>(
    root: &NodeDataRef<ElementData>,
    selectors: impl IntoIterator<Item = Selector<'i>>,
) -> HashSet<Selector<'i>> {
    let rule_set: RuleSet = selectors.into_iter().map(Rule::new).collect();
    let mut bloom = StyleBloom::new();
    bloom.rebuild(root.clone());

    let mut rules = HashSet::new();

    let mut stack: Vec<(NodeDataRef<ElementData>, usize)> =
        vec![(root.clone(), bloom.traversal_depth())];

    while let Some((el, depth)) = stack.pop() {
        // If we have ascended, update the bloom filter
        while bloom.traversal_depth() > depth {
            bloom.pop();
        }

        calculate_matching_rules(&el, &rule_set, &mut bloom, &mut rules);

        // Update bloom filter
        bloom.push(el.clone());
        let depth = bloom.traversal_depth();

        stack.extend(
            el.as_node()
                .children()
                .filter_map(|c| c.into_element_ref())
                .map(|c| (c, depth)),
        );
    }

    rules
        .into_iter()
        .map(|rule| rule.0.selector.clone())
        .collect()
}

#[cfg(test)]
mod tests {
    use itertools::Itertools;

    use super::*;
    use crate::html::parser::parse_html;
    use crate::html::select::Selectors;
    use crate::html::traits::*;

    #[test]
    fn test_calculate_styles_for_tree_empty_selectors() {
        let html = r#"<div><p>Hello</p></div>"#;
        let document = parse_html().one(html);
        let root = document.select_first("body").unwrap();

        let selectors = vec![];
        let result = calculate_styles_for_tree(&root, selectors);

        assert_eq!(result.len(), 0);
    }

    #[test]
    fn test_calculate_styles_for_tree_no_matches() {
        let html = r#"<div><p>Hello</p></div>"#;
        let document = parse_html().one(html);
        let root = document.select_first("body").unwrap();

        let selectors_compiled = Selectors::compile("span.nonexistent").unwrap();
        let selectors = selectors_compiled.0;
        let result = calculate_styles_for_tree(&root, selectors);

        assert_eq!(result.len(), 0);
    }

    #[test]
    fn test_calculate_styles_for_tree_simple_tag_selector() {
        let html = r#"<div><p>Hello</p><span>World</span></div>"#;
        let document = parse_html().one(html);
        let root = document.select_first("body").unwrap();

        let selectors_compiled = Selectors::compile("p").unwrap();
        let selectors = selectors_compiled.0;
        let result = calculate_styles_for_tree(&root, selectors);

        assert_eq!(result.len(), 1);
    }

    #[test]
    fn test_calculate_styles_for_tree_class_selector() {
        let html = r#"<div class="container"><p class="text">Hello</p><span>World</span></div>"#;
        let document = parse_html().one(html);
        let root = document.select_first("body").unwrap();

        let selectors_compiled = Selectors::compile(".text").unwrap();
        let selectors = selectors_compiled.0;
        let result = calculate_styles_for_tree(&root, selectors);

        assert_eq!(result.len(), 1);
    }

    #[test]
    fn test_calculate_styles_for_tree_id_selector() {
        let html = r#"<div><p id="intro">Hello</p><span>World</span></div>"#;
        let document = parse_html().one(html);
        let root = document.select_first("body").unwrap();

        let selectors_compiled = Selectors::compile("#intro").unwrap();
        let selectors = selectors_compiled.0;
        let result = calculate_styles_for_tree(&root, selectors);

        assert_eq!(result.len(), 1);
    }

    #[test]
    fn test_calculate_styles_for_tree_multiple_selectors() {
        let html = r#"<div><p class="text">Hello</p><span id="greeting">World</span></div>"#;
        let document = parse_html().one(html);
        let root = document.select_first("body").unwrap();

        let selectors_compiled = Selectors::compile("p.text, #greeting").unwrap();
        let selectors = selectors_compiled.0;
        let result = calculate_styles_for_tree(&root, selectors);

        assert_eq!(result.len(), 2);
    }

    #[test]
    fn test_calculate_styles_for_tree_universal_selector() {
        let html = r#"<div><p>Hello</p><span>World</span></div>"#;
        let document = parse_html().one(html);
        let root = document.select_first("body").unwrap();

        let selectors_compiled = Selectors::compile("*").unwrap();
        let selectors = selectors_compiled.0;
        let result = calculate_styles_for_tree(&root, selectors);

        // Universal selector should match all elements: html, head, body, div, p, span
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn test_calculate_styles_for_tree_descendant_selector() {
        let html = r#"<div><p><span>Hello</span></p><span>World</span></div>"#;
        let document = parse_html().one(html);
        let root = document.select_first("body").unwrap();

        let selectors_compiled = Selectors::compile("p span").unwrap();
        let selectors = selectors_compiled.0;
        let result = calculate_styles_for_tree(&root, selectors);

        // Only the span inside p should match
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn test_calculate_styles_for_tree_multiple_classes() {
        let html = r#"<div class="container main"><p class="text important">Hello</p></div>"#;
        let document = parse_html().one(html);
        let root = document.select_first("body").unwrap();

        let selectors_compiled = Selectors::compile(".container, .text").unwrap();
        let selectors = selectors_compiled.0;
        let result = calculate_styles_for_tree(&root, selectors);

        assert_eq!(result.len(), 2);
    }

    #[test]
    fn test_calculate_styles_for_tree_complex_selector() {
        let html = r#"<div class="outer"><div class="inner"><p class="text">Hello</p></div></div>"#;
        let document = parse_html().one(html);
        let root = document.select_first("body").unwrap();

        let selectors_compiled = Selectors::compile(".outer .inner .text").unwrap();
        let selectors = selectors_compiled.0;
        let result = calculate_styles_for_tree(&root, selectors);

        assert_eq!(result.len(), 1);
    }

    #[test]
    fn test_calculate_styles_for_tree_nested_elements() {
        let html = r#"
            <div class="level1">
                <div class="level2">
                    <p class="level3">Content</p>
                </div>
            </div>
        "#;
        let document = parse_html().one(html);
        let root = document.select_first("body").unwrap();

        let selectors = [".level2", ".level3", ".unused"]
            .iter()
            .flat_map(|s| Selectors::compile(s).unwrap().0)
            .collect_vec();
        let result = calculate_styles_for_tree(&root, selectors);

        // Should match both elements
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn test_calculate_styles_for_tree_single_element() {
        let html = r#"<p class="single">Only element</p>"#;
        let document = parse_html().one(html);
        let root = document.select_first("body").unwrap();

        let selectors_compiled = Selectors::compile("p.single").unwrap();
        let selectors = selectors_compiled.0;
        let result = calculate_styles_for_tree(&root, selectors);

        assert_eq!(result.len(), 1);
    }

    #[test]
    fn test_calculate_styles_for_tree_pseudo_selector() {
        let html = r#"<div><p>First</p><p>Second</p></div>"#;
        let document = parse_html().one(html);
        let root = document.select_first("body").unwrap();

        let selectors_compiled = Selectors::compile("p:first-child").unwrap();
        let selectors = selectors_compiled.0;
        let result = calculate_styles_for_tree(&root, selectors);

        // Should match the first p element
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn test_calculate_styles_for_tree_bloom_filter_isolation() {
        // This test ensures that selectors don't incorrectly match across different subtrees
        // due to bloom filter false positives or incorrect state persistence
        let html = r#"
            <div>
                <section class="sidebar">
                    <p class="nav-item">Sidebar content</p>
                    <div class="widget">
                        <span class="label">Widget</span>
                    </div>
                </section>
                <main class="content">
                    <article class="post">
                        <h1 class="title">Main content</h1>
                        <p class="body-text">Article body</p>
                    </article>
                    <aside class="metadata">
                        <span class="author">Author info</span>
                    </aside>
                </main>
            </div>
        "#;
        let document = parse_html().one(html);
        let root = document.select_first("body").unwrap();

        // Test selector that should only match elements in the sidebar subtree
        let sidebar_selectors = Selectors::compile(".sidebar .label").unwrap();
        let result = calculate_styles_for_tree(&root, sidebar_selectors.0);

        // Should only match the span.label inside .sidebar, not any elements in .content
        assert_eq!(result.len(), 1);

        // Test selector that should only match elements in the main content subtree
        let content_selectors = Selectors::compile(".content .title").unwrap();
        let result = calculate_styles_for_tree(&root, content_selectors.0);

        // Should only match h1.title inside .content, not any elements in .sidebar
        assert_eq!(result.len(), 1);

        // Test selector that matches elements in both subtrees but with different contexts
        let span_selectors = Selectors::compile("span").unwrap();
        let result = calculate_styles_for_tree(&root, span_selectors.0);

        // Should match both span elements: .label in sidebar and .author in main
        assert_eq!(result.len(), 1);

        // Test complex selector crossing subtrees (should not match due to structure)
        let cross_selectors = Selectors::compile(".sidebar .title").unwrap();
        let result = calculate_styles_for_tree(&root, cross_selectors.0);

        // Should not match anything since .title is not inside .sidebar
        assert_eq!(result.len(), 0);

        // Test another complex selector crossing subtrees (should not match)
        let cross_selectors2 = Selectors::compile(".content .label").unwrap();
        let result = calculate_styles_for_tree(&root, cross_selectors2.0);

        // Should not match anything since .label is not inside .content
        assert_eq!(result.len(), 0);
    }
}
