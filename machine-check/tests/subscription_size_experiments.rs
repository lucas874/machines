use machine_check::{
    composition::{composition_types::{DataResult, Granularity, InterfacingSwarms}, exact_weak_well_formed_sub, overapproximated_weak_well_formed_sub}, types::{EventType, Role}, Subscriptions
};

use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet}, fs::{create_dir_all, File}, path::Path, io::prelude::*
};

use walkdir::WalkDir;

#[test]
#[ignore]
fn full_run_bench_sub_sizes_general() {
    let parent_path = "bench_and_results".to_string();
    let dir_name = format!("subscription_size_benchmarks/general_pattern");
    create_directory(&parent_path, &dir_name);
    let mut interfacing_swarms_general =
        prepare_files_in_directory(String::from("./bench_and_results/benchmarks/general_pattern/"));
    interfacing_swarms_general.sort_by(|(size1, _), (size2, _)| size1.cmp(size2));
    let subs = serde_json::to_string(&BTreeMap::<Role, BTreeSet<EventType>>::new()).unwrap();
    let two_step_granularity = serde_json::to_string(&Granularity::TwoStep).unwrap();
    let number_of_inputs = interfacing_swarms_general.len();
    println!("Running the subscription size experiment with the full benchmark suite.");
    for (i, (_, bi)) in interfacing_swarms_general.iter().enumerate() {
        let swarms = serde_json::to_string(&bi.interfacing_swarms).unwrap();
        let subscriptions = match serde_json::from_str(&overapproximated_weak_well_formed_sub(swarms.clone(), subs.clone(), two_step_granularity.clone())).unwrap() {
            DataResult::OK{data: subscriptions} => Some(subscriptions),
            DataResult::ERROR{ .. } => None,
        };
        wrap_and_write_sub_out(&bi, subscriptions.unwrap(), two_step_granularity.replace("\"", ""), String::from("./bench_and_results/subscription_size_benchmarks/general_pattern"));

        let subscriptions = match serde_json::from_str(&exact_weak_well_formed_sub(swarms.clone(), subs.clone())).unwrap() {
            DataResult::OK{data: subscriptions} => {
                Some(subscriptions) },
            DataResult::ERROR{ .. } => None,
        };
        wrap_and_write_sub_out(&bi, subscriptions.unwrap(), String::from("Exact"), String::from("./bench_and_results/subscription_size_benchmarks/general_pattern"));
        println!("progress: {} / {} samples processed",  i+1, number_of_inputs);
    }
}

#[test]
#[ignore]
fn short_run_bench_sub_sizes_general() {
    let parent_path = "bench_and_results".to_string();
    let dir_name = format!("short_subscription_size_benchmarks/general_pattern");
    create_directory(&parent_path, &dir_name);
    let mut interfacing_swarms_general =
        prepare_files_in_directory(String::from("./bench_and_results/benchmarks/general_pattern/"));
    interfacing_swarms_general.sort_by(|(size1, _), (size2, _)| size1.cmp(size2));
    let subs = serde_json::to_string(&BTreeMap::<Role, BTreeSet<EventType>>::new()).unwrap();
    let two_step_granularity = serde_json::to_string(&Granularity::TwoStep).unwrap();
    let step: usize = 60;
    let number_of_inputs = interfacing_swarms_general.iter().step_by(step).len();
    println!("Running the execution time experiment with a subset of the samples in benchmark suite.");
    for (i, (_, bi)) in interfacing_swarms_general.iter().step_by(step).enumerate() {
        let swarms = serde_json::to_string(&bi.interfacing_swarms).unwrap();
        let subscriptions = match serde_json::from_str(&overapproximated_weak_well_formed_sub(swarms.clone(), subs.clone(), two_step_granularity.clone())).unwrap() {
            DataResult::OK{data: subscriptions} => Some(subscriptions),
            DataResult::ERROR{ .. } => None,
        };
        wrap_and_write_sub_out(&bi, subscriptions.unwrap(), two_step_granularity.replace("\"", ""), String::from("./bench_and_results/short_subscription_size_benchmarks/general_pattern"));

        let subscriptions = match serde_json::from_str(&exact_weak_well_formed_sub(swarms.clone(), subs.clone())).unwrap() {
            DataResult::OK{data: subscriptions} => {
                Some(subscriptions) },
            DataResult::ERROR{ .. } => None,
        };
        wrap_and_write_sub_out(&bi, subscriptions.unwrap(), String::from("Exact"), String::from("./bench_and_results/short_subscription_size_benchmarks/general_pattern"));
        println!("progress: {} / {} samples processed",  i+1, number_of_inputs);
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

fn create_directory(parent: &String, dir_name: &String) -> () {
    match create_dir_all(format!("{parent}/{dir_name}")) {
        Ok(_) => (),
        Err(ref e) if e.kind() == std::io::ErrorKind::AlreadyExists => (),
        Err(e) => panic!("couldn't create directory {}/{}: {}", parent, dir_name, e),
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


fn wrap_and_write_sub_out(bench_input: &BenchMarkInput, subscriptions: Subscriptions, granularity: String, parent_path: String) {
    let out = BenchmarkSubSizeOutput { state_space_size: bench_input.state_space_size, number_of_edges: bench_input.number_of_edges, subscriptions: subscriptions};
    let file_name = format!("{parent_path}/{:010}_{}.json", bench_input.state_space_size, granularity);
    let out = serde_json::to_string(&out).unwrap();
    write_file(&file_name, out);
}
