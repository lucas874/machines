use evaluation::{Cli, create_directory};
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use walkdir::WalkDir;
use std::{ffi::OsStr, fs::File, path::{Path, PathBuf}};
use clap::Parser;
use anyhow::{Context, Error, Result};

// From https://github.com/bheisler/cargo-criterion/blob/main/src/estimate.rs
#[derive(Clone, PartialEq, Deserialize, Serialize, Debug)]
pub struct ConfidenceInterval {
    pub confidence_level: f64,
    pub lower_bound: f64,
    pub upper_bound: f64,
}

#[derive(Clone, PartialEq, Deserialize, Serialize, Debug)]
pub struct Estimate {
    /// The confidence interval for this estimate
    pub confidence_interval: ConfidenceInterval,
    pub point_estimate: f64,
    /// The standard error of this estimate
    pub standard_error: f64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Estimates {
    pub mean: Estimate,
    pub median: Estimate,
    pub median_abs_dev: Estimate,
    pub slope: Option<Estimate>,
    pub std_dev: Estimate,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ChangeEstimates {
    pub mean: Estimate,
    pub median: Estimate,
}

// From https://github.com/bheisler/cargo-criterion/blob/main/src/connection.rs
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum Throughput {
    Bytes(u64),
    BytesDecimal(u64),
    Elements(u64),
}

// From https://github.com/bheisler/cargo-criterion/blob/main/src/model.rs
#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum ChangeDirection {
    NoChange,
    NotSignificant,
    Improved,
    Regressed,
}

// From https://github.com/bheisler/cargo-criterion/blob/main/src/model.rs
// Data stored on disk when running benchmarks
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SavedStatistics {
    // The timestamp of when these measurements were saved.
    pub datetime: DateTime<Utc>,
    // The number of iterations in each sample
    pub iterations: Vec<f64>,
    // The measured values from each sample
    pub values: Vec<f64>,
    // The average values from each sample, ie. values / iterations
    pub avg_values: Vec<f64>,
    // The statistical estimates from this run
    pub estimates: Estimates,
    // The throughput of this run
    pub throughput: Option<Throughput>,
    // The statistical differences compared to the last run. We save these so we don't have to
    // recompute them later for the history report.
    pub changes: Option<ChangeEstimates>,
    // Was the change (if any) significant?
    pub change_direction: Option<ChangeDirection>,

    // An optional user-provided identifier string. This might be a version control commit ID or
    // something custom
    pub history_id: Option<String>,
    // An optional user-provided description. This might be a version control commit message or
    // something custom.
    pub history_description: Option<String>,
}

// From https://github.com/bheisler/cargo-criterion/blob/main/src/model.rs
#[derive(Debug, Deserialize, Serialize)]
pub struct SavedBenchmarkId {
    group_id: String,
    function_id: Option<String>,
    value_str: Option<String>,
    throughput: Option<Throughput>,
}

// From https://github.com/bheisler/cargo-criterion/blob/main/src/model.rs
#[derive(Debug, Serialize, Deserialize)]
struct BenchmarkRecord {
    id: SavedBenchmarkId,
    latest_record: PathBuf,
}


#[derive(Debug, Serialize, Deserialize)]
struct NamedSavedStatistics {
    function_id: Option<String>,    // Name of benchmarked function e.g. Exact or Algorith 1
    value_str: Option<String>,      // Name of input to benchmarked function, for us, the number of states in the composition.
    saved_statistics: SavedStatistics
}

// Should be called with a path contatining criterion data directories (directories containing cbors with benchmark results). 
// Adapted from https://github.com/bheisler/cargo-criterion/blob/main/src/model.rs/: load impl for Model
fn load_latest_measurements(path: &Path) -> Result<Vec<NamedSavedStatistics>> {
    let mut stats = Vec::new();
    for entry in WalkDir::new(path)
        .into_iter()
        // Ignore errors.
        .filter_map(::std::result::Result::ok)
        .filter(|entry| entry.file_name() == OsStr::new("benchmark.cbor"))
    {   
        let statistics = load_latest(entry.path())?;
        stats.push(statistics);
    }

    Ok(stats)
}

// benchmark path is the path of a "benchmark.cbor" file
// Adapted from  https://github.com/bheisler/cargo-criterion/blob/main/src/model.rs/: load_stored_benchmark impl for model
fn load_latest(benchmark_path: &Path) -> Result<NamedSavedStatistics> {
    if !benchmark_path.is_file() {
        return Err(Error::msg(format!("Invalid benchmark_path: {} is not a file", benchmark_path.display())));
    }
    let mut benchmark_file = File::open(benchmark_path)
        .with_context(|| format!("Failed to open benchmark file {:?}", benchmark_path))?;
    let benchmark_record: BenchmarkRecord = serde_cbor::from_reader(&mut benchmark_file)
        .with_context(|| format!("Failed to read benchmark file {:?}", benchmark_path))?;

    let measurement_path = benchmark_path.with_file_name(benchmark_record.latest_record);
    if !measurement_path.is_file() {
        return Err(Error::msg(format!("Error: latest measurement {} is not a file", measurement_path.display())));
    }
    let mut measurement_file = File::open(&measurement_path)
        .with_context(|| format!("Failed to open measurement file {:?}", measurement_path))?;
    let saved_stats: SavedStatistics = serde_cbor::from_reader(&mut measurement_file)
        .with_context(|| format!("Failed to read measurement file {:?}", measurement_path))?;

    return Ok(
        NamedSavedStatistics { function_id: benchmark_record.id.function_id, value_str: benchmark_record.id.value_str, saved_statistics: saved_stats }
    )
}


// open a directory expecting to find a benchmark.cbor and a number of 
// measurement_<some_number>.cbor. Inspect benchmark.cbor to find the latest 
// measurement and the name of the benchmarked function. Read the latest and return
// a NamedSavedStatistics.
/* fn load_latest(dir: &Path) -> Result<NamedSavedStatistics> {
    fn load_measurement_from(measurement_path: &Path) -> Result<SavedStatistics> {
        let mut measurement_file = File::open(measurement_path).with_context(|| {
            format!("Failed to open measurement file {:?}", measurement_path)
        })?;
        serde_cbor::from_reader(&mut measurement_file)
            .with_context(|| format!("Failed to read measurement file {:?}", measurement_path))
    }
    fn load_benchmark_from(measurement_path: &Path) -> Result<BenchmarkRecord> {
        let mut measurement_file = File::open(measurement_path).with_context(|| {
            format!("Failed to open benchmark file {:?}", measurement_path)
        })?;
        serde_cbor::from_reader(&mut measurement_file)
            .with_context(|| format!("Failed to read benchmark file {:?}", measurement_path))
    } */
    

/*             for entry in WalkDir::new(&model.data_directory)
            .into_iter()
            // Ignore errors.
            .filter_map(::std::result::Result::ok)
            .filter(|entry| entry.file_name() == OsStr::new("benchmark.cbor"))
        {
            if let Err(e) = model.load_stored_benchmark(entry.path()) {
                error!("Encountered error while loading stored data: {}", e)
            }
        } */

    /* for entry in WalkDir::new(dir)
        .max_depth(1)
        .into_iter()
        // Ignore errors.
        .filter_map(::std::result::Result::ok)
    {
        let name_str = entry.file_name().to_string_lossy();
        if name_str.starts_with("benchmark") && name_str.ends_with(".cbor") {
            match load_benchmark_from(entry.path()) {
                Ok(saved_stats) => stats.push(saved_stats),
                Err(e) => return Err(Error::msg(format!(
                    "Unexpected error loading benchmark history from file {}: {:?}",
                    entry.path().display(),
                    e
                ))),
            }
        }
    } */

/* 

    unimplemented!()
} */


// From https://github.com/bheisler/cargo-criterion/blob/main/src/model.rs
fn load_history(dir: &Path) -> Result<Vec<SavedStatistics>> {

    fn load_from(measurement_path: &Path) -> Result<SavedStatistics> {
        let mut measurement_file = File::open(measurement_path).with_context(|| {
            format!("Failed to open measurement file {:?}", measurement_path)
        })?;
        serde_cbor::from_reader(&mut measurement_file)
            .with_context(|| format!("Failed to read measurement file {:?}", measurement_path))
    }

    let mut stats = Vec::new();
    for entry in WalkDir::new(dir)
        .max_depth(1)
        .into_iter()
        // Ignore errors.
        .filter_map(::std::result::Result::ok)
    {
        let name_str = entry.file_name().to_string_lossy();
        if name_str.starts_with("measurement_") && name_str.ends_with(".cbor") {
            match load_from(entry.path()) {
                Ok(saved_stats) => stats.push(saved_stats),
                Err(e) => return Err(Error::msg(format!(
                    "Unexpected error loading benchmark history from file {}: {:?}",
                    entry.path().display(),
                    e
                )))/* error!(
                    "Unexpected error loading benchmark history from file {}: {:?}",
                    entry.path().display(),
                    e
                ) */,
            }
        }
    }

    stats.sort_unstable_by_key(|st| st.datetime);

    Ok(stats)
}

fn main() {
    let cli = Cli::parse();
    let input_dir = cli.input_dir;
    let output_dir = cli.output_dir;

    let measurements = load_latest_measurements(&input_dir);
    if measurements.is_ok() {
        let unwrapped = measurements.unwrap();
        println!("Length: {}", unwrapped.len());
        for m in unwrapped {
            println!("{:#?} {:#?} {}", m.function_id, m.value_str, m.saved_statistics.estimates.mean.point_estimate)
        }
    }
    //create_directory(&output_dir);
}