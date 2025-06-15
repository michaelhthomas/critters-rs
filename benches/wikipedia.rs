use criterion::{criterion_group, criterion_main, Criterion};
use critters_rs::{Critters, CrittersOptions};
use std::{hint::black_box, path::Path, time::Duration};

fn rust_wikipedia(c: &mut Criterion) {
    env_logger::init();
    let html = include_str!("./bench_files/rust_wikipedia.html");
    let options = CrittersOptions {
        path: Path::new(file!())
            .parent()
            .unwrap()
            .join("bench_files")
            .to_string_lossy()
            .to_string(),
        additional_stylesheets: vec!["rust_wikipedia.css".to_string()],
        external: false,
        ..Default::default()
    };

    c.bench_function("inline_rust_wikipedia", |b| {
        b.iter(|| {
            let critters = Critters::new(options.clone());
            let res = black_box(critters).process(&black_box(html)).unwrap();

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
