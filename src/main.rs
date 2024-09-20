use std::{fs, path::PathBuf, time::Instant};

use clap::Parser;
use critters_rs::{Critters, CrittersOptions};
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
    let args = Args::parse();

    let base_path = args.options.path.clone();

    let critters = Critters::new(args.options);
    let files = locate_html_files(&base_path)?;

    for path in files {
        let start = Instant::now();

        let html = fs::read_to_string(path.clone())?;
        let processed = critters.process(&html)?;
        fs::write(path.clone(), processed)?;

        let duration = start.elapsed();

        println!(
            "Processed {} in {} ms",
            path.strip_prefix(&base_path)?.display(),
            duration.as_millis()
        );
    }

    Ok(())
}
