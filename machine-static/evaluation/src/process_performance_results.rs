use evaluation::{BenchMarkInput, Cli, create_directory, prepare_files_in_directory};
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use walkdir::WalkDir;
use std::{collections::BTreeMap, ffi::OsStr, fs::File, path::{Path, PathBuf}};
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
    function_id: Option<String>,    // Name of benchmarked function e.g. Exact or Algorithm 1
    value_str: Option<String>,      // Name of input to benchmarked function, for us, the number of states in the composition.
    measurement_fname: PathBuf,      // File containing statistics
    saved_statistics: SavedStatistics
}

#[derive(Debug, Serialize, Deserialize)]
enum Unit {
    NS,
    MUS
}


// Struct containing some of the fields from a NamedSavedStatistics + some info about input sample protocol used to generate it
// in a format we can easily write to csv.
#[derive(Debug, Serialize, Deserialize)]
struct FlattenedMeasurement {
    number_of_edges: usize,
    state_space_size: usize,
    algorithm: String,
    unit: Unit,
    
    mean_confidence_interval_lower_bound: f64,
    mean_confidence_interval_upper_bound: f64,
    mean_confidence_level: f64,
    mean_point_estimate: f64,
    mean_standard_error: f64,

    median_confidence_interval_lower_bound: f64,
    median_confidence_interval_upper_bound: f64,
    median_confidence_level: f64,
    median_point_estimate: f64,
    median_standard_error: f64,

    median_abs_dev_confidence_interval_lower_bound: f64,
    median_abs_dev_confidence_interval_upper_bound: f64,
    median_abs_dev_confidence_level: f64,
    median_abs_dev_point_estimate: f64,
    median_abs_dev_standard_error: f64,

    std_dev_confidence_interval_lower_bound: f64, 
    std_dev_confidence_interval_upper_bound: f64,
    std_dev_confidence_level: f64,
    std_dev_point_estimate: f64, 
    std_dev_standard_error: f64,
}

fn flatten_measurement(named_saved_statistics: NamedSavedStatistics, benchmark_input: &BenchMarkInput, unit: Unit) -> Option<FlattenedMeasurement> {
    let number_of_edges = benchmark_input.number_of_edges;
    let state_space_size: usize = named_saved_statistics.value_str?.parse().ok()?;
    //let state_space_size: usize = state_space_size_str.parse().ok()?;
    let algorithm = named_saved_statistics.function_id?;

    let mean_confidence_interval_lower_bound = named_saved_statistics.saved_statistics.estimates.mean.confidence_interval.lower_bound;
    let mean_confidence_interval_upper_bound = named_saved_statistics.saved_statistics.estimates.mean.confidence_interval.upper_bound;
    let mean_confidence_level = named_saved_statistics.saved_statistics.estimates.mean.confidence_interval.confidence_level;
    let mean_point_estimate = named_saved_statistics.saved_statistics.estimates.mean.point_estimate;
    let mean_standard_error = named_saved_statistics.saved_statistics.estimates.mean.standard_error;

    let median_confidence_interval_lower_bound = named_saved_statistics.saved_statistics.estimates.median.confidence_interval.lower_bound;
    let median_confidence_interval_upper_bound = named_saved_statistics.saved_statistics.estimates.median.confidence_interval.upper_bound;
    let median_confidence_level = named_saved_statistics.saved_statistics.estimates.median.confidence_interval.confidence_level;
    let median_point_estimate = named_saved_statistics.saved_statistics.estimates.median.point_estimate;
    let median_standard_error = named_saved_statistics.saved_statistics.estimates.median.standard_error;

    let median_abs_dev_confidence_interval_lower_bound = named_saved_statistics.saved_statistics.estimates.median_abs_dev.confidence_interval.lower_bound;
    let median_abs_dev_confidence_interval_upper_bound = named_saved_statistics.saved_statistics.estimates.median_abs_dev.confidence_interval.upper_bound;
    let median_abs_dev_confidence_level = named_saved_statistics.saved_statistics.estimates.median_abs_dev.confidence_interval.confidence_level;
    let median_abs_dev_point_estimate = named_saved_statistics.saved_statistics.estimates.median_abs_dev.point_estimate;
    let median_abs_dev_standard_error = named_saved_statistics.saved_statistics.estimates.median_abs_dev.standard_error;

    let std_dev_confidence_interval_lower_bound = named_saved_statistics.saved_statistics.estimates.std_dev.confidence_interval.lower_bound;
    let std_dev_confidence_interval_upper_bound = named_saved_statistics.saved_statistics.estimates.std_dev.confidence_interval.upper_bound;
    let std_dev_confidence_level = named_saved_statistics.saved_statistics.estimates.std_dev.confidence_interval.confidence_level;
    let std_dev_point_estimate = named_saved_statistics.saved_statistics.estimates.std_dev.point_estimate;
    let std_dev_standard_error = named_saved_statistics.saved_statistics.estimates.std_dev.standard_error;

    Some(
        FlattenedMeasurement {
            number_of_edges,
            state_space_size,
            algorithm,
            unit,

            mean_confidence_interval_lower_bound,
            mean_confidence_interval_upper_bound,
            mean_confidence_level,
            mean_point_estimate,
            mean_standard_error,

            median_confidence_interval_lower_bound,
            median_confidence_interval_upper_bound,
            median_confidence_level,
            median_point_estimate,
            median_standard_error,

            median_abs_dev_confidence_interval_lower_bound,
            median_abs_dev_confidence_interval_upper_bound,
            median_abs_dev_confidence_level,
            median_abs_dev_point_estimate,
            median_abs_dev_standard_error,

            std_dev_confidence_interval_lower_bound,
            std_dev_confidence_interval_upper_bound,
            std_dev_confidence_level,
            std_dev_point_estimate,
            std_dev_standard_error
        }
    )
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

// Benchmark path should be the path of a "benchmark.cbor" file
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
        NamedSavedStatistics { 
            function_id: benchmark_record.id.function_id, 
            value_str: benchmark_record.id.value_str,
            measurement_fname: measurement_path,
            saved_statistics: saved_stats 
        }
    )
}

fn write_csv<S: Serialize> (processed: Vec<S>, output_file: &Path) -> Result<()> {
    let mut wtr = csv::Writer::from_path(output_file)?;

    for record in processed {
        wtr.serialize(record)?;
    }
    wtr.flush()?;
    Ok(())
}

// Read all benchmark samples at path and return them as a map from number of states to samples
fn benchmark_inputs(path: &Path) -> BTreeMap<usize, BenchMarkInput> {
    prepare_files_in_directory(path)
        .into_iter()
        .collect()
}

// Transform a vec of NamedSavedStatistics into a vec of FlattenedMeasurements
/* fn flatten_measurements(measurements: Vec<NamedSavedStatistics>, benchmarks: BTreeMap<usize, BenchMarkInput>, unit: Unit) -> Result<Vec<FlattenedMeasurement>> {
    let mapper = |named_saved_statistics: NamedSavedStatistics| -> Result<FlattenedMeasurement> {
        let state_space_size: usize = named_saved_statistics.value_str?.parse().ok()?;
        let benchmark_input = benchmarks.get(&state_space_size).ok()?;
        unimplemented!()
        //Ok()

    };
    
    measurements
        .into_iter()
        .map(mapper)
        .collect()   
}
 */


fn main() {
    let cli = Cli::parse();
    let input_dir = cli.input_dir;
    let output_dir = cli.output_dir;

    let measurements = load_latest_measurements(&input_dir);
    if measurements.is_ok() {
        let unwrapped = measurements.unwrap();
        println!("Length: {}", unwrapped.len());
        for m in unwrapped {
            println!("{:#?} {:#?} {:#?} {}", m.function_id, m.value_str, m.measurement_fname.display(), m.saved_statistics.estimates.mean.point_estimate)
        }
    }
    //create_directory(&output_dir);
}