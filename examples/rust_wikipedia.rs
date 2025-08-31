use std::{fs, hint::black_box, path::PathBuf};

use critters_rs::{Critters, CrittersOptions};

fn main() {
    env_logger::init();
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("test_files");
    let html = fs::read_to_string(path.join("rust_wikipedia.html")).unwrap();
    let options = CrittersOptions {
        path: path.to_string_lossy().to_string(),
        additional_stylesheets: vec!["rust_wikipedia.css".to_string()],
        external: false,
        ..Default::default()
    };

    for _ in 0..100 {
        let critters = Critters::new(options.clone());
        let res = black_box(critters).process(black_box(&html)).unwrap();

        black_box(res);
    }
}
