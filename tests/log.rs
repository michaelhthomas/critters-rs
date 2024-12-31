use std::{fs, path::PathBuf};

use critters_rs::{Critters, CrittersOptions};
use insta::assert_snapshot;

#[test]
fn skip_invalid_path() {
    mock_logger::init();

    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/src/subpath");

    let critters = Critters::new(CrittersOptions {
        reduce_inline_styles: false,
        external: true,
        path: path.to_string_lossy().to_string(),
        ..Default::default()
    });

    let html = fs::read_to_string(path.join("subpath-validation.html")).unwrap();
    let result = critters.process(&html).expect("Failed to process html.");

    mock_logger::MockLogger::entries(|entries| {
        assert!(
            entries.iter().any(|l| l.level == log::Level::Warn
                && l.body.contains("not within the configured output path")
                && l.body.contains("styles.css")),
            "{}",
            entries
                .iter()
                .filter(|l| l.body.contains("critters_rs"))
                .map(|l| l.body.clone())
                .collect::<Vec<_>>()
                .join("\n")
        );
    });
    assert_snapshot!(result);
}
