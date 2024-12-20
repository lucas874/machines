use criterion::{criterion_group, criterion_main, Criterion, BenchmarkId};
use machine_check::composition::composition_types::Granularity;
use machine_check::composition::{composition_types::InterfacingSwarms,  exact_weak_well_formed_sub, overapproximated_weak_well_formed_sub};
use machine_check::types::{EventType, Role};
use serde::{Serialize, Deserialize};
extern crate machine_check;
use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, File};
use std::io::prelude::*;
use std::path::Path;
use walkdir::WalkDir;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct BenchMarkInput  {
    pub state_space_size: usize,
    pub interfacing_swarms: InterfacingSwarms<Role>
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
        Ok(_) => ()//print!("{} contains:\n{}", display, protos),
    }
    let (state_space_size, interfacing_swarms) = match  serde_json::from_str::<BenchMarkInput>(&protos) {
       Ok(input) => (input.state_space_size, input.interfacing_swarms),
        Err(e) => panic!("error parsing input file: {}", e),
    };
    /* let protocols = match serde_json::from_str::<machine_check::CompositionInputVec>(&protos) {
        Ok(p) => p,
        Err(e) => panic!("error parsing composition input: {}", e),
    };

    protocols */


    (state_space_size, serde_json::to_string(&interfacing_swarms).unwrap())
}

fn prepare_files_in_directory(directory: String) -> Vec<(usize, String)> {
    let mut inputs: Vec<(usize, String)> = vec![];

    for entry in WalkDir::new(directory) {
        match entry {
            Ok(entry) => {
                if entry.file_type().is_file() {
                    inputs.push(prepare_input(entry.path().as_os_str().to_str().unwrap().to_string()));
                }
            },
            Err(e) => panic!("error: {}", e),
        };
    }


    inputs
}

fn bench_composition(c: &mut Criterion) {
    let mut group = c.benchmark_group("Composition");
    let mut interfacing_swarms_refinement_2 = prepare_files_in_directory(String::from("./benches/protocols/refinement_pattern_2/"));
    interfacing_swarms_refinement_2.sort_by(|(size1, _), (size2, _)| size1.cmp(size2));
    for (size, _0_) in &interfacing_swarms_refinement_2 {
        println!("{}", size);
    }
    //group.measurement_time(Duration::new(20, 0));

    let subs = serde_json::to_string(&BTreeMap::<Role, BTreeSet::<EventType>>::new()).unwrap();
    let granularity = serde_json::to_string(&Granularity::Coarse).unwrap();
    for (i, (_, interfacing_swarm)) in interfacing_swarms_refinement_2.iter().enumerate() {
        group.bench_with_input(BenchmarkId::new("Overapproximation", i), interfacing_swarm,
        |b, input| b.iter(|| overapproximated_weak_well_formed_sub(input.clone(), subs.clone(), granularity.clone())));
        group.bench_with_input(BenchmarkId::new("Exact", i), interfacing_swarm,
        |b, input| b.iter(|| exact_weak_well_formed_sub(input.clone(), subs.clone())));

    }
    group.finish();
}

criterion_group!(benches, bench_composition);
criterion_main!(benches);