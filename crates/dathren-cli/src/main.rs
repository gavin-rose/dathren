use std::fs::File;
use std::io::{self, Write};
use std::path::PathBuf;

use anyhow::Result;
use clap::{Parser, Subcommand, ValueEnum};
use dathren::{Client, Query, export, registry};

#[derive(Parser)]
#[command(version, about = "Public health data, cleaned up and documented")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// List available datasets
    List,
    /// Show a dataset's measures and caveats
    Info { id: String },
    /// Fetch normalized observations from a dataset
    Fetch {
        id: String,
        /// Geography code, e.g. US or a state abbreviation
        #[arg(long)]
        geo: Option<String>,
        /// Measure id (repeatable); see `dathren info <id>`
        #[arg(long = "measure")]
        measures: Vec<String>,
        /// Only include data from this year onward
        #[arg(long)]
        since: Option<i32>,
        #[arg(long, value_enum, default_value_t = Format::Csv)]
        format: Format,
        /// Write to a file instead of stdout
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
}

#[derive(Clone, Copy, ValueEnum)]
enum Format {
    Csv,
    Json,
}

fn main() -> Result<()> {
    match Cli::parse().command {
        Command::List => {
            for source in registry::all() {
                let meta = source.meta();
                println!("{:<24} {}", meta.id, meta.title);
            }
        }
        Command::Info { id } => {
            let meta = registry::get(&id)?.meta();
            println!("{}\n{}\n\n{}\n", meta.title, meta.publisher, meta.summary);
            println!(
                "Source:  {}\nUpdates: {}\n",
                meta.source_url, meta.update_cadence
            );
            println!("Measures:");
            for m in meta.measures {
                let code = m.code.map(|c| format!(" [{c}]")).unwrap_or_default();
                println!("  {:<32} {}{code}", m.id, m.label);
            }
            println!("\nCaveats:");
            for c in meta.caveats {
                println!("  - {}: {}", c.title, c.detail);
            }
        }
        Command::Fetch {
            id,
            geo,
            measures,
            since,
            format,
            output,
        } => {
            let source = registry::get(&id)?;
            let query = Query {
                geo,
                measures,
                since_year: since,
            };
            let observations = source.fetch(&Client::new()?, &query)?;
            let mut out: Box<dyn Write> = match output {
                Some(path) => Box::new(File::create(path)?),
                None => Box::new(io::stdout().lock()),
            };
            match format {
                Format::Csv => export::write_csv(&mut out, &observations)?,
                Format::Json => serde_json::to_writer_pretty(&mut out, &observations)?,
            }
        }
    }
    Ok(())
}
