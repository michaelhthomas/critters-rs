use std::{collections::HashSet, fs, path::PathBuf};

use critters_rs::html::{parse_html, traits::*, ElementData, NodeDataRef, Selector, Selectors};
use lightningcss::{stylesheet::StyleSheet, traits::ToCss};
use test_log::test;

/// A naive implementation of style calculation for a tree structure.
fn naive_calculate_styles_for_tree(
    element: &NodeDataRef<ElementData>,
    selectors: Vec<Selector>,
) -> Vec<Selector> {
    selectors
        .into_iter()
        .filter(|selector| {
            element
                .as_node()
                .select_first(&format!("{}", selector))
                .is_ok()
        })
        .collect()
}

#[test]
pub fn rust_wikipedia() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("test_files");

    let html = fs::read_to_string(path.join("rust_wikipedia.html")).unwrap();
    let css = fs::read_to_string(path.join("rust_wikipedia.css")).unwrap();

    let document = parse_html().one(html);
    let root = document.select_first("body").unwrap();
    let stylesheet = StyleSheet::parse(&css, Default::default()).unwrap();

    let selectors: Vec<Selector> = stylesheet
        .rules
        .0
        .iter()
        .filter_map(|rule| match rule {
            lightningcss::rules::CssRule::Style(style) => Some(style.selectors.clone()),
            _ => None,
        })
        .filter_map(|selectors| {
            Selectors::compile(&selectors.to_css_string(Default::default()).unwrap()).ok()
        })
        .flat_map(|selectors| selectors.0)
        .collect();

    let expected = naive_calculate_styles_for_tree(&root, selectors.clone());
    let actual = critters_rs::html::style_calculation::calculate_styles_for_tree(&root, selectors);

    let expected_set: HashSet<String> = expected
        .iter()
        .map(|selector| selector.to_string())
        .collect();
    let actual_set: HashSet<String> = actual.iter().map(|selector| selector.to_string()).collect();

    assert_eq!(expected_set, actual_set);
}
