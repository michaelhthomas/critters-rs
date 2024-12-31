use std::{fs, io::Write, path::PathBuf};

use critters_rs::{Critters, CrittersOptions, PreloadStrategy};
use insta::assert_snapshot;
use kuchikiki::traits::TendrilSink;
use tempdir::TempDir;
use test_log::test;

/// Given a dictionary of paths and file contents, construct a temporary directory structure.
///
/// Returns the path to the created temporary folder.
fn create_test_folder(files: &[(&str, &str)]) -> String {
    let tmp_dir = TempDir::new("dist").expect("Failed to create temporary directory");

    for (path, contents) in files {
        let file_path = tmp_dir.path().join(path);
        let mut tmp_file = fs::File::create(file_path).unwrap();
        writeln!(tmp_file, "{}", contents).unwrap();
    }

    tmp_dir.into_path().to_string_lossy().to_string()
}

fn construct_html(head: &str, body: &str) -> String {
    format!(
        r#"<html>
            <head>
                {head}
            </head>
            <body>
                {body}
            </body>
        </html>"#
    )
}

#[test]
fn basic_usage() {
    let tmp_dir = create_test_folder(&[(
        "style.css",
        "h1 { color: blue; }\np { color: purple; }\np.unused { color: orange; }",
    )]);

    let html = construct_html(
        r#"<link rel="stylesheet" href="style.css">"#,
        r#"<h1>Hello World!</h1><p>This is a paragraph</p>"#,
    );

    let critters = Critters::new(CrittersOptions {
        path: tmp_dir.clone(),
        external: true,
        preload: PreloadStrategy::BodyPreload,
        ..Default::default()
    });

    let processed = critters
        .process(&html)
        .expect("Failed to inline critical css");

    let parser = kuchikiki::parse_html();
    let dom = parser.one(processed);

    let inline_style = dom
        .select_first("head > style")
        .expect("Failed to locate inline style")
        .text_contents();

    assert!(inline_style.contains("h1{color:#00f}"), "{inline_style}");
    assert!(inline_style.contains("p{color:purple}"), "{inline_style}");
    assert!(!inline_style.contains("p.unused"), "{inline_style}");

    let stylesheet_link = dom
        .select_first("body > link[rel=stylesheet]:last-child")
        .expect("Failed to locate external stylesheet link.");
    assert_eq!(
        stylesheet_link.attributes.borrow().get("rel"),
        Some("stylesheet")
    );
    assert_eq!(
        stylesheet_link.attributes.borrow().get("href"),
        Some("style.css")
    );
}

#[test]
fn run_on_html_file() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/src");

    let critters = Critters::new(CrittersOptions {
        reduce_inline_styles: false,
        path: path.to_string_lossy().to_string(),
        ..Default::default()
    });

    let html = fs::read_to_string(path.join("index.html")).unwrap();

    let result = critters.process(&html).expect("Failed to process html.");
    insta::assert_snapshot!(result);
}

#[test]
fn does_not_encode_html() {
    let tmp_dir = create_test_folder(&[("style.css", "h1 { color: #00f; }")]);

    let critters = Critters::new(CrittersOptions {
        reduce_inline_styles: false,
        path: tmp_dir,
        ..Default::default()
    });

    let result = critters
        .process(
            r#"<html>
          <head>
            <title>$title</title>
            <link rel="stylesheet" href="/style.css">
          </head>
          <body>
            <h1>Hello World!</h1>
          </body>
        </html>"#,
        )
        .expect("Failed to process html.");

    assert!(result.contains("<style>h1{color:#00f}</style>"), "{result}");
    assert!(
        result.contains("<link rel=\"stylesheet\" href=\"/style.css\">"),
        "{result}"
    );
    assert!(result.contains("<title>$title</title>"), "{result}");
}

#[test]
fn should_keep_existing_link_tag_attributes_in_noscript_link() {
    let tmp_dir = create_test_folder(&[("style.css", "h1 { color: #00f; }")]);

    let critters = Critters::new(CrittersOptions {
        reduce_inline_styles: false,
        path: tmp_dir,
        preload: PreloadStrategy::Media,
        ..Default::default()
    });

    let result = critters.process(
        r#"<html>
        <head>
          <title>$title</title>
          <link rel="stylesheet" href="/style.css" crossorigin="anonymous" integrity="sha384-j1GsrLo96tLqzfCY+">
        </head>
        <body>
          <h1>Hello World!</h1>
        </body>
      </html>"#
    ).expect("Failed to process html.");

    assert!(result.contains("<style>h1{color:#00f}</style>"), "{result}");
    assert!(
        result.contains(r#"<link rel="stylesheet" href="/style.css" crossorigin="anonymous" integrity="sha384-j1GsrLo96tLqzfCY+" media="print" onload="this.media='all'">"#),
        "{result}"
    );
    assert_snapshot!(result);
}

#[test]
fn should_keep_existing_link_tag_attributes() {
    let tmp_dir = create_test_folder(&[("style.css", "h1 { color: #00f; }")]);

    let critters = Critters::new(CrittersOptions {
        reduce_inline_styles: false,
        path: tmp_dir,
        ..Default::default()
    });

    let result = critters.process(
        r#"<html>
        <head>
          <title>$title</title>
          <link rel="stylesheet" href="/style.css" crossorigin="anonymous" integrity="sha384-j1GsrLo96tLqzfCY+">
        </head>
        <body>
          <h1>Hello World!</h1>
        </body>
      </html>"#
    ).expect("Failed to process html.");

    assert!(result.contains("<style>h1{color:#00f}</style>"), "{result}");
    assert!(
        result.contains(r#"<link rel="stylesheet" href="/style.css" crossorigin="anonymous" integrity="sha384-j1GsrLo96tLqzfCY+">"#),
        "{result}"
    );
    assert_snapshot!(result);
}

#[test]
fn does_not_decode_entities_in_html_document() {
    let critters = Critters::new(Default::default());

    let result = critters
        .process(
            r#"<html>
                <body>
                    &lt;h1&gt;Hello World!&lt;/h1&gt;
                </body>
            </html>"#,
        )
        .expect("Failed to process html.");

    assert!(
        result.contains("&lt;h1&gt;Hello World!&lt;/h1&gt;"),
        "{result}"
    );
}

#[test]
fn prevent_injection_via_media_attr() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/src");

    let critters = Critters::new(CrittersOptions {
        reduce_inline_styles: false,
        path: path.to_string_lossy().to_string(),
        preload: PreloadStrategy::Media,
        ..Default::default()
    });

    let html = fs::read_to_string(path.join("media-validation.html")).unwrap();
    let result = critters.process(&html).expect("Failed to process html.");

    assert!(
        result.contains(r#"<noscript><link rel="stylesheet" href="styles2.css" media="screen and (min-width: 480px)"></noscript>"#),
        "{result}"
    );
    assert_snapshot!(result);
}
