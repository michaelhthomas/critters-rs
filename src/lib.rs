use itertools::Itertools;
use kuchikiki::traits::TendrilSink;
use kuchikiki::{NodeData, NodeRef};
use lightningcss::printer::PrinterOptions;
use lightningcss::properties::PropertyId;
use lightningcss::rules::{font_face::FontFaceProperty, keyframes::KeyframesName, CssRule};
use lightningcss::selector::SelectorList;
use lightningcss::stylesheet::StyleSheet;
use lightningcss::traits::ToCss;
use lightningcss::values::ident::CustomIdent;
use log::error;
use regex::Regex;
use std::collections::HashSet;
use std::fs;
use std::{default, path};
use utils::{NodeRefExt, StyleRuleExt};

mod utils;

#[derive(Debug, Clone, Default, clap::ValueEnum)]
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

#[derive(Debug, Clone, Default, clap::ValueEnum)]
pub enum KeyframesStrategy {
    /// Inline keyframes rules used by the critical CSS
    #[default]
    Critical,
    /// Inline all keyframes rules
    All,
    /// Remove all keyframes rules
    None,
}

#[derive(Debug, Clone)]
pub enum SelectorMatcher {
    String(String),
    Regex(Regex),
}

#[derive(Debug, Clone, clap::Args)]
pub struct CrittersOptions {
    /// Base path location of the CSS files
    #[clap(short, long)]
    pub path: String,
    /// Public path of the CSS resources. This prefix is removed from the href.
    #[clap(long, default_value_t)]
    pub public_path: String,
    /// Inline styles from external stylesheets
    #[clap(long, action = clap::ArgAction::Set, default_value_t = true)]
    pub external: bool,
    /// Inline stylesheets smaller than a given size.
    #[clap(long, default_value_t)]
    pub inline_threshold: usize,
    /// If the non-critical external stylesheet would be below this size, just inline it
    #[clap(long, default_value_t)]
    pub minimum_external_size: usize,
    /// Remove inlined rules from the external stylesheet
    #[clap(long)]
    pub prune_source: bool,
    /// Merged inlined stylesheets into a single `<style>` tag
    #[clap(long, action = clap::ArgAction::Set, default_value_t = true)]
    pub merge_stylesheets: bool,
    /// Glob for matching other stylesheets to be used while looking for critical CSS.
    #[clap(long)]
    pub additional_stylesheets: Vec<String>,
    /// Option indicates if inline styles should be evaluated for critical CSS. By default
    /// inline style tags will be evaluated and rewritten to only contain critical CSS.
    /// Set it to false to skip processing inline styles.
    #[clap(long, action = clap::ArgAction::Set, default_value_t = true)]
    pub reduce_inline_styles: bool,
    /// Which preload strategy to use.
    #[clap(long, default_value = "body-preload")]
    pub preload: PreloadStrategy,
    /// Add <noscript> fallback to JS-based strategies.
    #[clap(long, action = clap::ArgAction::Set, default_value_t = true)]
    pub noscript_fallback: bool,
    /// Inline critical font-face rules.
    #[clap(long)]
    pub inline_fonts: bool,
    /// Preloads critical fonts
    #[clap(long, action = clap::ArgAction::Set, default_value_t = true)]
    pub preload_fonts: bool,
    /// Controls which keyframes rules are inlined.
    #[clap(long, default_value = "critical")]
    pub keyframes: KeyframesStrategy,
    /// Compress resulting critical CSS
    #[clap(long, action = clap::ArgAction::Set, default_value_t = true)]
    pub compress: bool,
    /// Provide a list of selectors that should be included in the critical CSS.
    #[clap(skip)]
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

#[derive(Clone)]
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

        let mut styles = Vec::new();

        // Inline styles
        if self.options.reduce_inline_styles {
            styles.append(&mut self.get_inline_stylesheets(&dom));
        }

        // External stylesheets
        if self.options.external {
            styles.append(&mut self.get_external_stylesheets(&dom));
        }

        // Additional stylesheets
        if self.options.additional_stylesheets.len() > 0 {
            styles.append(&mut self.get_additional_stylesheets(&dom)?);
        }

        // Extract and inline critical CSS
        for style in styles {
            let res = self.process_style_el(style, dom.clone());
            // Log processing errors and skip associated stylesheets
            if let Err(err) = res {
                error!(
                    "Error encountered when processing stylesheet, skipping. {}",
                    err
                );
            }
        }

        // Merge stylesheets
        if self.options.merge_stylesheets {
            // TODO: merge stylesheets
        }

        // Serialize back to an HTML string
        let mut result = Vec::new();
        dom.serialize(&mut result)?;
        return Ok(String::from_utf8(result)?);
    }

    /// Gets inline styles from the document.
    fn get_inline_stylesheets(&self, dom: &NodeRef) -> Vec<NodeRef> {
        dom.select("style")
            .unwrap()
            .map(|n| n.as_node().clone())
            .collect()
    }

    /// Resolve links to external stylesheets, inlining them and replacing the link with a preload strategy.
    fn get_external_stylesheets(&self, dom: &NodeRef) -> Vec<NodeRef> {
        let external_sheets: Vec<_> = dom.select("link[rel=\"stylesheet\"]").unwrap().collect();

        external_sheets
            .iter()
            .map(|link| {
                self.inline_external_stylesheet(link.as_node(), dom)
                    .unwrap_or_else(|e| {
                        error!("Failed to inline external stylesheet. {e}");
                        None
                    })
            })
            .flatten()
            .collect()
    }

    /// Resolve styles for the provided additional stylesheets, if any, and append them to the head.
    fn get_additional_stylesheets(&self, dom: &NodeRef) -> anyhow::Result<Vec<NodeRef>> {
        self.options
            .additional_stylesheets
            .iter()
            .sorted()
            .dedup()
            .map(|href| self.get_css_asset(href))
            .flatten()
            .map(|css| self.inject_style(&css, dom))
            .collect()
    }

    /// Parse the given stylesheet and reduce it to contain only the nodes present in the given document.
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
                let href_regex =
                    fancy_regex::Regex::new(r#"url\s*\(\s*(['"]?)(.+?)\1\s*\)"#).unwrap();
                let mut href = None;
                let mut family = None;

                for p in &f.properties {
                    match p {
                        FontFaceProperty::Source(s) => {
                            let src = s.to_css_string(Default::default()).unwrap();
                            href = href_regex
                                .captures(&src)
                                .unwrap()
                                .map(|m| m.get(2).map(|c| c.as_str().to_string()))
                                .flatten();
                        }
                        FontFaceProperty::FontFamily(f) => {
                            family = Some(f.to_css_string(Default::default()).unwrap())
                        }
                        _ => (),
                    }
                }

                // add preload directive to head
                if href.is_some()
                    && self.options.preload_fonts
                    && !preloaded_fonts.contains(href.as_ref().unwrap())
                {
                    let href = href.clone().unwrap();
                    if let Err(e) = self.inject_font_preload(&href, &dom) {
                        error!("Failed to inject font preload directive. {e}");
                    }
                    preloaded_fonts.insert(href);
                }

                self.options.inline_fonts
                    && family.is_some()
                    && href.as_ref().is_some()
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

    /// Parse the stylesheet within a <style> element, then reduce it to contain only rules used by the document.
    fn process_style_el(&self, style: NodeRef, dom: NodeRef) -> anyhow::Result<()> {
        let style_child = match style.children().nth(0) {
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
        style.children().for_each(|c| c.detach());
        style.append(NodeRef::new_text(css));

        Ok(())
    }

    /// Given href, find the corresponding CSS asset
    fn get_css_asset(&self, href: &str) -> Option<String> {
        let output_path = &self.options.path;
        let public_path = &self.options.public_path;

        // CHECK - the output path
        // path on disk (with output.publicPath removed)
        let mut normalized_path = href.strip_prefix("/").unwrap_or(href);
        let path_prefix = Regex::new(r"/(^\/|\/$)/")
            .unwrap()
            .replace_all(public_path, "")
            + "/";

        if normalized_path.starts_with(&*path_prefix) {
            normalized_path = normalized_path
                .strip_prefix(&*path_prefix)
                .unwrap_or(normalized_path);
            normalized_path = normalized_path.strip_prefix("/").unwrap_or(normalized_path);
        }

        // Ignore remote stylesheets
        if Regex::new(r"/^https?:\/\//")
            .unwrap()
            .is_match(normalized_path)
            || href.starts_with("//")
        {
            return None;
        }

        let filename = match path::absolute(path::Path::new(output_path).join(normalized_path)) {
            Ok(path) => path,
            Err(e) => {
                error!(
                    "Failed to resolve path with output path {} and href {}. {e}",
                    output_path, normalized_path
                );
                return None;
            }
        };

        // Check if the resolved path is valid
        if !filename.starts_with(output_path) {
            error!("Matched stylesheet with path \"{}\", which is not within the configured output path.", filename.display());
            return None;
        }

        match fs::read_to_string(filename.clone()) {
            Ok(sheet) => Some(sheet),
            Err(e) => {
                error!(
                    "Loading stylesheet at path \"{}\" failed. {e}",
                    filename.display()
                );
                None
            }
        }
    }

    /// Inline the provided stylesheet link, provided it matches the filtering options. Add preload markers for the external stylesheet as necessary.
    fn inline_external_stylesheet(
        &self,
        link: &NodeRef,
        dom: &NodeRef,
    ) -> anyhow::Result<Option<NodeRef>> {
        let link_el = link.as_element().unwrap();
        let link_attrs = link_el.attributes.borrow();
        let href = match link_attrs.get("href") {
            Some(v) if v.ends_with(".css") => v.to_owned(),
            _ => return Ok(None),
        };
        drop(link_attrs);

        let sheet = match self.get_css_asset(&href) {
            Some(v) => v,
            None => return Ok(None),
        };

        let style = NodeRef::new_html_element("style", vec![]);
        style.append(NodeRef::new_text(sheet));
        link.insert_before(style.clone());

        // TODO: inline threshold?

        let body = dom
            .select_first("body")
            .map_err(|_| anyhow::Error::msg("Failed to locate document body"))?;

        let update_link_to_preload = || {
            let mut link_attrs = link_el.attributes.borrow_mut();
            link_attrs.insert("rel", "preload".to_string());
            link_attrs.insert("as", "style".to_string());
        };

        // TODO: Implement other branches, removing those deemed unnecessary
        match self.options.preload {
            PreloadStrategy::BodyPreload => {
                // create new identical link
                let body_link = NodeRef::new(link.data().clone());

                // If an ID is present, remove it to avoid collisions.
                let mut body_link_attrs = body_link.as_element().unwrap().attributes.borrow_mut();
                body_link_attrs.remove("id");
                drop(body_link_attrs);

                body.as_node().append(body_link);

                update_link_to_preload();
            }
            PreloadStrategy::Body => body.as_node().append(link.clone()),
            PreloadStrategy::Media => todo!(),
            PreloadStrategy::Swap => todo!(),
            PreloadStrategy::SwapHigh => todo!(),
            PreloadStrategy::Js | PreloadStrategy::JsLazy => todo!(),
            PreloadStrategy::None => todo!(),
        };

        Ok(Some(style))
    }

    /// Inject the given CSS stylesheet as a new <style> tag in the DOM
    fn inject_style(&self, sheet: &str, dom: &NodeRef) -> anyhow::Result<NodeRef> {
        let head = dom
            .select_first("head")
            .map_err(|_| anyhow::Error::msg("Failed to locate <head> element in DOM."))?;
        let style_node = NodeRef::new_html_element("style", vec![]);

        style_node.append(NodeRef::new_text(sheet));
        head.as_node().append(style_node.clone());

        Ok(style_node)
    }

    /// Injects a preload directive into the head for the given font URL.
    fn inject_font_preload(&self, font: &str, dom: &NodeRef) -> anyhow::Result<()> {
        let head = dom
            .select_first("head")
            .map_err(|_| anyhow::Error::msg("Failed to locate <head> element in DOM."))?;

        head.as_node().append(NodeRef::new_html_element(
            "link",
            vec![
                ("rel", "preload"),
                ("as", "font"),
                ("crossorigin", "anonymous"),
                ("href", font.trim()),
            ],
        ));

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::fs::File;
    use std::io::Write;
    use tempdir::TempDir;
    use test_log::test;

    use super::*;

    const BASIC_CSS: &'static str = r#"
        .critical { color: red; }
        .non-critical { color: blue; }
    "#;

    const BASIC_HTML: &'static str = r#"
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

    fn construct_html(head: &str, body: &str) -> String {
        format!(
            r#"
            <html>
                <head>
                    {head}
                </head>
                <body>
                    {body}
                </body>
            </html>
            "#
        )
    }

    /// Given a dictionary of paths and file contents, construct a temporary directory structure.
    ///
    /// Returns the path to the created temporary folder.
    fn create_test_folder(files: &[(&str, &str)]) -> String {
        let tmp_dir = TempDir::new("dist").expect("Failed to create temporary directory");

        for (path, contents) in files {
            let file_path = tmp_dir.path().join(path);
            let mut tmp_file = File::create(file_path).unwrap();
            writeln!(tmp_file, "{}", contents).unwrap();
        }

        tmp_dir.into_path().to_string_lossy().to_string()
    }

    #[test]
    fn basic() {
        let critters = Critters::new(Default::default());

        let processed = critters.process(BASIC_HTML).unwrap();

        let parser = kuchikiki::parse_html();
        let dom = parser.one(processed);
        let stylesheet = dom.select_first("style").unwrap().text_contents();

        assert!(stylesheet.contains(".critical"));
        assert!(!stylesheet.contains(".non-critical"));
    }

    #[test]
    fn font_preload() {
        let html = construct_html(
            r#"<style>
                @font-face {
                  font-family: "Trickster";
                  src:
                    local("Trickster"),
                    url("trickster-COLRv1.otf") format("opentype") tech(color-COLRv1),
                    url("trickster-outline.otf") format("opentype"),
                    url("trickster-outline.woff") format("woff");
                }
            </style>"#,
            "",
        );
        let critters = Critters::new(Default::default());

        let processed = critters.process(&html).unwrap();

        let parser = kuchikiki::parse_html();
        let dom = parser.one(processed);
        let preload = dom
            .select_first("head > link[rel=preload]")
            .expect("Failed to locate preload link.");
        let preload_attrs = preload.attributes.borrow();

        assert_eq!(preload_attrs.get("rel"), Some("preload"));
        assert_eq!(preload_attrs.get("as"), Some("font"));
        assert_eq!(preload_attrs.get("crossorigin"), Some("anonymous"));
        assert_eq!(preload_attrs.get("href"), Some("trickster-COLRv1.otf"));
    }

    #[test]
    fn external_stylesheet() {
        let tmp_dir = create_test_folder(&[("external.css", BASIC_CSS)]);

        let html = construct_html(
            r#"<link rel="stylesheet" href="external.css" />"#,
            r#"<div class="critical">Hello world</div>"#,
        );

        let critters = Critters::new(CrittersOptions {
            path: tmp_dir,
            external: true,
            preload: PreloadStrategy::BodyPreload,
            ..Default::default()
        });

        let processed = critters
            .process(&html)
            .expect("Failed to inline critical css");

        let parser = kuchikiki::parse_html();
        let dom = parser.one(processed);

        let preload_link = dom
            .select_first("head > link[rel=preload]")
            .expect("Failed to locate preload link.");
        assert_eq!(
            preload_link.attributes.borrow().get("href"),
            Some("external.css")
        );
        assert_eq!(preload_link.attributes.borrow().get("as"), Some("style"));

        let stylesheet = dom
            .select_first("style")
            .expect("Failed to locate inline stylesheet")
            .text_contents();
        assert!(stylesheet.contains(".critical"));
        assert!(!stylesheet.contains(".non-critical"));

        let stylesheet_link = dom
            .select_first("body > link[rel=stylesheet]:last-child")
            .expect("Failed to locate external stylesheet link.");
        assert_eq!(
            stylesheet_link.attributes.borrow().get("rel"),
            Some("stylesheet")
        );
        assert_eq!(
            stylesheet_link.attributes.borrow().get("href"),
            Some("external.css")
        );
    }

    #[test]
    fn additional_stylesheets() {
        let tmp_dir = create_test_folder(&[(
            "add.css",
            ".critical { background-color: blue; } .non-critical { background-color: red; }",
        )]);

        let critters = Critters::new(CrittersOptions {
            path: tmp_dir,
            additional_stylesheets: vec!["add.css".to_string()],
            ..Default::default()
        });

        let processed = critters.process(BASIC_HTML).unwrap();

        let parser = kuchikiki::parse_html();
        let dom = parser.one(processed);
        let stylesheets: Vec<_> = dom
            .select("style")
            .unwrap()
            .map(|s| s.text_contents())
            .collect();

        assert_eq!(stylesheets.len(), 2);
        assert!(stylesheets[0].contains(".critical{color:red}"));
        assert!(!stylesheets[0].contains(".non-critical"));
        assert!(stylesheets[1].contains(".critical{background-color"));
        assert!(!stylesheets[1].contains(".non-critical"));
    }
}
