
use machine_core::types::typescript_types::{
    InterfacingProtocols, State, Subscriptions,
    SwarmProtocolType,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeSet,
    fs::{File, create_dir_all},
    io::prelude::*,
    path::{Path, PathBuf},
};
use clap::Parser;
use walkdir::WalkDir;

pub const BENCHMARK_DIR: &str = "./bench_and_results";
pub const SPECIAL_SYMBOL: &str = "done-special-symbol";

#[derive(Parser)]
#[command(version, about, long_about = None)]
pub struct Cli {
    // Sets a custom config file
    #[arg(short, long)]
    pub input_dir: PathBuf,

    #[arg(short, long)]
    pub output_dir: PathBuf,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct BenchMarkInput {
    pub state_space_size: usize,
    pub number_of_edges: usize,
    pub interfacing_swarms: InterfacingProtocols,
}

// TODO: give this type a 'Method' field that is either a Granularity or 'Exact'.
// Use this instead of inspecting file name later.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct BenchmarkSubSizeOutput {
    pub state_space_size: usize,
    pub number_of_edges: usize,
    pub subscriptions: Subscriptions,
}

// The two types below are used for comparing sizes of subscriptions generated using
// the `Behavioural Types for Local-First Software` notion of well-formedness
// with subscription generated using the compositional notion.
// A SimpleProtoBenchmark contains a single protocol without concurrency.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct SimpleProtoBenchMarkInput {
    pub state_space_size: usize,
    pub number_of_edges: usize,
    // We reuse the old benchmark suite for now.
    // This means we flatten a benchmark sample consisting of a number of protocols,
    // to a number of indiviual samples. Then multiple samples will possibly have
    // same number of states and transitions --> give a unique id to each sample somehow.
    pub id: Option<String>,
    pub proto: SwarmProtocolType,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Version {
    KMT23,                   // Kuhn, Melgratti, Tuosto 23
    CompositionalExact,      // expand protocol and compute subscription
    CompositionalOverapprox, // overapproximated well formed -- 'Algorithm 1'
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct SimpleProtoBenchmarkSubSizeOutput {
    pub state_space_size: usize,
    pub number_of_edges: usize,
    pub id: String,
    pub subscriptions: Subscriptions,
    pub version: Version,
}

/// Transform a file containing a benchmark input to a BenchmarkInput. Return the number
/// of states in the composition of the protocols in the input and the BenchMarkInput.
pub fn prepare_input(path: &Path) -> (usize, BenchMarkInput) {
    // Create a path to the desired file
    let display = path.display();

    // Open the path in read-only mode, returns `io::Result<File>`
    let mut file = match File::open(&path) {
        Err(why) => panic!("couldn't open {}: {}", display, why),
        Ok(file) => file,
    };

    // Read the file contents into a string, returns `io::Result<usize>`
    let mut protos = String::new();
    match file.read_to_string(&mut protos) {
        Err(why) => panic!("couldn't read {}: {}", display, why),
        Ok(_) => (),
    }
    let (state_space_size, interfacing_swarms) =
        match serde_json::from_str::<BenchMarkInput>(&protos) {
            Ok(input) => (input.state_space_size, input),
            Err(e) => panic!("error parsing input file: {}", e),
        };
    (state_space_size, interfacing_swarms)
}

pub fn prepare_files_in_directory(directory: &Path) -> Vec<(usize, BenchMarkInput)> {
    let mut inputs: Vec<(usize, BenchMarkInput)> = vec![];

    for entry in WalkDir::new(directory) {
        match entry {
            Ok(entry) => {
                if entry.file_type().is_file() {
                    inputs.push(prepare_input(
                        entry.path(),
                    ));
                }
            }
            Err(e) => panic!("error: {}", e),
        };
    }

    inputs
}

pub fn benchmark_input_to_simple_input(
    benchmark_input: BenchMarkInput,
) -> Vec<SimpleProtoBenchMarkInput> {
    let proto_to_simple_benchmark_input = |proto: SwarmProtocolType| -> SimpleProtoBenchMarkInput {
        let mut states: Vec<State> = proto
            .transitions
            .iter()
            .flat_map(|label| vec![label.source.clone(), label.target.clone()])
            .collect();
        states.push(proto.initial.clone());
        let state_space_size: usize = BTreeSet::from_iter(states.into_iter()).len();
        let number_of_edges = proto.transitions.len();

        SimpleProtoBenchMarkInput {
            state_space_size,
            number_of_edges,
            id: None,
            proto: proto,
        }
    };
    benchmark_input
        .interfacing_swarms
        .0
        .into_iter()
        .map(proto_to_simple_benchmark_input)
        .collect()
}

pub fn prepare_simple_inputs_in_directory(path: &Path) -> Vec<SimpleProtoBenchMarkInput> {
    let mut inputs: Vec<SimpleProtoBenchMarkInput> = vec![];

    for entry in WalkDir::new(path) {
        match entry {
            Ok(entry) => {
                if entry.file_type().is_file() {
                    let (_, benchmark_input) =
                        prepare_input(entry.path());
                    inputs.append(&mut benchmark_input_to_simple_input(benchmark_input));
                }
            }
            Err(e) => panic!("error: {}", e),
        };
    }
    let make_id = |state_space_size: usize, number_of_edges: usize, index: usize| -> String {
        format!(
            "{:0>10}_{:0>10}_{:0>2}",
            state_space_size, number_of_edges, index
        )
    };
    inputs
        .into_iter()
        .enumerate()
        .map(
            |(index, simple_benchmark_input)| SimpleProtoBenchMarkInput {
                id: Some(make_id(
                    simple_benchmark_input.state_space_size,
                    simple_benchmark_input.number_of_edges,
                    index,
                )),
                ..simple_benchmark_input
            },
        )
        .collect()
}

pub fn create_directory(path: &Path) -> () {
    match create_dir_all(path) {
        Ok(_) => (),
        Err(ref e) if e.kind() == std::io::ErrorKind::AlreadyExists => (),
        Err(e) => panic!("couldn't create directory {}: {}", path.display(), e),
    }
}

pub fn write_file(path: &Path, content: String) -> () {
    let display = path.display();

    // Open a file in write-only mode, returns `io::Result<File>`
    let mut file = match File::create(&path) {
        Err(why) => panic!("couldn't create {}: {}", display, why),
        Ok(file) => file,
    };

    match file.write_all(content.as_bytes()) {
        Err(why) => panic!("couldn't write to {}: {}", display, why),
        Ok(_) => (),
    }
}

pub fn wrap_and_write_sub_out(
    bench_input: &BenchMarkInput,
    subscriptions: Subscriptions,
    granularity: String,
    parent_path: &Path,
) {
    let out = BenchmarkSubSizeOutput {
        state_space_size: bench_input.state_space_size,
        number_of_edges: bench_input.number_of_edges,
        subscriptions: subscriptions,
    };
    let file_name = format!(
        "{:010}_{}.json",
        bench_input.state_space_size, granularity
    );
    let out = serde_json::to_string(&out).unwrap();
    write_file(&parent_path.join(file_name), out);
}

pub fn wrap_and_write_sub_out_simple(
    bench_input: &SimpleProtoBenchMarkInput,
    subscriptions: Subscriptions,
    version: Version,
    parent_path: &Path,
) {
    let id = bench_input.id.clone().unwrap_or(String::from("N/A"));
    let out = SimpleProtoBenchmarkSubSizeOutput {
        state_space_size: bench_input.state_space_size,
        number_of_edges: bench_input.number_of_edges,
        id,
        subscriptions: subscriptions,
        version: version,
    };
    let file_name = format!(
        "{}_{}.json",
        out.id,
        serde_json::to_string(&out.version)
            .unwrap()
            .replace("\"", "")
    );
    let out = serde_json::to_string(&out).unwrap();
    write_file(&parent_path.join(file_name), out);
}

/* 
#[test]
#[ignore]
fn write_flattened() {
    let input_dir = format!("{BENCHMARK_DIR}/benchmarks/general_pattern/");
    let output_dir = format!("{BENCHMARK_DIR}/benchmarks/general_pattern_flattened");
    create_directory(&output_dir);
    let inputs = prepare_simple_inputs_in_directory(input_dir);

    for input in inputs.iter() {
        //let id = input.id.clone().unwrap_or(String::from("N/A"));
        let file_name = format!(
            "{output_dir}/{}.json",
            input.id.clone().unwrap_or(String::from("NA"))
        );
        write_file(&file_name, serde_json::to_string(input).unwrap());
    }
} */
