use std::{collections::HashSet, fs, path::PathBuf};

use critters_rs::html::{parse_html, traits::*, ElementData, NodeDataRef, Selector};
use lightningcss::stylesheet::StyleSheet;
use test_log::test;

/// A naive implementation of style calculation: match every selector against every element in the
/// tree, with none of the indexing or bloom-filter fast rejection the real implementation uses.
fn naive_calculate_styles_for_tree<'i>(
    element: &NodeDataRef<ElementData>,
    selectors: Vec<Selector<'i>>,
) -> Vec<Selector<'i>> {
    let elements: Vec<_> = element
        .as_node()
        .inclusive_descendants()
        .elements()
        .collect();

    selectors
        .into_iter()
        .filter(|selector| elements.iter().any(|element| selector.matches(element)))
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
            lightningcss::rules::CssRule::Style(style) => Some(&style.selectors),
            _ => None,
        })
        .flat_map(|selectors| selectors.0.iter())
        .filter(|selector| selector.is_matchable())
        .cloned()
        .collect();

    let expected: HashSet<Selector> = naive_calculate_styles_for_tree(&root, selectors.clone())
        .into_iter()
        .collect();
    let actual = critters_rs::html::style_calculation::calculate_styles_for_tree(&root, selectors);

    assert_eq!(expected, actual);
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
