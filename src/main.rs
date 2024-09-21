use std::{fs, path::PathBuf, time::Instant};

use clap::Parser;
use critters_rs::{Critters, CrittersOptions};
use indicatif::{MultiProgress, ParallelProgressIterator, ProgressBar, ProgressStyle};
use indicatif_log_bridge::LogWrapper;
use log::{info, warn};
use rayon::prelude::*;
use walkdir::WalkDir;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Critters options.
    #[command(flatten)]
    options: CrittersOptions,
}

fn locate_html_files(path: &str) -> anyhow::Result<Vec<PathBuf>> {
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

fn main() -> anyhow::Result<()> {
    let logger = env_logger::Builder::from_env(env_logger::Env::default()).build();
    let multi = MultiProgress::new();
    LogWrapper::new(multi.clone(), logger).try_init().unwrap();
    let progress_style = ProgressStyle::default_bar()
        .progress_chars("━ ━")
        .template("{prefix} {bar:60!.magenta/dim} {pos:>7.cyan}/{len:7.cyan}")
        .unwrap();

    let args = Args::parse();

    let base_path = args.options.path.clone();

    let critters = Critters::new(args.options);
    let files = locate_html_files(&base_path)?;

    let start = Instant::now();
    let progress_bar = multi
        .add(ProgressBar::new(files.len() as u64).with_style(progress_style))
        .with_prefix("Inlining Critical CSS");

    files
        .par_iter()
        .progress_with(progress_bar.clone())
        .for_each(|path| {
            let start = Instant::now();

            let html =
                fs::read_to_string(path.clone()).expect("Failed to load HTML file from disk.");
            let processed = match critters.process(&html) {
                Ok(s) => s,
                Err(e) => {
                    warn!("Failed to process file {} with error {e}", path.display());
                    return;
                }
            };
            fs::write(path.clone(), processed).expect("Failed to write HTML file to disk.");

            let duration = start.elapsed();

            info!(
                "Processed {} in {} ms",
                path.strip_prefix(&base_path).unwrap().display(),
                duration.as_millis()
            );
        });

    progress_bar.finish();
    multi.remove(&progress_bar);
    println!(
        "Processed {} files in {}s.",
        files.len(),
        start.elapsed().as_secs()
    );

    Ok(())
}
