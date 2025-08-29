use criterion::{criterion_group, criterion_main, Criterion};
use critters_rs::{Critters, CrittersOptions};
use std::{fs, hint::black_box, path::PathBuf, time::Duration};

fn rust_wikipedia(c: &mut Criterion) {
    env_logger::init();
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("test_files");
    let html = fs::read_to_string(path.join("rust_wikipedia.html")).unwrap();
    let options = CrittersOptions {
        path: path.to_string_lossy().to_string(),
        additional_stylesheets: vec!["rust_wikipedia.css".to_string()],
        external: false,
        ..Default::default()
    };

    c.bench_function("inline_rust_wikipedia", |b| {
        b.iter(|| {
            let critters = Critters::new(options.clone());
            let res = black_box(critters).process(black_box(&html)).unwrap();

            black_box(res);
        })
    });
}

criterion_group! {
    name = benches;
    config = Criterion::default().measurement_time(Duration::from_secs(60));
    targets = rust_wikipedia
}
criterion_main!(benches);
