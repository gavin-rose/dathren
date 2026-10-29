use std::fs::File;
use std::io::{self, Write};
use std::path::PathBuf;

use anyhow::Result;
use clap::{Args, Parser, Subcommand, ValueEnum};
use dathren::{Client, Observation, Query, export, registry, tools};

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
        #[command(flatten)]
        select: Select,
        /// Convert counts to rates per this many residents, using Census population
        #[arg(long, value_name = "PER", num_args = 0..=1, default_missing_value = "100000")]
        per_capita: Option<f64>,
        #[arg(long, value_enum, default_value_t = Format::Csv)]
        format: Format,
        /// Write to a file instead of stdout
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Sum one count measure, showing suppressed cells as a range instead of zeros
    Total {
        #[command(flatten)]
        select: Select,
        /// Smallest and largest value a suppressed cell can hold
        #[arg(long, num_args = 2, value_names = ["LOW", "HIGH"], default_values_t = [1.0, 9.0])]
        suppressed: Vec<f64>,
    },
    /// Print a citation for a dataset
    Cite {
        id: String,
        #[arg(long, value_enum, default_value_t = Style::Apa)]
        style: Style,
    },
}

#[derive(Args)]
struct Select {
    id: String,
    /// US, a state abbreviation, a 5-digit county FIPS code, or a country code
    #[arg(long)]
    geo: Option<String>,
    /// Measure id (repeatable); see `dathren info <id>`
    #[arg(long = "measure")]
    measures: Vec<String>,
    /// Only include data from this year onward
    #[arg(long)]
    since: Option<i32>,
    /// Keep only each place's newest value
    #[arg(long)]
    latest: bool,
}

impl Select {
    fn fetch(&self, client: &Client) -> Result<Vec<Observation>> {
        let query = Query {
            geo: self.geo.clone(),
            measures: self.measures.clone(),
            since_year: self.since,
        };
        let observations = registry::get(&self.id)?.fetch(client, &query)?;
        Ok(if self.latest {
            tools::latest(observations)
        } else {
            observations
        })
    }
}

#[derive(Clone, Copy, ValueEnum)]
enum Format {
    Csv,
    Json,
}

#[derive(Clone, Copy, ValueEnum)]
enum Style {
    Apa,
    Bibtex,
    Text,
}

fn main() -> Result<()> {
    match Cli::parse().command {
        Command::List => {
            let width = registry::all()
                .iter()
                .map(|s| s.meta().id.len())
                .max()
                .unwrap_or(0);
            for source in registry::all() {
                let meta = source.meta();
                println!("{:<width$}  {}", meta.id, meta.title);
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
            select,
            per_capita,
            format,
            output,
        } => {
            let client = Client::new()?;
            let mut observations = select.fetch(&client)?;
            if let Some(per) = per_capita {
                observations = tools::per_capita_fetch(&client, &observations, per)?;
            }
            let counts = tools::flag_counts(&observations);
            for flag in ["suppressed", "modeled"] {
                if let Some(n) = counts.get(flag) {
                    eprintln!("note: {n} of {} values are {flag}", observations.len());
                }
            }
            let mut out: Box<dyn Write> = match output {
                Some(path) => Box::new(File::create(path)?),
                None => Box::new(io::stdout().lock()),
            };
            match format {
                Format::Csv => export::write_csv(&mut out, &observations)?,
                Format::Json => serde_json::to_writer_pretty(&mut out, &observations)?,
            }
        }
        Command::Total { select, suppressed } => {
            let observations = select.fetch(&Client::new()?)?;
            let t = tools::total(&observations, (suppressed[0], suppressed[1]))?;
            let fmt = |v: f64| format!("{v:.0}");
            if t.suppressed_count == 0 {
                println!("{} from {} values", fmt(t.reported), t.reported_count);
            } else {
                println!(
                    "{} to {} ({} reported from {} values, plus {} suppressed)",
                    fmt(t.low),
                    fmt(t.high),
                    fmt(t.reported),
                    t.reported_count,
                    t.suppressed_count
                );
            }
        }
        Command::Cite { id, style } => {
            let c = tools::cite(registry::get(&id)?.meta(), &tools::today());
            match style {
                Style::Apa => println!("{}", c.apa),
                Style::Bibtex => println!("{}", c.bibtex),
                Style::Text => println!("{}", c.text),
            }
        }
    }
    Ok(())
}
