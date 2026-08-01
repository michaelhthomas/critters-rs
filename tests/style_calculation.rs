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

/// Load a real-world fixture's HTML and its concatenated stylesheets.
fn load_sources(name: &str) -> (String, String) {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("test_files")
        .join(name);

    let html = fs::read_to_string(dir.join("index.html")).unwrap();

    // Concatenate every downloaded stylesheet
    let mut assets: Vec<PathBuf> = fs::read_dir(dir.join("assets"))
        .unwrap()
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|ext| ext == "css"))
        .collect();
    assets.sort();
    let css = assets
        .iter()
        .map(|p| fs::read_to_string(p).unwrap())
        .collect::<Vec<_>>()
        .join("\n");

    (html, css)
}

/// Assert that the optimized style calculation agrees with the naive reference
/// implementation for a real-world page.
fn check_style_calculation(name: &str) {
    let (html, css) = load_sources(name);

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

macro_rules! real_world_sites {
    ($($name:ident),* $(,)?) => {
        $(
            #[test]
            fn $name() {
                check_style_calculation(stringify!($name));
            }
        )*
    };
}

include!("../test_files/sites.gen.rs");
