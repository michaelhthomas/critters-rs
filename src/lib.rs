use kuchikiki::traits::TendrilSink;
use kuchikiki::{ElementData, NodeData, NodeDataRef, NodeRef};
use lightningcss::properties::PropertyId;
use lightningcss::rules::font_face::FontFaceProperty;
use lightningcss::rules::keyframes::KeyframesName;
use lightningcss::rules::style::StyleRule;
use lightningcss::rules::CssRule;
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

pub struct CrittersOptions {
    pub path: String,
    pub public_path: String,
    pub reduce_inline_styles: bool,
    pub prune_source: bool,
    pub additional_stylesheets: Vec<String>,
    pub allow_rules: Vec<String>,
    pub preload_fonts: bool,
    pub inline_fonts: bool,
}

impl default::Default for CrittersOptions {
    fn default() -> Self {
        Self {
            path: "./dist".to_string(),
            public_path: Default::default(),
            reduce_inline_styles: Default::default(),
            prune_source: Default::default(),
            additional_stylesheets: Default::default(),
            allow_rules: Default::default(),
            preload_fonts: Default::default(),
            inline_fonts: Default::default(),
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
            let res = self.process_style(style, dom.clone());
            if let Err(err) = res {
                warn!("Error encountered when processing stylesheet. {}", err)
            }
        }

        // Serialize back to an HTML string
        let mut result = Vec::new();
        dom.serialize(&mut result)?;
        return Ok(String::from_utf8(result)?);
    }

    /// Parse the stylesheet within a <style> element, then reduce it to contain only rules used by the document.
    fn process_style<'a>(
        &self,
        style: NodeDataRef<ElementData>,
        dom: NodeRef,
    ) -> anyhow::Result<()> {
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

        let critters_container = dom.select_first("body").unwrap();
        let mut failed_selectors = Vec::new();
        let mut rules_to_remove = HashSet::new();
        let mut critical_keyframe_names: HashSet<String> = HashSet::new();
        let mut critical_fonts = String::new();

        let mut ast = StyleSheet::parse(&sheet, Default::default())
            .map_err(|_| anyhow::Error::msg("Failed to parse stylesheet."))?;

        fn get_rule_id(rule: &StyleRule) -> u32 {
            fn cantor(a: u32, b: u32) -> u32 {
                (a + b + 1) * (a + b) / 2 + b
            }
            cantor(
                rule.loc.source_index,
                cantor(rule.loc.line, rule.loc.column),
            )
        }

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
                    rules_to_remove.insert(get_rule_id(style_rule));
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
            CssRule::Style(s) => !rules_to_remove.contains(&get_rule_id(s)),
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
        let css = ast.to_css(Default::default())?;
        // remove all existing text from style node
        style_node.children().for_each(|c| c.detach());
        style_node.append(NodeRef::new_text(css.code));

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

        match critters.process(html) {
            Ok(result) => println!("{}", result),
            Err(e) => panic!("Error: {}", e),
        }
    }
}
