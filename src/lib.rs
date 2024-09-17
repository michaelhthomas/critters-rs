use kuchikiki::traits::TendrilSink;
use kuchikiki::{ElementData, NodeData, NodeDataRef, NodeRef};
use lightningcss::printer::PrinterOptions;
use lightningcss::properties::PropertyId;
use lightningcss::rules::{font_face::FontFaceProperty, keyframes::KeyframesName, CssRule};
use lightningcss::selector::SelectorList;
use lightningcss::stylesheet::StyleSheet;
use lightningcss::traits::ToCss;
use lightningcss::values::ident::CustomIdent;
use log::warn;
use markup5ever::{local_name, namespace_url, ns, QualName};
use regex::Regex;
use std::collections::HashSet;
use std::default;
use std::fs;
use std::path::PathBuf;
use utils::StyleRuleExt;

mod utils;

#[derive(Debug, Default)]
pub enum PreloadStrategy {
    /// Move stylesheet links to the end of the document and insert preload meta tags in their place.
    #[default]
    BodyPreload,
    /// Move all external stylesheet links to the end of the document.
    Body,
    /// Load stylesheets asynchronously by adding media="not x" and removing once loaded. JS
    Media,
    /// Convert stylesheet links to preloads that swap to rel="stylesheet" once loaded (details). JS
    Swap,
    /// Use <link rel="alternate stylesheet preload"> and swap to rel="stylesheet" once loaded (details). JS
    SwapHigh,
    /// Inject an asynchronous CSS loader similar to LoadCSS and use it to load stylesheets. JS
    Js,
    /// Like "js", but the stylesheet is disabled until fully loaded.
    JsLazy,
    /// Disables adding preload tags.
    None,
}

#[derive(Debug, Default)]
pub enum KeyframesStrategy {
    /// Inline keyframes rules used by the critical CSS
    #[default]
    Critical,
    /// Inline all keyframes rules
    All,
    /// Remove all keyframes rules
    None,
}

#[derive(Debug)]
pub enum SelectorMatcher {
    String(String),
    Regex(Regex),
}

#[derive(Debug)]
pub struct CrittersOptions {
    /// Base path location of the CSS files
    pub path: String,
    /// Public path of the CSS resources. This prefix is removed from the href.
    pub public_path: String,
    /// Inline styles from external stylesheets
    pub external: bool,
    /// Inline stylesheets smaller than a given size.
    pub inline_threshold: usize,
    /// If the non-critical external stylesheet would be below this size, just inline it
    pub minimum_external_size: usize,
    /// Remove inlined rules from the external stylesheet
    pub prune_source: bool,
    /// Merged inlined stylesheets into a single `<style>` tag
    pub merge_stylesheets: bool,
    /// Glob for matching other stylesheets to be used while looking for critical CSS.
    pub additional_stylesheets: Vec<String>,
    /// Option indicates if inline styles should be evaluated for critical CSS. By default
    /// inline style tags will be evaluated and rewritten to only contain critical CSS.
    /// Set it to false to skip processing inline styles.
    pub reduce_inline_styles: bool,
    /// Which preload strategy to use.
    pub preload: PreloadStrategy,
    /// Add <noscript> fallback to JS-based strategies.
    pub noscript_fallback: bool,
    /// Inline critical font-face rules.
    pub inline_fonts: bool,
    /// Preloads critical fonts (default: true)
    pub preload_fonts: bool,
    /// Controls which keyframes rules are inlined.
    pub keyframes: KeyframesStrategy,
    /// Compress resulting critical CSS
    pub compress: bool,
    /// Provide a list of selectors that should be included in the critical CSS.
    pub allow_rules: Vec<SelectorMatcher>,
}

impl default::Default for CrittersOptions {
    fn default() -> Self {
        Self {
            path: Default::default(),
            public_path: Default::default(),
            external: true,
            inline_threshold: 0,
            minimum_external_size: 0,
            prune_source: false,
            merge_stylesheets: true,
            additional_stylesheets: Default::default(),
            reduce_inline_styles: true,
            preload: Default::default(),
            noscript_fallback: true,
            inline_fonts: false,
            preload_fonts: true,
            keyframes: Default::default(),
            compress: true,
            allow_rules: Default::default(),
        }
    }
}

pub struct Critters {
    options: CrittersOptions,
}

impl Critters {
    pub fn new(options: CrittersOptions) -> Self {
        Critters { options }
    }

    /// Process the given HTML, extracting and inlining critical CSS
    pub fn process(&self, html: &str) -> anyhow::Result<String> {
        // Parse the HTML into a DOM
        let parser = kuchikiki::parse_html();
        let dom = parser.one(html);

        // TODO: handle external stylesheets
        let external_sheets: Vec<NodeDataRef<ElementData>> = dom
            .select("link[rel=\"stylesheet\"]")
            .map_err(|_| anyhow::Error::msg("Failed to select"))?
            .collect();
        println!("{:?}", external_sheets);

        // Locate inline stylesheets
        let styles: Vec<NodeDataRef<ElementData>> = dom.select("style").unwrap().collect();

        // Extract and inline critical CSS
        for style in styles {
            let res = self.process_style_el(style, dom.clone());
            // Log processing errors and skip associated stylesheets
            if let Err(err) = res {
                warn!("Error encountered when processing stylesheet. {}", err);
            }
        }

        // Serialize back to an HTML string
        let mut result = Vec::new();
        dom.serialize(&mut result)?;
        return Ok(String::from_utf8(result)?);
    }

    /// Parse the stylesheet within a <style> element, then reduce it to contain only rules used by the document.
    fn process_style(&self, sheet: &str, dom: NodeRef) -> anyhow::Result<String> {
        // TODO: support container element
        let critters_container = dom.select_first("body").unwrap();
        let mut failed_selectors = Vec::new();
        let mut rules_to_remove = HashSet::new();
        let mut critical_keyframe_names: HashSet<String> = HashSet::new();
        let mut critical_fonts = String::new();

        let mut ast = StyleSheet::parse(&sheet, Default::default())
            .map_err(|_| anyhow::Error::msg("Failed to parse stylesheet."))?;

        // TODO: use a visitor to handle nested rules
        // First pass, mark rules not present in the document for removal
        for rule in &mut ast.rules.0 {
            if let CssRule::Style(style_rule) = rule {
                // TODO: Handle allowed rules
                let global_pseudo_regex = Regex::new(r"/^::?(before|after)$/").unwrap();

                // Filter selectors based on their usage in the document
                let filtered_selectors = style_rule
                    .selectors
                    .0
                    .iter()
                    .filter(|sel| {
                        let selector = sel.to_css_string(Default::default()).unwrap();
                        // easy selectors
                        if selector == ":root"
                            || selector == "html"
                            || selector == "body"
                            || global_pseudo_regex.is_match(&selector)
                        {
                            return true;
                        }

                        // check DOM for elements matching selector
                        match critters_container.as_node().select(&selector) {
                            Ok(iter) => iter.count() > 0,
                            Err(_) => {
                                failed_selectors
                                    .push(format!("{} -> {}", &selector, "Invalid syntax"));
                                false
                            }
                        }
                    })
                    .cloned()
                    .collect::<Vec<_>>();

                if filtered_selectors.is_empty() {
                    rules_to_remove.insert(style_rule.id());
                    break;
                } else {
                    style_rule.selectors = SelectorList::new(filtered_selectors.into());
                }

                // Detect and collect keyframes and font usage
                for decl in &style_rule.declarations.declarations {
                    if matches!(
                        decl.property_id(),
                        PropertyId::Animation(_) | PropertyId::AnimationName(_)
                    ) {
                        let value = decl.value_to_css_string(Default::default()).unwrap();
                        for v in value.split_whitespace() {
                            if !v.trim().is_empty() {
                                critical_keyframe_names.insert(v.trim().to_string());
                            }
                        }
                    }

                    if matches!(decl.property_id(), PropertyId::FontFamily) {
                        critical_fonts.push_str(
                            format!(
                                " {}",
                                &decl.value_to_css_string(Default::default()).unwrap()
                            )
                            .as_str(),
                        );
                    }
                }
            }
        }

        let mut preloaded_fonts = HashSet::new();
        ast.rules.0.retain(|rule| match rule {
            CssRule::Style(s) => !rules_to_remove.contains(&s.id()),
            CssRule::Keyframes(k) => {
                // TODO: keyframes mode options
                let kf_name = match &k.name {
                    KeyframesName::Ident(CustomIdent(id)) | KeyframesName::Custom(id) => id,
                };
                critical_keyframe_names.contains(&kf_name.to_string())
            }
            CssRule::FontFace(f) => {
                let mut src = None;
                let mut family = None;

                for p in &f.properties {
                    match p {
                        FontFaceProperty::Source(s) => {
                            src = Some(s.to_css_string(Default::default()).unwrap())
                        }
                        FontFaceProperty::FontFamily(f) => {
                            family = Some(f.to_css_string(Default::default()).unwrap())
                        }
                        _ => (),
                    }
                }

                // add preload directive to head
                if src.is_some()
                    && self.options.preload_fonts
                    && !preloaded_fonts.contains(src.as_ref().unwrap())
                {
                    let src = src.clone().unwrap();
                    preloaded_fonts.insert(src);
                    let head = dom.select_first("head").unwrap();
                    head.as_node().append(NodeRef::new_element(
                        QualName::new(None, ns!(html), local_name!("link")),
                        vec![(
                            kuchikiki::ExpandedName::new(ns!(html), "rel"),
                            kuchikiki::Attribute {
                                prefix: None,
                                value: "".into(),
                            },
                        )],
                    ))
                }

                self.options.inline_fonts
                    && family.is_some()
                    && src.as_ref().is_some()
                    && critical_fonts.contains(&family.unwrap())
            }
            _ => true,
        });

        // serialize stylesheet
        let css = ast.to_css(PrinterOptions {
            minify: self.options.compress,
            ..Default::default()
        })?;

        Ok(css.code)
    }
    
    fn process_style_el(&self, style: NodeDataRef<ElementData>, dom: NodeRef) -> anyhow::Result<()> {
        let style_node = style.as_node();
        let style_child = match style_node.children().nth(0) {
            Some(c) => c,
            // skip empty stylesheets
            None => return Ok(()),
        };
        let style_data = style_child.data();

        let sheet = match style_data {
            NodeData::Text(t) => t.borrow().to_string(),
            _ => return Err(anyhow::Error::msg("Invalid style tag")),
        };

        // skip empty stylesheets
        if sheet.is_empty() {
            return Ok(());
        }
        
        let css = self.process_style(&sheet, dom)?;

        // remove all existing text from style node
        style_node.children().for_each(|c| c.detach());
        style_node.append(NodeRef::new_text(css));

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_test() {
        let html = r#"
        <html>
            <head>
                <style>
                    .critical { color: red; }
                    .non-critical { color: blue; }
                </style>
            </head>
            <body>
                <div class="critical">Hello World</div>
            </body>
        </html>
    "#;

        let critters = Critters::new(Default::default());

        let processed = critters.process(html).unwrap();
        
        let parser = kuchikiki::parse_html();
        let dom = parser.one(processed);
        let stylesheet = dom.select_first("style").unwrap().text_contents();
        
        assert!(stylesheet.contains(".critical"));
        assert!(!stylesheet.contains(".non-critical"));
    }
}
