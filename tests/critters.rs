use std::{fs, io::Write, path::PathBuf};

use critters_rs::html::traits::TendrilSink;
use critters_rs::{Critters, CrittersOptions, Matcher, PreloadStrategy};
use insta::assert_snapshot;
use regex::Regex;
use tempdir::TempDir;
use test_log::test;

/// Given a dictionary of paths and file contents, construct a temporary directory structure.
///
/// Returns the path to the created temporary folder.
fn create_test_folder(files: &[(&str, &str)]) -> String {
    let tmp_dir = TempDir::new("dist").expect("Failed to create temporary directory");

    for (path, contents) in files {
        let file_path = tmp_dir.path().join(path);
        // Create parent directories if they don't exist
        if let Some(parent) = file_path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
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

    let parser = critters_rs::html::parse_html();
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
fn preserves_template_contents() {
    let critters = Critters::new(Default::default());

    let result = critters
        .process(&construct_html(
            "<style>.blue { color: #00f; }</style",
            r#"<div class="blue"><template>Hello world!</template></div>"#,
        ))
        .expect("Failed to process html.");

    assert!(result.contains("Hello world!"));
    insta::assert_snapshot!(result);
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

#[test]
fn run_on_rust_wikipedia() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("test_files");

    let critters = Critters::new(CrittersOptions {
        reduce_inline_styles: false,
        external: false,
        additional_stylesheets: vec!["rust_wikipedia.css".to_string()],
        path: path.to_string_lossy().to_string(),
        ..Default::default()
    });

    let html = fs::read_to_string(path.join("rust_wikipedia.html")).unwrap();

    let result = critters.process(&html).expect("Failed to process html.");
    insta::assert_snapshot!(result);
}

#[test]
fn exclude_external_string_matcher() {
    let tmp_dir = create_test_folder(&[
        ("main.css", "h1 { color: red; } .unused { color: blue; }"),
        (
            "vendor.css",
            ".vendor-class { font-size: 16px; } .vendor-unused { margin: 10px; }",
        ),
    ]);

    let html = construct_html(
        r#"<link rel="stylesheet" href="main.css">
           <link rel="stylesheet" href="vendor.css">"#,
        r#"<h1>Hello World!</h1>
           <div class="vendor-class">Vendor content</div>"#,
    );

    let critters = Critters::new(CrittersOptions {
        path: tmp_dir,
        external: true,
        preload: PreloadStrategy::BodyPreload,
        exclude_external: vec![Matcher::String("vendor.css".to_string())],
        merge_stylesheets: false,
        ..Default::default()
    });

    let processed = critters.process(&html).expect("Failed to process HTML");
    let parser = critters_rs::html::parse_html();
    let dom = parser.one(processed);

    // Check that vendor.css is completely excluded (no link elements)
    let vendor_links = dom
        .select("link[href=\"vendor.css\"]")
        .unwrap()
        .collect::<Vec<_>>();
    assert_eq!(
        vendor_links.len(),
        0,
        "vendor.css link should be completely removed"
    );

    // Check that main.css follows normal preload strategy
    let main_preload = dom.select_first("head > link[rel=preload][href=\"main.css\"]");
    assert!(main_preload.is_ok(), "main.css should have preload link");

    let main_body_link = dom.select_first("body > link[rel=stylesheet][href=\"main.css\"]");
    assert!(
        main_body_link.is_ok(),
        "main.css should have body stylesheet link"
    );

    // Check that both stylesheets have their critical CSS inlined
    let inline_styles = dom.select("head > style").unwrap().collect::<Vec<_>>();
    assert_eq!(
        inline_styles.len(),
        2,
        "Should have 2 inline style elements"
    );

    let combined_styles: String = inline_styles.iter().map(|s| s.text_contents()).collect();
    assert!(
        combined_styles.contains("h1{color:red}"),
        "Should contain main.css critical CSS"
    );
    assert!(
        combined_styles.contains(".vendor-class{font-size:16px}"),
        "Should contain vendor.css critical CSS"
    );
    assert!(
        !combined_styles.contains(".unused"),
        "Should not contain unused main.css rules"
    );
    assert!(
        !combined_styles.contains(".vendor-unused"),
        "Should not contain unused vendor.css rules"
    );
}

#[test]
fn exclude_external_multiple_stylesheets() {
    let tmp_dir = create_test_folder(&[
        (
            "main.css",
            "h1 { color: red; } .main-unused { color: blue; }",
        ),
        (
            "vendor.css",
            ".vendor { font-size: 16px; } .vendor-unused { margin: 10px; }",
        ),
        (
            "theme.css",
            ".theme { background: white; } .theme-unused { padding: 5px; }",
        ),
        (
            "bootstrap.css",
            ".btn { padding: 8px; } .btn-unused { border: 1px solid; }",
        ),
    ]);

    let html = construct_html(
        r#"<link rel="stylesheet" href="main.css">
           <link rel="stylesheet" href="vendor.css">
           <link rel="stylesheet" href="theme.css">
           <link rel="stylesheet" href="bootstrap.css">"#,
        r#"<h1>Hello World!</h1>
           <div class="vendor">Vendor content</div>
           <div class="btn">Button</div>"#,
    );

    let critters = Critters::new(CrittersOptions {
        path: tmp_dir,
        external: true,
        preload: PreloadStrategy::BodyPreload,
        exclude_external: vec![
            Matcher::String("vendor.css".to_string()),
            Matcher::Regex(Regex::new("bootstrap\\.css$").unwrap()),
        ],
        merge_stylesheets: false,
        ..Default::default()
    });

    let processed = critters.process(&html).expect("Failed to process HTML");
    let parser = critters_rs::html::parse_html();
    let dom = parser.one(processed);

    // Check excluded stylesheets are completely removed
    assert!(
        dom.select("link[href=\"vendor.css\"]")
            .unwrap()
            .next()
            .is_none(),
        "vendor.css should be removed"
    );
    assert!(
        dom.select("link[href=\"bootstrap.css\"]")
            .unwrap()
            .next()
            .is_none(),
        "bootstrap.css should be removed"
    );

    // Check non-excluded stylesheets follow normal preload strategy
    assert!(
        dom.select_first("link[rel=preload][href=\"main.css\"]")
            .is_ok(),
        "main.css should have preload"
    );
    assert!(
        dom.select_first("link[rel=preload][href=\"theme.css\"]")
            .is_ok(),
        "theme.css should have preload"
    );
    assert!(
        dom.select_first("body > link[rel=stylesheet][href=\"main.css\"]")
            .is_ok(),
        "main.css should be in body"
    );
    assert!(
        dom.select_first("body > link[rel=stylesheet][href=\"theme.css\"]")
            .is_ok(),
        "theme.css should be in body"
    );

    // Check that all stylesheets have their critical CSS inlined
    let inline_styles = dom.select("head > style").unwrap().collect::<Vec<_>>();
    assert_eq!(
        inline_styles.len(),
        4,
        "Should have 4 inline style elements"
    );

    let combined_styles: String = inline_styles.iter().map(|s| s.text_contents()).collect();
    assert!(
        combined_styles.contains("h1{color:red}"),
        "Should contain main.css critical CSS"
    );
    assert!(
        combined_styles.contains(".vendor{font-size:16px}"),
        "Should contain vendor.css critical CSS"
    );
    assert!(
        combined_styles.contains(".btn{padding:8px}"),
        "Should contain bootstrap.css critical CSS"
    );
    // theme.css has no matching elements, so no critical CSS should be inlined
    assert!(
        !combined_styles.contains(".theme"),
        "Should not contain unused theme.css rules"
    );
}

#[test]
fn exclude_external_with_different_preload_strategies() {
    let tmp_dir = create_test_folder(&[
        ("main.css", "h1 { color: red; }"),
        ("excluded.css", ".excluded { font-size: 14px; }"),
    ]);

    let html = construct_html(
        r#"<link rel="stylesheet" href="main.css">
           <link rel="stylesheet" href="excluded.css">"#,
        r#"<h1>Hello World!</h1>
           <div class="excluded">Excluded content</div>"#,
    );

    // Test with Media strategy
    let critters_media = Critters::new(CrittersOptions {
        path: tmp_dir.clone(),
        external: true,
        preload: PreloadStrategy::Media,
        exclude_external: vec![Matcher::String("excluded.css".to_string())],
        ..Default::default()
    });

    let processed = critters_media
        .process(&html)
        .expect("Failed to process HTML");
    let parser = critters_rs::html::parse_html();
    let dom = parser.one(processed);

    // Excluded stylesheet should not exist anywhere
    assert!(
        dom.select("link[href=\"excluded.css\"]")
            .unwrap()
            .next()
            .is_none(),
        "excluded.css should not exist"
    );
    // Main stylesheet should use Media strategy
    assert!(
        dom.select_first("link[href=\"main.css\"][media=print]")
            .is_ok(),
        "main.css should have media=print"
    );
    assert!(
        dom.select_first("noscript").is_ok(),
        "Should have noscript fallback"
    );

    // Test with Swap strategy
    let critters_swap = Critters::new(CrittersOptions {
        path: tmp_dir.clone(),
        external: true,
        preload: PreloadStrategy::Swap,
        exclude_external: vec![Matcher::String("excluded.css".to_string())],
        ..Default::default()
    });

    let processed = critters_swap
        .process(&html)
        .expect("Failed to process HTML");
    let parser = critters_rs::html::parse_html();
    let dom = parser.one(processed);

    // Excluded stylesheet should not exist anywhere
    assert!(
        dom.select("link[href=\"excluded.css\"]")
            .unwrap()
            .next()
            .is_none(),
        "excluded.css should not exist"
    );
    // Main stylesheet should use Swap strategy
    assert!(
        dom.select_first("link[href=\"main.css\"][rel=preload]")
            .is_ok(),
        "main.css should be preload"
    );
    assert!(
        dom.select_first("noscript").is_ok(),
        "Should have noscript fallback"
    );

    // Test with None strategy (no preload)
    let critters_none = Critters::new(CrittersOptions {
        path: tmp_dir,
        external: true,
        preload: PreloadStrategy::None,
        exclude_external: vec![Matcher::String("excluded.css".to_string())],
        ..Default::default()
    });

    let processed = critters_none
        .process(&html)
        .expect("Failed to process HTML");
    let parser = critters_rs::html::parse_html();
    let dom = parser.one(processed);

    // Excluded stylesheet should not exist anywhere
    assert!(
        dom.select("link[href=\"excluded.css\"]")
            .unwrap()
            .next()
            .is_none(),
        "excluded.css should not exist"
    );
    // Main stylesheet should remain as normal link (no preload transformation)
    assert!(
        dom.select_first("link[href=\"main.css\"][rel=stylesheet]")
            .is_ok(),
        "main.css should remain as stylesheet"
    );
    assert!(
        dom.select("link[rel=preload]").unwrap().next().is_none(),
        "Should have no preload links"
    );
}

#[test]
fn exclude_external_path_variations() {
    let tmp_dir = create_test_folder(&[
        ("styles.css", ".styles { color: blue; }"),
        ("css/main.css", ".main { color: red; }"),
        ("vendor/bootstrap.css", ".bootstrap { font-size: 16px; }"),
        ("assets/theme.css", ".theme { background: white; }"),
    ]);

    let html = construct_html(
        r#"<link rel="stylesheet" href="/styles.css">
           <link rel="stylesheet" href="./css/main.css">
           <link rel="stylesheet" href="vendor/bootstrap.css">
           <link rel="stylesheet" href="/assets/theme.css">"#,
        r#"<div class="styles">Styles content</div>
           <div class="main">Main content</div>
           <div class="bootstrap">Bootstrap content</div>
           <div class="theme">Theme content</div>"#,
    );

    let critters = Critters::new(CrittersOptions {
        path: tmp_dir,
        external: true,
        preload: PreloadStrategy::BodyPreload,
        exclude_external: vec![
            Matcher::String("/styles.css".to_string()), // matches /styles.css
            Matcher::String("./css/main.css".to_string()), // matches ./css/main.css
            Matcher::Regex(Regex::new("assets/.*\\.css$").unwrap()), // matches /assets/theme.css
        ],
        merge_stylesheets: false,
        ..Default::default()
    });

    let processed = critters.process(&html).expect("Failed to process HTML");
    let parser = critters_rs::html::parse_html();
    let dom = parser.one(processed);

    // Check excluded stylesheets are completely removed
    assert!(
        dom.select("link[href=\"/styles.css\"]")
            .unwrap()
            .next()
            .is_none(),
        "/styles.css should be removed"
    );
    assert!(
        dom.select("link[href=\"./css/main.css\"]")
            .unwrap()
            .next()
            .is_none(),
        "./css/main.css should be removed"
    );
    assert!(
        dom.select("link[href=\"/assets/theme.css\"]")
            .unwrap()
            .next()
            .is_none(),
        "/assets/theme.css should be removed"
    );

    // Check non-excluded stylesheet follows normal preload strategy
    assert!(
        dom.select_first("link[rel=preload][href=\"vendor/bootstrap.css\"]")
            .is_ok(),
        "vendor/bootstrap.css should have preload"
    );
    assert!(
        dom.select_first("body > link[rel=stylesheet][href=\"vendor/bootstrap.css\"]")
            .is_ok(),
        "vendor/bootstrap.css should be in body"
    );

    // Check that all stylesheets have their critical CSS inlined
    let inline_styles = dom.select("head > style").unwrap().collect::<Vec<_>>();
    assert_eq!(
        inline_styles.len(),
        4,
        "Should have 4 inline style elements"
    );

    let combined_styles: String = inline_styles.iter().map(|s| s.text_contents()).collect();
    assert!(
        combined_styles.contains(".styles{color:#00f}"),
        "Should contain styles.css critical CSS"
    );
    assert!(
        combined_styles.contains(".main{color:red}"),
        "Should contain main.css critical CSS"
    );
    assert!(
        combined_styles.contains(".bootstrap{font-size:16px}"),
        "Should contain bootstrap.css critical CSS"
    );
    assert!(
        combined_styles.contains(".theme{background:#fff}"),
        "Should contain theme.css critical CSS"
    );
}

#[test]
fn exclude_external_complex_regex_patterns() {
    let tmp_dir = create_test_folder(&[
        ("bootstrap.min.css", ".bootstrap { color: blue; }"),
        ("foundation.min.css", ".foundation { color: green; }"),
        ("vendor-v1.2.3.css", ".vendor { color: red; }"),
        ("lib-utils.css", ".utils { color: purple; }"),
        ("app-main.css", ".main { color: orange; }"),
        ("custom.css", ".custom { color: yellow; }"),
    ]);

    let html = construct_html(
        r#"<link rel="stylesheet" href="bootstrap.min.css">
           <link rel="stylesheet" href="foundation.min.css">
           <link rel="stylesheet" href="vendor-v1.2.3.css">
           <link rel="stylesheet" href="lib-utils.css">
           <link rel="stylesheet" href="app-main.css">
           <link rel="stylesheet" href="custom.css">"#,
        r#"<div class="bootstrap">Bootstrap</div>
           <div class="foundation">Foundation</div>
           <div class="vendor">Vendor</div>
           <div class="utils">Utils</div>
           <div class="main">Main</div>
           <div class="custom">Custom</div>"#,
    );

    let critters = Critters::new(CrittersOptions {
        path: tmp_dir,
        external: true,
        preload: PreloadStrategy::BodyPreload,
        exclude_external: vec![
            // Match any .min.css files
            Matcher::Regex(Regex::new(r"\.min\.css$").unwrap()),
            // Match vendor files with version numbers
            Matcher::Regex(Regex::new(r"vendor-v\d+\.\d+\.\d+\.css$").unwrap()),
            // Match files starting with "lib-"
            Matcher::Regex(Regex::new(r"^lib-.*\.css$").unwrap()),
            // Complex pattern: match either bootstrap OR foundation
            Matcher::Regex(Regex::new(r"^(bootstrap|foundation).*\.css$").unwrap()),
        ],
        merge_stylesheets: false,
        ..Default::default()
    });

    let processed = critters.process(&html).expect("Failed to process HTML");
    let parser = critters_rs::html::parse_html();
    let dom = parser.one(processed);

    // Check that all matched stylesheets are excluded
    assert!(
        dom.select("link[href=\"bootstrap.min.css\"]")
            .unwrap()
            .next()
            .is_none(),
        "bootstrap.min.css should be excluded"
    );
    assert!(
        dom.select("link[href=\"foundation.min.css\"]")
            .unwrap()
            .next()
            .is_none(),
        "foundation.min.css should be excluded"
    );
    assert!(
        dom.select("link[href=\"vendor-v1.2.3.css\"]")
            .unwrap()
            .next()
            .is_none(),
        "vendor-v1.2.3.css should be excluded"
    );
    assert!(
        dom.select("link[href=\"lib-utils.css\"]")
            .unwrap()
            .next()
            .is_none(),
        "lib-utils.css should be excluded"
    );

    // Check non-matched stylesheets follow normal preload strategy
    assert!(
        dom.select_first("link[rel=preload][href=\"app-main.css\"]")
            .is_ok(),
        "app-main.css should have preload"
    );
    assert!(
        dom.select_first("link[rel=preload][href=\"custom.css\"]")
            .is_ok(),
        "custom.css should have preload"
    );
    assert!(
        dom.select_first("body > link[rel=stylesheet][href=\"app-main.css\"]")
            .is_ok(),
        "app-main.css should be in body"
    );
    assert!(
        dom.select_first("body > link[rel=stylesheet][href=\"custom.css\"]")
            .is_ok(),
        "custom.css should be in body"
    );

    // Check that all stylesheets have their critical CSS inlined
    let inline_styles = dom.select("head > style").unwrap().collect::<Vec<_>>();
    assert_eq!(
        inline_styles.len(),
        6,
        "Should have 6 inline style elements"
    );

    let combined_styles: String = inline_styles.iter().map(|s| s.text_contents()).collect();
    assert!(
        combined_styles.contains(".bootstrap{color:#00f}"),
        "Should contain bootstrap critical CSS"
    );
    assert!(
        combined_styles.contains(".foundation{color:green}"),
        "Should contain foundation critical CSS"
    );
    assert!(
        combined_styles.contains(".vendor{color:red}"),
        "Should contain vendor critical CSS"
    );
    assert!(
        combined_styles.contains(".utils{color:purple}"),
        "Should contain utils critical CSS"
    );
    assert!(
        combined_styles.contains(".main{color:orange}"),
        "Should contain main critical CSS"
    );
    assert!(
        combined_styles.contains(".custom{color:#ff0}"),
        "Should contain custom critical CSS"
    );
}

#[test]
fn exclude_external_comprehensive_snapshot() {
    let tmp_dir = create_test_folder(&[
        (
            "global.css",
            r#"
            body { margin: 0; padding: 0; }
            .container { max-width: 1200px; margin: 0 auto; }
            .unused-global { display: none; }
        "#,
        ),
        (
            "components.css",
            r#"
            .header { background: #333; color: white; }
            .nav-item { padding: 10px; }
            .footer { background: #f5f5f5; }
            .unused-component { visibility: hidden; }
        "#,
        ),
        (
            "vendor.css",
            r#"
            .vendor-reset * { box-sizing: border-box; }
            .vendor-grid { display: grid; }
            .vendor-unused { opacity: 0; }
        "#,
        ),
        (
            "theme.css",
            r#"
            .theme-primary { color: #007bff; }
            .theme-secondary { color: #6c757d; }
            .theme-unused { transform: scale(0); }
        "#,
        ),
    ]);

    let html = construct_html(
        r#"<title>Exclude External Test Page</title>
           <meta charset="utf-8">
           <link rel="stylesheet" href="global.css">
           <link rel="stylesheet" href="components.css">
           <link rel="stylesheet" href="vendor.css">
           <link rel="stylesheet" href="theme.css">
           <style>
               .inline-style { font-weight: bold; }
               .unused-inline { text-decoration: underline; }
           </style>"#,
        r#"<div class="container">
               <header class="header">
                   <nav>
                       <span class="nav-item">Home</span>
                       <span class="nav-item">About</span>
                   </nav>
               </header>
               <main>
                   <div class="vendor-grid">
                       <div class="theme-primary inline-style">Primary content</div>
                       <div class="theme-secondary">Secondary content</div>
                   </div>
               </main>
               <footer class="footer">Footer content</footer>
           </div>"#,
    );

    let critters = Critters::new(CrittersOptions {
        path: tmp_dir,
        external: true,
        preload: PreloadStrategy::Swap,
        exclude_external: vec![
            Matcher::String("vendor.css".to_string()),
            Matcher::Regex(Regex::new("theme\\.css$").unwrap()),
        ],
        reduce_inline_styles: true,
        merge_stylesheets: false,
        noscript_fallback: true,
        compress: true,
        ..Default::default()
    });

    let processed = critters.process(&html).expect("Failed to process HTML");

    // Use snapshot testing for complete output verification
    assert_snapshot!(processed);
}

#[test]
fn exclude_external_edge_cases() {
    let tmp_dir = create_test_folder(&[
        ("existing.css", ".existing { color: blue; }"),
        ("empty.css", ""),
        ("malformed.css", ".malformed { color: ; }"), // Invalid CSS
    ]);

    // Test with empty exclude_external list
    let html_empty = construct_html(
        r#"<link rel="stylesheet" href="existing.css">"#,
        r#"<div class="existing">Content</div>"#,
    );

    let critters_empty = Critters::new(CrittersOptions {
        path: tmp_dir.clone(),
        external: true,
        exclude_external: vec![], // Empty exclude list
        ..Default::default()
    });

    let processed = critters_empty
        .process(&html_empty)
        .expect("Failed to process HTML with empty exclude list");
    let parser = critters_rs::html::parse_html();
    let dom = parser.one(processed);

    // With empty exclude list, stylesheet should follow normal processing
    assert!(
        dom.select_first("link[rel=preload][href=\"existing.css\"]")
            .is_ok(),
        "Should have preload with empty exclude list"
    );

    // Test excluding non-existent stylesheet
    let html_nonexistent = construct_html(
        r#"<link rel="stylesheet" href="existing.css">
           <link rel="stylesheet" href="nonexistent.css">"#,
        r#"<div class="existing">Content</div>"#,
    );

    let critters_nonexistent = Critters::new(CrittersOptions {
        path: tmp_dir.clone(),
        external: true,
        exclude_external: vec![Matcher::String("nonexistent.css".to_string())],
        ..Default::default()
    });

    let processed = critters_nonexistent
        .process(&html_nonexistent)
        .expect("Failed to process HTML with nonexistent exclude");
    let parser = critters_rs::html::parse_html();
    let dom = parser.one(processed);

    // Nonexistent stylesheet link should remain unchanged (since file doesn't exist, no processing happens)
    // Existing stylesheet should be processed normally
    assert!(
        dom.select_first("link[rel=preload][href=\"existing.css\"]")
            .is_ok(),
        "Existing stylesheet should be processed"
    );
    // Nonexistent stylesheet link should remain in place since the file doesn't exist to process
    assert!(
        dom.select_first("link[rel=stylesheet][href=\"nonexistent.css\"]")
            .is_ok(),
        "Nonexistent stylesheet should remain as original link"
    );

    // Test excluding empty CSS file
    let html_empty_css = construct_html(
        r#"<link rel="stylesheet" href="existing.css">
           <link rel="stylesheet" href="empty.css">"#,
        r#"<div class="existing">Content</div>"#,
    );

    let critters_empty_css = Critters::new(CrittersOptions {
        path: tmp_dir.clone(),
        external: true,
        exclude_external: vec![Matcher::String("empty.css".to_string())],
        ..Default::default()
    });

    let processed = critters_empty_css
        .process(&html_empty_css)
        .expect("Failed to process HTML with empty CSS exclude");
    let parser = critters_rs::html::parse_html();
    let dom = parser.one(processed);

    // Empty CSS file should be excluded, existing one should be processed
    assert!(
        dom.select_first("link[rel=preload][href=\"existing.css\"]")
            .is_ok(),
        "Existing stylesheet should be processed"
    );
    assert!(
        dom.select("link[href=\"empty.css\"]")
            .unwrap()
            .next()
            .is_none(),
        "Empty stylesheet should be excluded"
    );

    // Test with malformed CSS that might still be excluded
    let html_malformed = construct_html(
        r#"<link rel="stylesheet" href="existing.css">
           <link rel="stylesheet" href="malformed.css">"#,
        r#"<div class="existing">Content</div>"#,
    );

    let critters_malformed = Critters::new(CrittersOptions {
        path: tmp_dir,
        external: true,
        exclude_external: vec![Matcher::String("malformed.css".to_string())],
        ..Default::default()
    });

    let processed = critters_malformed
        .process(&html_malformed)
        .expect("Failed to process HTML with malformed CSS exclude");
    let parser = critters_rs::html::parse_html();
    let dom = parser.one(processed);

    // Malformed CSS should be excluded regardless of its content
    assert!(
        dom.select_first("link[rel=preload][href=\"existing.css\"]")
            .is_ok(),
        "Existing stylesheet should be processed"
    );
    assert!(
        dom.select("link[href=\"malformed.css\"]")
            .unwrap()
            .next()
            .is_none(),
        "Malformed stylesheet should be excluded"
    );

    // Verify that we still get inline styles even when some are excluded
    let inline_styles = dom.select("head > style").unwrap().collect::<Vec<_>>();
    assert!(
        !inline_styles.is_empty(),
        "Should have at least one inline style from existing.css"
    );
}
