use crate::html::NodeRef;
use lightningcss::{rules::style::StyleRule, traits::Parse};

/// Locate all the HTML files within a given directory.
#[cfg(feature = "directory")]
pub fn locate_html_files(path: &str) -> anyhow::Result<Vec<std::path::PathBuf>> {
    use walkdir::WalkDir;

    let mut paths = Vec::new();

    for entry in WalkDir::new(path)
        .follow_links(true)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let f_name = entry.file_name().to_string_lossy();

        if f_name.ends_with(".html") {
            paths.push(entry.into_path())
        }
    }

    Ok(paths)
}

pub trait StyleRuleExt {
    /// Generates a unique identifier that can be used to identify the rule in later passes of the AST.
    fn id(&self) -> u128;
}
impl StyleRuleExt for StyleRule<'_> {
    fn id(&self) -> u128 {
        let mut packed_value: u128 = 0;

        packed_value |= (self.loc.source_index as u128) << 64;
        packed_value |= (self.loc.line as u128) << 32;
        packed_value |= self.loc.column as u128;

        packed_value
    }
}

pub trait NodeRefExt {
    /// Creates a new HTML element with the given name and attributes.
    fn new_html_element(name: &str, attributes: Vec<(&str, &str)>) -> NodeRef;
}
impl NodeRefExt for NodeRef {
    fn new_html_element(name: &str, attributes: Vec<(&str, &str)>) -> NodeRef {
        use crate::html::{Attribute, ExpandedName};
        use markup5ever::{namespace_url, ns, LocalName, QualName};

        NodeRef::new_element(
            QualName::new(None, ns!(html), LocalName::from(name)),
            attributes.into_iter().map(|(n, v)| {
                (
                    ExpandedName::new(ns!(), n),
                    Attribute {
                        prefix: None,
                        value: v.to_string(),
                    },
                )
            }),
        )
    }
}

#[cfg(feature = "directory")]
pub trait ProgressBarExt {
    fn with_crate_style(self) -> indicatif::ProgressBar;
}
#[cfg(feature = "directory")]
impl ProgressBarExt for indicatif::ProgressBar {
    fn with_crate_style(self) -> indicatif::ProgressBar {
        self.with_style(
            indicatif::ProgressStyle::default_bar()
                .progress_chars("━ ━")
                .template("{prefix} {bar:60!.magenta/dim} {pos:>7.cyan}/{len:7.cyan}")
                .unwrap(),
        )
    }
}

pub fn is_valid_media_query(s: &str) -> bool {
    lightningcss::media_query::MediaQuery::parse_string(s).is_ok()
}

/// Macro to create a cached Regular expression literal.
macro_rules! regex {
    ($re:expr) => {
        regex!($re, regex::Regex)
    };
    ($re:expr, $engine:ty) => {{
        static RE: std::sync::LazyLock<$engine> =
            std::sync::LazyLock::new(|| <$engine>::new($re).unwrap());
        std::sync::LazyLock::force(&RE)
    }};
}

pub(crate) use regex;
