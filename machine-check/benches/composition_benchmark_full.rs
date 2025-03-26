use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use machine_check::composition::composition_types::Granularity;
use machine_check::composition::{
    composition_types::InterfacingSwarms, exact_weak_well_formed_sub,
    overapproximated_weak_well_formed_sub,
};
use machine_check::types::{EventType, Role};
use serde::{Deserialize, Serialize};
extern crate machine_check;
use std::collections::{BTreeMap, BTreeSet};
use std::fs::File;
use std::io::prelude::*;
use std::path::Path;
use walkdir::WalkDir;
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct BenchMarkInput {
    pub state_space_size: usize,
    pub number_of_edges: usize,
    pub interfacing_swarms: InterfacingSwarms<Role>,
}

fn prepare_input(file_name: String) -> (usize, String) {
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
            Ok(input) => (input.state_space_size, input.interfacing_swarms),
            Err(e) => panic!("error parsing input file: {}", e),
        };

    (
        state_space_size,
        serde_json::to_string(&interfacing_swarms).unwrap(),
    )
}

fn prepare_files_in_directory(directory: String) -> Vec<(usize, String)> {
    let mut inputs: Vec<(usize, String)> = vec![];

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

fn full_bench_general(c: &mut Criterion) {
    let mut group = c.benchmark_group("General pattern algorithm 1 vs. exact. Full.");
    group.sample_size(10);
    let mut interfacing_swarms_general =
        prepare_files_in_directory(String::from("./bench_and_results/benchmarks/general_pattern/"));
    interfacing_swarms_general.sort_by(|(size1, _), (size2, _)| size1.cmp(size2));

    let subs = serde_json::to_string(&BTreeMap::<Role, BTreeSet<EventType>>::new()).unwrap();
    let two_step_granularity = serde_json::to_string(&Granularity::TwoStep).unwrap();
    let number_of_inputs = interfacing_swarms_general.len();
    println!("Running the execution time experiment with the full benchmark suite.");
    for (i, (size, interfacing_swarms)) in interfacing_swarms_general.iter().enumerate() {
        group.bench_with_input(BenchmarkId::new("Algorithm 1", size), interfacing_swarms,
        |b, input| b.iter(|| overapproximated_weak_well_formed_sub(input.clone(), subs.clone(), two_step_granularity.clone())));

        group.bench_with_input(BenchmarkId::new("Exact", size), interfacing_swarms,
        |b, input| b.iter(|| exact_weak_well_formed_sub(input.clone(), subs.clone())));
        println!("progress: {} / {} samples processed",  i+1, number_of_inputs);
    }
    group.finish();
}

criterion_group!(benches, full_bench_general);
criterion_main!(benches);
