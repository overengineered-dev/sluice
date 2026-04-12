use std::fs::File;
use std::io::{self, BufReader, BufWriter, Read, Write};
use std::path::PathBuf;
use std::time::Instant;

use anyhow::{Context, Result};
use clap::Parser;
use flate2::read::GzDecoder;
use sluice::{classify, IndexReader, Record, Uinfo};
use serde::Serialize;

/// Parse a Maven Central index file (full or incremental chunk) into JSON
/// Lines on stdout.
#[derive(Debug, Parser)]
#[command(
    name = "sluice",
    version,
    about = "Stream Maven Central index documents as JSON Lines."
)]
struct Args {
    /// Path to a gzipped Maven index file. Reads from stdin if omitted.
    input: Option<PathBuf>,

    /// Also emit `ArtifactRemove` records (type="remove") alongside adds.
    #[arg(long)]
    include_removes: bool,

    /// Print summary stats to stderr at end of run.
    #[arg(long)]
    stats: bool,
}

#[derive(Serialize)]
#[serde(tag = "type", rename_all = "lowercase")]
enum OutputRecord<'a> {
    Add {
        group_id: &'a str,
        artifact_id: &'a str,
        version: &'a str,
        extension: Option<&'a str>,
    },
    Remove {
        group_id: &'a str,
        artifact_id: &'a str,
        version: &'a str,
        extension: Option<&'a str>,
    },
}

#[derive(Default)]
struct Stats {
    total: u64,
    adds: u64,
    removes: u64,
    descriptor: u64,
    all_groups: u64,
    root_groups: u64,
    unknown: u64,
    emitted: u64,
    filtered_classifier: u64,
    errors: u64,
    first_uinfo_adds: Vec<String>,
}

impl Stats {
    fn record_add(&mut self, u: &Uinfo) {
        if self.first_uinfo_adds.len() < 10 {
            self.first_uinfo_adds.push(format!(
                "{} | {} | {} | {} | {}",
                u.group_id,
                u.artifact_id,
                u.version,
                u.classifier.as_deref().unwrap_or("NA"),
                u.extension.as_deref().unwrap_or("-"),
            ));
        }
    }
}

fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_writer(io::stderr)
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("warn")),
        )
        .init();

    let args = Args::parse();

    let reader: Box<dyn Read> = match args.input.as_ref() {
        Some(path) => {
            Box::new(File::open(path).with_context(|| format!("opening {}", path.display()))?)
        }
        None => Box::new(io::stdin().lock()),
    };
    let gz = GzDecoder::new(BufReader::new(reader));
    let buffered = BufReader::new(gz);
    let index = IndexReader::new(buffered).context("reading index header")?;

    let stdout = io::stdout().lock();
    let mut stdout = BufWriter::new(stdout);
    let mut stats = Stats::default();

    let start = Instant::now();
    for item in index {
        let doc = item.context("parsing document")?;
        stats.total += 1;

        let record = match classify(&doc) {
            Ok(r) => r,
            Err(e) => {
                stats.errors += 1;
                return Err(anyhow::Error::from(e).context("classifying document"));
            }
        };

        match record {
            Record::Descriptor => stats.descriptor += 1,
            Record::AllGroups => stats.all_groups += 1,
            Record::RootGroups => stats.root_groups += 1,
            Record::Unknown => stats.unknown += 1,
            Record::ArtifactAdd(u) => {
                stats.adds += 1;
                stats.record_add(&u);
                if u.classifier.is_some() {
                    stats.filtered_classifier += 1;
                } else {
                    let rec = OutputRecord::Add {
                        group_id: &u.group_id,
                        artifact_id: &u.artifact_id,
                        version: &u.version,
                        extension: u.extension.as_deref(),
                    };
                    serde_json::to_writer(&mut stdout, &rec)?;
                    stdout.write_all(b"\n")?;
                    stats.emitted += 1;
                }
            }
            Record::ArtifactRemove(u) => {
                stats.removes += 1;
                if u.classifier.is_some() {
                    stats.filtered_classifier += 1;
                } else if args.include_removes {
                    let rec = OutputRecord::Remove {
                        group_id: &u.group_id,
                        artifact_id: &u.artifact_id,
                        version: &u.version,
                        extension: u.extension.as_deref(),
                    };
                    serde_json::to_writer(&mut stdout, &rec)?;
                    stdout.write_all(b"\n")?;
                    stats.emitted += 1;
                }
            }
        }
    }
    stdout.flush()?;
    let elapsed = start.elapsed();

    if args.stats {
        let mut err = io::stderr().lock();
        writeln!(
            err,
            "parsed {} documents in {}ms",
            stats.total,
            elapsed.as_millis()
        )?;
        writeln!(err, "  adds:       {}", stats.adds)?;
        writeln!(err, "  removes:    {}", stats.removes)?;
        writeln!(err, "  descriptor: {}", stats.descriptor)?;
        writeln!(err, "  allGroups:  {}", stats.all_groups)?;
        writeln!(err, "  rootGroups: {}", stats.root_groups)?;
        writeln!(err, "  unknown:    {}", stats.unknown)?;
        writeln!(
            err,
            "emitted {} records (filtered {} by classifier != NA)",
            stats.emitted, stats.filtered_classifier
        )?;
        writeln!(err, "errors: {}", stats.errors)?;
        writeln!(err, "first 10 UINFO (adds):")?;
        for line in &stats.first_uinfo_adds {
            writeln!(err, "  {line}")?;
        }
    }

    Ok(())
}
