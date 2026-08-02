//! Profiling example over the real-world site fixtures.
//!
//! Takes a site name (as listed in `test_files/manifest.jsonc`) as an argument
//! and processes the site's HTML in a tight loop so profilers like
//! `cargo-flamegraph` or `samply` produce meaningful stacks:
//!
//!     cargo run --release --example real_world -- rust_wikipedia

use std::{env, hint::black_box};

use critters_rs::Critters;

#[path = "../test_files/real_world_harness.rs"]
mod harness;

fn main() {
    env_logger::init();

    let name = match env::args().nth(1) {
        Some(name) => name,
        None => {
            eprintln!("usage: cargo run --release --example real_world -- <site-name>");
            eprintln!(
                "  site-name: any site listed in test_files/manifest.jsonc, e.g. rust_wikipedia"
            );
            std::process::exit(2);
        }
    };

    let (options, html) = harness::load_options(&name);

    for _ in 0..100 {
        let critters = Critters::new(options.clone());
        let res = black_box(critters).process(black_box(&html)).unwrap();

        black_box(res);
    }
}
