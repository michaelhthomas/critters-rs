use clap::{command, Parser};
use indicatif::MultiProgress;

use critters_rs::{Critters, CrittersOptions};
use indicatif_log_bridge::LogWrapper;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Critters options.
    #[command(flatten)]
    options: CrittersOptions,
}

fn main() -> anyhow::Result<()> {
    let logger = env_logger::Builder::from_env(
        env_logger::Env::default().default_filter_or("critters_rs=warn"),
    )
    .build();
    let multi = MultiProgress::new();
    LogWrapper::new(multi.clone(), logger).try_init().unwrap();

    let args = Args::parse();

    let critters = Critters::new(args.options);
    let stats = critters.process_dir(Some(&multi))?;

    println!(
        "\x1b[0;32m✓ Processed {} pages in {:.2}s.\x1b[0m",
        stats.pages, stats.time_sec
    );
    Ok(())
}
