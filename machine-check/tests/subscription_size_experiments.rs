use machine_check::{
    check_swarm, composition::{check_composed_swarm, composition_types::{Granularity, InterfacingProtocols, InterfacingSwarms}, exact_well_formed_sub, overapproximated_well_formed_sub}, types::{CheckResult, DataResult, EventType, Role, State}, well_formed_sub, Subscriptions, SwarmProtocolType
};

use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet}, fs::{create_dir_all, File}, path::Path, io::prelude::*
};

use walkdir::WalkDir;

const BENCHMARK_DIR: &str = "./bench_and_results";
const SPECIAL_SYMBOL: &str = "done-special-symbol";

fn to_interfacing_protocols(interfacing_swarms: InterfacingSwarms<Role>) -> InterfacingProtocols {
    InterfacingProtocols(interfacing_swarms
        .0
        .into_iter()
        .map(|cc| cc.protocol)
        .collect())
}

#[test]
#[ignore]
fn full_run_bench_sub_sizes_general() {
    let input_dir = format!("{BENCHMARK_DIR}/benchmarks/general_pattern/");
    let output_dir = format!("{BENCHMARK_DIR}/subscription_size_benchmarks/general_pattern");
    create_directory(&output_dir);
    let mut interfacing_swarms_general =
        prepare_files_in_directory(input_dir);
    interfacing_swarms_general.sort_by(|(size1, _), (size2, _)| size1.cmp(size2));
    let subs = serde_json::to_string(&BTreeMap::<Role, BTreeSet<EventType>>::new()).unwrap();
    let two_step_granularity = Granularity::TwoStep;

    for (_, bi) in interfacing_swarms_general.iter() {
        let swarms = &bi.interfacing_swarms;
        let subscriptions = match overapproximated_well_formed_sub(to_interfacing_protocols(swarms.clone()), subs.clone(), two_step_granularity.clone()) {
            DataResult::OK{data: subscriptions} => Some(subscriptions),
            DataResult::ERROR{ .. } => None,
        };
        wrap_and_write_sub_out(&bi, subscriptions.unwrap(), serde_json::to_string(&two_step_granularity).unwrap().replace("\"", ""), &output_dir);

        let subscriptions = match exact_well_formed_sub(to_interfacing_protocols(swarms.clone()), subs.clone()) {
            DataResult::OK{data: subscriptions} => {
                Some(subscriptions) },
            DataResult::ERROR{ .. } => None,
        };
        wrap_and_write_sub_out(&bi, subscriptions.unwrap(), String::from("Exact"), &output_dir);
        println!("{}", SPECIAL_SYMBOL);
    }
}

#[test]
#[ignore]
fn short_run_bench_sub_sizes_general() {
    let input_dir = format!("{BENCHMARK_DIR}/benchmarks/general_pattern/");
    let output_dir = format!("{BENCHMARK_DIR}/short_subscription_size_benchmarks/general_pattern");
    create_directory(&output_dir);
    let mut interfacing_swarms_general =
        prepare_files_in_directory(input_dir);
    interfacing_swarms_general.sort_by(|(size1, _), (size2, _)| size1.cmp(size2));
    let subs = serde_json::to_string(&BTreeMap::<Role, BTreeSet<EventType>>::new()).unwrap();
    let two_step_granularity = Granularity::TwoStep;
    let step: usize = 120;

    for (_, bi) in interfacing_swarms_general.iter().step_by(step) {
        let swarms = &bi.interfacing_swarms;
        let subscriptions = match overapproximated_well_formed_sub(to_interfacing_protocols(swarms.clone()), subs.clone(), two_step_granularity.clone()) {
            DataResult::OK{data: subscriptions} => Some(subscriptions),
            DataResult::ERROR{ .. } => None,
        };
        wrap_and_write_sub_out(&bi, subscriptions.unwrap(), serde_json::to_string(&two_step_granularity).unwrap().replace("\"", ""), &output_dir);

        let subscriptions = match exact_well_formed_sub(to_interfacing_protocols(swarms.clone()), subs.clone()) {
            DataResult::OK{data: subscriptions} => {
                Some(subscriptions) },
            DataResult::ERROR{ .. } => None,
        };
        wrap_and_write_sub_out(&bi, subscriptions.unwrap(), String::from("Exact"), &output_dir);
        println!("{}", SPECIAL_SYMBOL);
    }
}

#[test]
#[ignore]
fn full_simple_run_bench_sub_sizes_general() {
    let input_dir = format!("{BENCHMARK_DIR}/benchmarks/general_pattern/");
    let output_dir = format!("{BENCHMARK_DIR}/subscription_size_benchmarks/ecoop23-compositional-comparison/general_pattern");
    create_directory(&output_dir);
    let inputs =
        prepare_simple_inputs_in_directory(input_dir);
    let subs = serde_json::to_string(&BTreeMap::<Role, BTreeSet<EventType>>::new()).unwrap();

    for input in inputs.iter() {
        let subscriptions = match well_formed_sub(input.proto.clone(), subs.clone()) {
            DataResult::OK{data: subscriptions} => Some(subscriptions),
            DataResult::ERROR{ .. } => None,
        };
        match check_swarm(input.proto.clone(), serde_json::to_string(&subscriptions.clone().unwrap()).unwrap()) {
            CheckResult::OK => (),
            CheckResult::ERROR { errors } => { println!("id: {}, cause: {},\n subscriptions: {}",
            input.id.clone().unwrap_or(String::from("")), errors.join(", "), serde_json::to_string_pretty(&subscriptions.clone()).unwrap());
            panic!("Not ok compositional") }
        }
        wrap_and_write_sub_out_simple(&input, subscriptions.unwrap(), Version::KMT23, &output_dir);

        let subscriptions = match exact_well_formed_sub(InterfacingProtocols(vec![input.proto.clone()]), subs.clone()) {
            DataResult::OK{data: subscriptions} => {
                Some(subscriptions) },
            DataResult::ERROR{ .. } => None,
        };
        match check_composed_swarm(InterfacingProtocols(vec![input.proto.clone()]), serde_json::to_string(&subscriptions.clone().unwrap()).unwrap()) {
            CheckResult::OK => (),
            CheckResult::ERROR { errors } => { println!("id: {}, cause: {},\n subscriptions: {}",
            input.id.clone().unwrap_or(String::from("")), errors.join(", "), serde_json::to_string_pretty(&subscriptions.clone()).unwrap());
            panic!("Not ok compositional") }
        }
        wrap_and_write_sub_out_simple(&input, subscriptions.unwrap(), Version::Compositional, &output_dir);
        println!("{}", SPECIAL_SYMBOL);
    }
}

#[test]
#[ignore]
fn short_simple_run_bench_sub_sizes_general() {
    let input_dir = format!("{BENCHMARK_DIR}/benchmarks/general_pattern/");
    let output_dir = format!("{BENCHMARK_DIR}/short_subscription_size_benchmarks/ecoop23-compositional-comparison/general_pattern");
    create_directory(&output_dir);
    let inputs =
        prepare_simple_inputs_in_directory(input_dir);
    let subs = serde_json::to_string(&BTreeMap::<Role, BTreeSet<EventType>>::new()).unwrap();
    let step: usize = 120;
    for input in inputs.iter().step_by(step) {
        let subscriptions = match well_formed_sub(input.proto.clone(), subs.clone()) {
            DataResult::OK{data: subscriptions} => Some(subscriptions),
            DataResult::ERROR{ .. } => None,
        };
        match check_swarm(input.proto.clone(), serde_json::to_string(&subscriptions.clone().unwrap()).unwrap()) {
            CheckResult::OK => (),
            CheckResult::ERROR { errors } => { println!("id: {}, cause: {},\n subscriptions: {}",
            input.id.clone().unwrap_or(String::from("")), errors.join(", "), serde_json::to_string_pretty(&subscriptions.clone()).unwrap());
            panic!("Not ok compositional") }
        }
        wrap_and_write_sub_out_simple(&input, subscriptions.unwrap(), Version::KMT23, &output_dir);

        let subscriptions = match exact_well_formed_sub(InterfacingProtocols(vec![input.proto.clone()]), subs.clone()) {
            DataResult::OK{data: subscriptions} => {
                Some(subscriptions) },
            DataResult::ERROR{ .. } => None,
        };
        match check_composed_swarm(InterfacingProtocols(vec![input.proto.clone()]), serde_json::to_string(&subscriptions.clone().unwrap()).unwrap()) {
            CheckResult::OK => (),
            CheckResult::ERROR { errors } => { println!("id: {}, cause: {},\n subscriptions: {}",
            input.id.clone().unwrap_or(String::from("")), errors.join(", "), serde_json::to_string_pretty(&subscriptions.clone()).unwrap());
            panic!("Not ok compositional") }
        }
        wrap_and_write_sub_out_simple(&input, subscriptions.unwrap(), Version::Compositional, &output_dir);
        println!("{}", SPECIAL_SYMBOL);
    }
}

#[test]
#[ignore]
fn write_flattened() {
    let input_dir = format!("{BENCHMARK_DIR}/benchmarks/general_pattern/");
    let output_dir = format!("{BENCHMARK_DIR}/benchmarks/general_pattern_flattened");
    create_directory(&output_dir);
    let inputs =
        prepare_simple_inputs_in_directory(input_dir);

    for input in inputs.iter() {
        //let id = input.id.clone().unwrap_or(String::from("N/A"));
        let file_name = format!("{output_dir}/{}.json", input.id.clone().unwrap_or(String::from("NA")));
        write_file(&file_name, serde_json::to_string(input).unwrap());
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct BenchMarkInput  {
    pub state_space_size: usize,
    pub number_of_edges: usize,
    pub interfacing_swarms: InterfacingSwarms<Role>
}
// TODO: give this type a 'Method' field that is either a Granularity or 'Exact'.
// Use this instead of inspecting file name later.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct BenchmarkSubSizeOutput  {
    pub state_space_size: usize,
    pub number_of_edges: usize,
    pub subscriptions: Subscriptions,
}

// The two types below are used for comparing sizes of subscriptions generated using
// the `Behavioural Types for Local-First Software` notion of well-formedness
// with subscription generated using the compositional notion.
// A SimpleProtoBenchmark contains a single protocol without concurrency.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct SimpleProtoBenchMarkInput  {
    pub state_space_size: usize,
    pub number_of_edges: usize,
    // We reuse the old benchmark suite for now.
    // This means we flatten a benchmark sample consisting of a number of protocols,
    // to a number of indiviual samples. Then multiple samples will possibly have
    // same number of states and transitions --> give a unique id to each sample somehow.
    pub id: Option<String>,
    pub proto: SwarmProtocolType
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Version {
    KMT23, // Kuhn, Melgratti, Tuosto 23
    Compositional,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct SimpleProtoBenchmarkSubSizeOutput  {
    pub state_space_size: usize,
    pub number_of_edges: usize,
    pub id: String,
    pub subscriptions: Subscriptions,
    pub version: Version,
}

/// Transform a file containing a benchmark input to a BenchmarkInput. Return the number
/// of states in the composition of the protocols in the input and the BenchMarkInput.
fn prepare_input(file_name: String) -> (usize, BenchMarkInput) {
    // Create a path to the desired file
    let path = Path::new(&file_name);
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
    (
        state_space_size,
        interfacing_swarms
    )
}

fn prepare_files_in_directory(directory: String) -> Vec<(usize, BenchMarkInput)> {
    let mut inputs: Vec<(usize, BenchMarkInput)> = vec![];

    for entry in WalkDir::new(directory) {
        match entry {
            Ok(entry) => {
                if entry.file_type().is_file() {
                    inputs.push(prepare_input(
                        entry.path().as_os_str().to_str().unwrap().to_string(),
                    ));
                }
            }
            Err(e) => panic!("error: {}", e),
        };
    }

    inputs
}

fn benchmark_input_to_simple_input(benchmark_input: BenchMarkInput) -> Vec<SimpleProtoBenchMarkInput> {
    let proto_to_simple_benchmark_input = |proto: SwarmProtocolType| -> SimpleProtoBenchMarkInput {
        let mut states: Vec<State> = proto.transitions
            .iter()
            .flat_map(|label|
                vec![label.source.clone(), label.target.clone()])
            .collect();
        states.push(proto.initial.clone());
        let state_space_size: usize = BTreeSet::from_iter(states.into_iter()).len();
        let number_of_edges = proto.transitions.len();

        SimpleProtoBenchMarkInput { state_space_size, number_of_edges, id: None, proto: proto }
    };
    to_interfacing_protocols(benchmark_input.interfacing_swarms)
        .0
        .into_iter()
        .map(proto_to_simple_benchmark_input)
        .collect()

}

fn prepare_simple_inputs_in_directory(directory: String) -> Vec<SimpleProtoBenchMarkInput> {
    let mut inputs: Vec<SimpleProtoBenchMarkInput> = vec![];

    for entry in WalkDir::new(directory) {
        match entry {
            Ok(entry) => {
                if entry.file_type().is_file() {
                    let (_, benchmark_input) = prepare_input(
                        entry.path().as_os_str().to_str().unwrap().to_string());
                    inputs.append(&mut benchmark_input_to_simple_input(benchmark_input));
                }
            }
            Err(e) => panic!("error: {}", e),
        };
    }
    let make_id = |state_space_size: usize, number_of_edges: usize, index: usize| -> String {
        format!("{:0>10}_{:0>10}_{:0>2}", state_space_size, number_of_edges, index)
    };
    inputs
        .into_iter()
        .enumerate()
        .map(|(index, simple_benchmark_input)|
            SimpleProtoBenchMarkInput {
                id: Some(make_id(simple_benchmark_input.state_space_size, simple_benchmark_input.number_of_edges, index)),
                ..simple_benchmark_input})
        .collect()

}

fn create_directory(dir_name: &String) -> () {
    match create_dir_all(dir_name) {
        Ok(_) => (),
        Err(ref e) if e.kind() == std::io::ErrorKind::AlreadyExists => (),
        Err(e) => panic!("couldn't create directory {}: {}", dir_name, e),
    }
}

fn write_file(file_name: &String, content: String) -> () {
    let path = Path::new(&file_name);
    let display = path.display();

    // Open a file in write-only mode, returns `io::Result<File>`
    let mut file = match File::create(&path) {
        Err(why) => panic!("couldn't create {}: {}", display, why),
        Ok(file) => file,
    };

    match file.write_all(content.as_bytes()) {
        Err(why) => panic!("couldn't write to {}: {}", display, why),
        Ok(_) => ()
    }
}


fn wrap_and_write_sub_out(bench_input: &BenchMarkInput, subscriptions: Subscriptions, granularity: String, parent_path: &String) {
    let out = BenchmarkSubSizeOutput { state_space_size: bench_input.state_space_size, number_of_edges: bench_input.number_of_edges, subscriptions: subscriptions};
    let file_name = format!("{parent_path}/{:010}_{}.json", bench_input.state_space_size, granularity);
    let out = serde_json::to_string(&out).unwrap();
    write_file(&file_name, out);
}

fn wrap_and_write_sub_out_simple(bench_input: &SimpleProtoBenchMarkInput, subscriptions: Subscriptions, version: Version, parent_path: &String) {
    let id = bench_input.id.clone().unwrap_or(String::from("N/A"));
    let out = SimpleProtoBenchmarkSubSizeOutput {
        state_space_size: bench_input.state_space_size,
        number_of_edges: bench_input.number_of_edges,
        id,
        subscriptions: subscriptions,
        version: version
    };
    let file_name = format!("{parent_path}/{}_{}.json", out.id, serde_json::to_string(&out.version).unwrap().replace("\"", ""));
    let out = serde_json::to_string(&out).unwrap();
    write_file(&file_name, out);
}