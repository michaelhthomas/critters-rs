//! Criterion benchmarks over real-world pages fetched by
//! `scripts/fetch-real-world.ts`.
//!
//! Each site listed in `test_files/sites.gen.rs` expands to its own benchmark,
//! so an individual site can be run with e.g.
//!
//!     cargo bench --bench real_world -- rust_wikipedia
//!

use criterion::{criterion_group, criterion_main, Criterion};
use std::{hint::black_box, time::Duration};

#[path = "../test_files/real_world_harness.rs"]
mod harness;

/// Expands the generated site list into one `bench_function` per site.
macro_rules! real_world_sites {
    ($($name:ident),* $(,)?) => {
        fn real_world(c: &mut Criterion) {
            $(
                let (critters, html) = harness::load(stringify!($name));
                c.bench_function(stringify!($name), |b| {
                    b.iter(|| black_box(critters.process(black_box(&html)).unwrap()));
                });
            )*
        }
    };
}

include!("../test_files/sites.gen.rs");

criterion_group! {
    name = benches;
    config = Criterion::default().measurement_time(Duration::from_secs(60));
    targets = real_world
}
criterion_main!(benches);
