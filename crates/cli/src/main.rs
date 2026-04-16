use std::fs::File;
use std::io::{self, BufRead, BufReader, BufWriter, Read, Write};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use clap::Parser;
use flate2::read::GzDecoder;
use serde::Serialize;
use sluice::{IndexReader, Record, Uinfo};

/// Number of artifact-add UINFO strings sampled into [`Stats::first_uinfo_adds`].
const SAMPLE_SIZE: usize = 10;

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

    /// Emit all records including classified artifacts (sources, javadoc, etc.)
    /// with their classifier and extension. Default: only classifier=NA records.
    #[arg(long)]
    full: bool,

    /// Print summary stats to stderr at end of run.
    #[arg(long)]
    stats: bool,
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "lowercase")]
enum RecordKind {
    Add,
    Remove,
}

#[derive(Serialize)]
struct OutputRecord<'a> {
    #[serde(rename = "type")]
    kind: RecordKind,
    group_id: &'a str,
    artifact_id: &'a str,
    version: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    classifier: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    extension: Option<&'a str>,
}

impl<'a> OutputRecord<'a> {
    fn from_uinfo(uinfo: &'a Uinfo, kind: RecordKind) -> Self {
        OutputRecord {
            kind,
            group_id: &uinfo.group_id,
            artifact_id: &uinfo.artifact_id,
            version: &uinfo.version,
            classifier: uinfo.classifier.as_deref(),
            extension: uinfo.extension.as_deref(),
        }
    }
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
    first_uinfo_adds: Vec<String>,
}

impl Stats {
    fn record_add(&mut self, u: &Uinfo) {
        if self.first_uinfo_adds.len() < SAMPLE_SIZE {
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

    fn print_summary(&self, elapsed: Duration) -> Result<()> {
        let mut err = io::stderr().lock();
        writeln!(
            err,
            "parsed {} documents in {}ms",
            self.total,
            elapsed.as_millis()
        )?;
        writeln!(err, "  adds:       {}", self.adds)?;
        writeln!(err, "  removes:    {}", self.removes)?;
        writeln!(err, "  descriptor: {}", self.descriptor)?;
        writeln!(err, "  allGroups:  {}", self.all_groups)?;
        writeln!(err, "  rootGroups: {}", self.root_groups)?;
        writeln!(err, "  unknown:    {}", self.unknown)?;
        writeln!(
            err,
            "emitted {} records (filtered {} by classifier != NA)",
            self.emitted, self.filtered_classifier
        )?;
        writeln!(err, "first {SAMPLE_SIZE} UINFO (adds):")?;
        for line in &self.first_uinfo_adds {
            writeln!(err, "  {line}")?;
        }
        Ok(())
    }
}

/// Open the input file (or stdin) as a buffered reader ready for streaming.
///
/// `io::stdin().lock()` already buffers internally, so we hand it back
/// unwrapped; files are wrapped in a `BufReader` here so callers don't need
/// to remember to do it themselves.
fn open_input(path: Option<&Path>) -> Result<Box<dyn BufRead>> {
    match path {
        Some(p) => {
            let file = File::open(p).with_context(|| format!("opening {}", p.display()))?;
            Ok(Box::new(BufReader::new(file)))
        }
        None => Ok(Box::new(io::stdin().lock())),
    }
}

/// Write a JSON Lines record (object + trailing newline).
fn write_jsonl(out: &mut impl Write, rec: &OutputRecord<'_>) -> Result<()> {
    serde_json::to_writer(&mut *out, rec)?;
    out.write_all(b"\n")?;
    Ok(())
}

/// Parse the index from `input`, write JSON Lines to `out`, and update `stats`.
///
/// `stats` is updated incrementally so the caller can inspect partial progress
/// even if processing fails partway through.
fn process<R, W>(input: R, out: &mut W, args: &Args, stats: &mut Stats) -> Result<()>
where
    R: Read,
    W: Write,
{
    let index = IndexReader::new(input).context("reading index header")?;

    for item in index {
        let doc = item.context("parsing document")?;
        stats.total += 1;

        let record = Record::try_from(&doc).context("classifying document")?;

        match record {
            Record::Descriptor => stats.descriptor += 1,
            Record::AllGroups => stats.all_groups += 1,
            Record::RootGroups => stats.root_groups += 1,
            Record::ArtifactAdd(ref u) => {
                stats.adds += 1;
                stats.record_add(u);
                if !args.full && u.classifier.is_some() {
                    stats.filtered_classifier += 1;
                } else {
                    write_jsonl(out, &OutputRecord::from_uinfo(u, RecordKind::Add))?;
                    stats.emitted += 1;
                }
            }
            Record::ArtifactRemove(ref u) => {
                stats.removes += 1;
                if !args.full && u.classifier.is_some() {
                    stats.filtered_classifier += 1;
                } else if args.include_removes {
                    write_jsonl(out, &OutputRecord::from_uinfo(u, RecordKind::Remove))?;
                    stats.emitted += 1;
                }
            }
            // `Record` is `#[non_exhaustive]`; future variants count as unknown.
            _ => stats.unknown += 1,
        }
    }
    out.flush()?;
    Ok(())
}

fn init_tracing() {
    let filter = match std::env::var("RUST_LOG") {
        Ok(value) => match tracing_subscriber::EnvFilter::try_new(&value) {
            Ok(f) => f,
            Err(e) => {
                // Surface bad RUST_LOG values rather than silently downgrading
                // to the default — typos here are easy to miss.
                let _ = writeln!(
                    io::stderr(),
                    "warning: ignoring invalid RUST_LOG={value:?}: {e}"
                );
                tracing_subscriber::EnvFilter::new("warn")
            }
        },
        Err(_) => tracing_subscriber::EnvFilter::new("warn"),
    };
    tracing_subscriber::fmt()
        .with_writer(io::stderr)
        .with_env_filter(filter)
        .init();
}

/// Restore the default SIGPIPE disposition so that `sluice | head` exits
/// silently instead of surfacing a `BrokenPipe` error. Rust ignores SIGPIPE
/// by default, turning pipe closures into write errors.
#[cfg(unix)]
fn reset_sigpipe() {
    // SAFETY: `signal(2)` is async-signal-safe and `SIGPIPE`/`SIG_DFL` are
    // valid POSIX constants. The returned previous handler is intentionally
    // discarded; we never restore it.
    unsafe {
        libc::signal(libc::SIGPIPE, libc::SIG_DFL);
    }
}

#[cfg(not(unix))]
fn reset_sigpipe() {}

fn main() -> Result<()> {
    reset_sigpipe();
    init_tracing();
    let args = Args::parse();

    let input = open_input(args.input.as_deref())?;
    let gz = GzDecoder::new(input);
    let buffered = BufReader::new(gz);

    let mut stdout = BufWriter::new(io::stdout().lock());
    let mut stats = Stats::default();
    let start = Instant::now();
    let result = process(buffered, &mut stdout, &args, &mut stats);
    let elapsed = start.elapsed();

    // Print stats even if processing failed, so the user sees the partial
    // progress recorded up to the failure point. A broken-pipe (or other I/O
    // error) on stderr while writing stats must not override the real
    // processing result; we deliberately swallow it here.
    if args.stats {
        let _ = stats.print_summary(elapsed);
    }

    result
}
