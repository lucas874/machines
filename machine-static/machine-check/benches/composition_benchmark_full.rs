use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use machine_check::composition::composition_types::{Granularity, InterfacingProtocols};
use machine_check::composition::{
    composition_types::InterfacingSwarms, exact_well_formed_sub,
    overapproximated_well_formed_sub,
};
use machine_check::types::{EventType, Role};
use serde::{Deserialize, Serialize};
extern crate machine_check;
use std::collections::{BTreeMap, BTreeSet};
use std::fs::File;
use std::io::prelude::*;
use std::path::Path;
use walkdir::WalkDir;
use tracing_subscriber::{fmt, fmt::format::FmtSpan, EnvFilter};

const BENCHMARK_DIR: &str = "./bench_and_results";
const SPECIAL_SYMBOL: &str = "done-special-symbol";

fn setup_logger() {
    fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .with_span_events(FmtSpan::ENTER | FmtSpan::CLOSE)
        .try_init()
        .ok();
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct BenchMarkInput {
    pub state_space_size: usize,
    pub number_of_edges: usize,
    pub interfacing_swarms: InterfacingSwarms<Role>,
}

fn prepare_input(file_name: String) -> (usize, InterfacingProtocols) {
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
        InterfacingProtocols(interfacing_swarms.0.into_iter().map(|cc| cc.protocol).collect()),
    )
}

fn prepare_files_in_directory(directory: String) -> Vec<(usize, InterfacingProtocols)> {
    let mut inputs: Vec<(usize, InterfacingProtocols)> = vec![];

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
    setup_logger();
    let mut group = c.benchmark_group("General-pattern-algorithm1-vs.-exact-full-run");
    group.sample_size(10);
    let input_dir = format!("{BENCHMARK_DIR}/benchmarks/general_pattern/");
    let mut interfacing_swarms_general =
        prepare_files_in_directory(input_dir);
    interfacing_swarms_general.sort_by(|(size1, _), (size2, _)| size1.cmp(size2));

    let subs = serde_json::to_string(&BTreeMap::<Role, BTreeSet<EventType>>::new()).unwrap();
    let two_step_granularity = Granularity::TwoStep;

    for (size, interfacing_swarms) in interfacing_swarms_general.iter() {
        group.bench_with_input(BenchmarkId::new("Algorithm 1", size), interfacing_swarms,
        |b, input| b.iter(|| overapproximated_well_formed_sub(input.clone(), subs.clone(), two_step_granularity.clone())));

        group.bench_with_input(BenchmarkId::new("Exact", size), interfacing_swarms,
        |b, input| b.iter(|| exact_well_formed_sub(input.clone(), subs.clone())));

        println!("{}", SPECIAL_SYMBOL);
    }
    group.finish();
}

criterion_group!(benches, full_bench_general);
criterion_main!(benches);
