//! Snapshot tests over real-world pages fetched by `scripts/fetch-real-world.ts`.
//!
//! Each site listed in `test_files/sites.gen.rs` expands to its own `#[test]`,
//! so an individual site can be run with e.g.
//!
//!     cargo test --test real_world rust_wikipedia
//!
//! Snapshots live in `tests/snapshots/real_world__<name>.snap`. Regenerate them
//! after refetching with `cargo insta accept`.

#[path = "../test_files/real_world_harness.rs"]
mod harness;

/// Expands the generated site list into one snapshot test per site.
macro_rules! real_world_sites {
    ($($name:ident),* $(,)?) => {
        $(
            #[test]
            fn $name() {
                let (critters, html) = harness::load(stringify!($name));
                let result = critters.process(&html).expect("failed to process html");
                insta::with_settings!({ snapshot_path => "../tests/snapshots" }, {
                    insta::assert_snapshot!(result);
                });
            }
        )*
    };
}

include!("../test_files/sites.gen.rs");
