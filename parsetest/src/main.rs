/// Regression testing tool for csdemoparser.
use clap::{Parser, Subcommand};
use console::style;
use indicatif::{ParallelProgressIterator, ProgressBar, ProgressStyle};
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use serde_json::Value;
use std::fs::File;
use std::io::{BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};
use std::sync::Mutex;

use parsetest::diff::match_json;

#[derive(Parser)]
#[command(version, about)]
struct Cli {
    /// Number of threads to use for running the demo parsers (defaults to number of logical CPUs).
    #[arg(long)]
    threads: Option<usize>,
    /// Path to the directory with the demos or to .dem file.
    #[arg(long)]
    demos: String,
    /// Path to the demo parser.
    #[arg(long, default_value_t=String::from("csdemoparser"))]
    parser: String,
    /// Limit number of replays parsed.
    #[arg(long)]
    limit: Option<usize>,

    /// Parse demos and compare the result to the data stored in the database.
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Parse the demos.
    Parse,
    /// Parse the demos and write the output to <demo>.parsed.json.
    Write,
    /// Diff parser output against <demo>.parsed.json.
    Diff,
}

fn main() -> anyhow::Result<ExitCode> {
    let cli = Cli::parse();
    let demos = if cli.demos.ends_with(".dem") {
        vec![PathBuf::from(cli.demos)]
    } else {
        let pattern = cli.demos + "/**/*.dem";
        let demo_iter = glob::glob(&pattern)
            .expect("failed to read glob pattern")
            .filter_map(|f| f.ok());
        let demos: Vec<_> = match cli.limit {
            Some(limit) => demo_iter.take(limit).collect(),
            None => demo_iter.collect(),
        };
        demos
    };
    if let Some(threads) = cli.threads {
        rayon::ThreadPoolBuilder::new()
            .num_threads(threads)
            .build_global()
            .unwrap();
    }
    let stats = Mutex::new(<TestStats as Default>::default());
    let total = demos.len() as u64;
    let bar = ProgressBar::new(total).with_style(
        ProgressStyle::with_template("{wide_bar} {msg} {pos:>}/{len} [{eta}]").unwrap(),
    );
    demos
        .par_iter()
        .progress_with(bar.clone())
        .for_each(|demo| {
            let result = parse_demo(cli.parser.as_str(), &cli.command, demo);
            let mut stats = stats.lock().unwrap();
            match result {
                TestResult::Ok => stats.ok += 1,
                TestResult::Fail => stats.fail += 1,
                TestResult::Diff => stats.diff += 1,
            }
            bar.set_message(stats.to_string());
        });
    bar.finish_and_clear();
    let stats = stats.lock().unwrap();
    println!("{}", stats);
    if stats.fail > 0 || stats.diff > 0 {
        Ok(ExitCode::FAILURE)
    } else {
        Ok(ExitCode::SUCCESS)
    }
}

enum TestResult {
    Ok,
    Fail,
    Diff,
}

#[derive(Default)]
pub struct TestStats {
    ok: usize,
    diff: usize,
    fail: usize,
}

impl std::fmt::Display for TestStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.ok > 0 {
            write!(f, "{} OK", style(self.ok).green())?;
        }
        if self.diff > 0 {
            write!(f, " {} DIFF", style(self.diff).green())?;
        }
        if self.fail > 0 {
            write!(f, " {} FAIL", style(self.fail).green())?;
        }
        Ok(())
    }
}

fn parse_demo(parser: &str, command: &Commands, demo: &Path) -> TestResult {
    let out = Command::new(parser)
        .arg(demo.to_string_lossy().to_string())
        .output()
        .expect("parser failed to start");
    if out.status.success() {
        match command {
            Commands::Parse => TestResult::Ok,
            Commands::Write => {
                write_parsed(demo, &out.stdout);
                TestResult::Ok
            }
            Commands::Diff => diff_parsed(demo, &out.stdout),
        }
    } else {
        println!("{} failed to parse", demo.display());
        std::io::stdout().write_all(&out.stderr).unwrap();
        TestResult::Fail
    }
}

fn write_parsed(demo: &Path, stdout: &[u8]) {
    let parsed_path = demo.with_extension("parsed.json");
    let mut writer =
        File::create(&parsed_path).unwrap_or_else(|e| panic!("{}: {}", parsed_path.display(), e));
    writer.write_all(stdout).unwrap();
}

fn diff_parsed(demo: &Path, stdout: &[u8]) -> TestResult {
    let actual: Value = serde_json::from_slice(stdout).expect("parser output should be json");
    let parsed_path = demo.with_extension("parsed.json");
    let reader = match File::open(&parsed_path) {
        Ok(reader) => BufReader::new(reader),
        Err(_) => {
            println!("{} does not exist\n", parsed_path.display());
            return TestResult::Diff;
        }
    };
    let expected: Value = serde_json::from_reader(reader).expect("expected output should be json");
    if let Err(msg) = match_json(&actual, &expected) {
        println!("{} has diff:\n{}", demo.display(), msg);
        TestResult::Diff
    } else {
        TestResult::Ok
    }
}
#[cfg(test)]
mod tests {
    use super::Cli;

    #[test]
    fn verify_cli() {
        use clap::CommandFactory;
        Cli::command().debug_assert()
    }
}
