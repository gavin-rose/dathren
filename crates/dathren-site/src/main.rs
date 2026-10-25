use std::path::PathBuf;

use anyhow::Result;
use clap::Parser;

/// Builds the dathren website
#[derive(Parser)]
struct Args {
    #[arg(long, default_value = "site")]
    src: PathBuf,
    #[arg(long, default_value = "dist")]
    out: PathBuf,
}

fn main() -> Result<()> {
    let args = Args::parse();
    dathren_site::build(&args.src, &args.out, &dathren::Client::new()?)?;
    println!("Site written to {}", args.out.display());
    Ok(())
}
