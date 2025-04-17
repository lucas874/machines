use machine_check::{
    composition::{composition_types::{DataResult, Granularity, InterfacingSwarms}, exact_weak_well_formed_sub, overapproximated_weak_well_formed_sub}, types::{EventType, Role}, Subscriptions
};

use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet}, fs::{create_dir_all, File}, path::Path, io::prelude::*
};

use walkdir::WalkDir;

const BENCHMARK_DIR: &str = "./bench_and_results";
const SPECIAL_SYMBOL: &str = "done-special-symbol";

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
    let two_step_granularity = serde_json::to_string(&Granularity::TwoStep).unwrap();

    for (_, bi) in interfacing_swarms_general.iter() {
        let swarms = serde_json::to_string(&bi.interfacing_swarms).unwrap();
        let subscriptions = match serde_json::from_str(&overapproximated_weak_well_formed_sub(swarms.clone(), subs.clone(), two_step_granularity.clone())).unwrap() {
            DataResult::OK{data: subscriptions} => Some(subscriptions),
            DataResult::ERROR{ .. } => None,
        };
        wrap_and_write_sub_out(&bi, subscriptions.unwrap(), two_step_granularity.replace("\"", ""), &output_dir);

        let subscriptions = match serde_json::from_str(&exact_weak_well_formed_sub(swarms.clone(), subs.clone())).unwrap() {
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
    let two_step_granularity = serde_json::to_string(&Granularity::TwoStep).unwrap();
    let step: usize = 120;

    for (_, bi) in interfacing_swarms_general.iter().step_by(step) {
        let swarms = serde_json::to_string(&bi.interfacing_swarms).unwrap();
        let subscriptions = match serde_json::from_str(&overapproximated_weak_well_formed_sub(swarms.clone(), subs.clone(), two_step_granularity.clone())).unwrap() {
            DataResult::OK{data: subscriptions} => Some(subscriptions),
            DataResult::ERROR{ .. } => None,
        };
        wrap_and_write_sub_out(&bi, subscriptions.unwrap(), two_step_granularity.replace("\"", ""), &output_dir);

        let subscriptions = match serde_json::from_str(&exact_weak_well_formed_sub(swarms.clone(), subs.clone())).unwrap() {
            DataResult::OK{data: subscriptions} => {
                Some(subscriptions) },
            DataResult::ERROR{ .. } => None,
        };
        wrap_and_write_sub_out(&bi, subscriptions.unwrap(), String::from("Exact"), &output_dir);
        println!("{}", SPECIAL_SYMBOL);
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct BenchMarkInput  {
    pub state_space_size: usize,
    pub number_of_edges: usize,
    pub interfacing_swarms: InterfacingSwarms<Role>
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct BenchmarkSubSizeOutput  {
    pub state_space_size: usize,
    pub number_of_edges: usize,
    pub subscriptions: Subscriptions,
}

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
