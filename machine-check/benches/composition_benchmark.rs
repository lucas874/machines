use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use machine_check::composition::{compose_protocols, project_combine, revised_projection};
use machine_check::composition::composition_types::{DataResult, Granularity};
use machine_check::composition::{
    composition_types::InterfacingSwarms, exact_weak_well_formed_sub,
    overapproximated_weak_well_formed_sub,
};
use machine_check::types::{EventType, Role};
use machine_check::{Machine, Subscriptions, SwarmProtocol};
use serde::{Deserialize, Serialize};
use tracing_subscriber::registry::Data;
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
        Ok(_) => (), //print!("{} contains:\n{}", display, protos),
    }
    let (state_space_size, interfacing_swarms) =
        match serde_json::from_str::<BenchMarkInput>(&protos) {
            Ok(input) => (input.state_space_size, input.interfacing_swarms),
            Err(e) => panic!("error parsing input file: {}", e),
        };
    /* let protocols = match serde_json::from_str::<machine_check::CompositionInputVec>(&protos) {
        Ok(p) => p,
        Err(e) => panic!("error parsing composition input: {}", e),
    };

    protocols */

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
                    println!("file: {}", entry.path().as_os_str().to_str().unwrap().to_string());
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

/* fn bench_composition_refinement_pattern_1(c: &mut Criterion) {
    let mut group = c.benchmark_group("Composition refinement pattern 1");
    group.sample_size(50);
    let mut interfacing_swarms_refinement_1 =
        prepare_files_in_directory(String::from("./benches/benchmark_data_selected/refinement_pattern_1/"));
    interfacing_swarms_refinement_1.sort_by(|(size1, _), (size2, _)| size1.cmp(size2));

    let subs = serde_json::to_string(&BTreeMap::<Role, BTreeSet<EventType>>::new()).unwrap();
    let coarse_granularity = serde_json::to_string(&Granularity::Coarse).unwrap();
    let medium_granularity = serde_json::to_string(&Granularity::Medium).unwrap();
    let fine_granularity = serde_json::to_string(&Granularity::Fine).unwrap();
    for (size, interfacing_swarms) in interfacing_swarms_refinement_1.iter() {
        group.bench_with_input(BenchmarkId::new("coarse", size), interfacing_swarms,
        |b, input| b.iter(|| overapproximated_weak_well_formed_sub(input.clone(), subs.clone(), coarse_granularity.clone())));

        group.bench_with_input(BenchmarkId::new("medium", size), interfacing_swarms,
        |b, input| b.iter(|| overapproximated_weak_well_formed_sub(input.clone(), subs.clone(), medium_granularity.clone())));

        group.bench_with_input(BenchmarkId::new("fine", size), interfacing_swarms,
        |b, input| b.iter(|| overapproximated_weak_well_formed_sub(input.clone(), subs.clone(), fine_granularity.clone())));

        group.bench_with_input(BenchmarkId::new("exact", size), interfacing_swarms,
        |b, input| b.iter(|| exact_weak_well_formed_sub(input.clone(), subs.clone())));
    }
    group.finish();
} */

/* fn bench_composition_refinement_pattern_2(c: &mut Criterion) {
    let mut group = c.benchmark_group("Composition refinement pattern 2");
    group.sample_size(50);
    let mut interfacing_swarms_refinement_2 =
        prepare_files_in_directory(String::from("./benches/benchmark_data_selected/refinement_pattern_2/"));
    interfacing_swarms_refinement_2.sort_by(|(size1, _), (size2, _)| size1.cmp(size2));

    let subs = serde_json::to_string(&BTreeMap::<Role, BTreeSet<EventType>>::new()).unwrap();
    let coarse_granularity = serde_json::to_string(&Granularity::Coarse).unwrap();
    let medium_granularity = serde_json::to_string(&Granularity::Medium).unwrap();
    let fine_granularity = serde_json::to_string(&Granularity::Fine).unwrap();
    for (size, interfacing_swarms) in interfacing_swarms_refinement_2.iter() {
        group.bench_with_input(BenchmarkId::new("coarse", size), interfacing_swarms,
        |b, input| b.iter(|| overapproximated_weak_well_formed_sub(input.clone(), subs.clone(), coarse_granularity.clone())));

        group.bench_with_input(BenchmarkId::new("medium", size), interfacing_swarms,
        |b, input| b.iter(|| overapproximated_weak_well_formed_sub(input.clone(), subs.clone(), medium_granularity.clone())));

        group.bench_with_input(BenchmarkId::new("fine", size), interfacing_swarms,
        |b, input| b.iter(|| overapproximated_weak_well_formed_sub(input.clone(), subs.clone(), fine_granularity.clone())));

        group.bench_with_input(BenchmarkId::new("exact", size), interfacing_swarms,
        |b, input| b.iter(|| exact_weak_well_formed_sub(input.clone(), subs.clone())));
    }
    group.finish();
} */

/* fn bench_composition_random(c: &mut Criterion) {
    let mut group = c.benchmark_group("Composition random");
    let mut interfacing_swarms_random =
        prepare_files_in_directory(String::from("./benches/benchmark_data_selected/random/"));
    interfacing_swarms_random.sort_by(|(size1, _), (size2, _)| size1.cmp(size2));

    let subs = serde_json::to_string(&BTreeMap::<Role, BTreeSet<EventType>>::new()).unwrap();
    let coarse_granularity = serde_json::to_string(&Granularity::Coarse).unwrap();
    let medium_granularity = serde_json::to_string(&Granularity::Medium).unwrap();
    let fine_granularity = serde_json::to_string(&Granularity::Fine).unwrap();
    for (size, interfacing_swarms) in interfacing_swarms_random.iter() {
        group.bench_with_input(BenchmarkId::new("coarse", size), interfacing_swarms,
        |b, input| b.iter(|| overapproximated_weak_well_formed_sub(input.clone(), subs.clone(), coarse_granularity.clone())));

        group.bench_with_input(BenchmarkId::new("medium", size), interfacing_swarms,
        |b, input| b.iter(|| overapproximated_weak_well_formed_sub(input.clone(), subs.clone(), medium_granularity.clone())));

        group.bench_with_input(BenchmarkId::new("fine", size), interfacing_swarms,
        |b, input| b.iter(|| overapproximated_weak_well_formed_sub(input.clone(), subs.clone(), fine_granularity.clone())));

        group.bench_with_input(BenchmarkId::new("exact", size), interfacing_swarms,
        |b, input| b.iter(|| exact_weak_well_formed_sub(input.clone(), subs.clone())));
    }
    group.finish();
} */

/* fn bench_composition_pattern_3(c: &mut Criterion) {
    let mut group = c.benchmark_group("Composition pattern 3");
    group.sample_size(50);
    let mut interfacing_swarms_random =
        prepare_files_in_directory(String::from("./benches/pattern_3/1_non_interfacing_IR0_first/"));
    interfacing_swarms_random.sort_by(|(size1, _), (size2, _)| size1.cmp(size2));
    for (size, _) in interfacing_swarms_random.iter() {
        println!("{}", size);
    }

    let subs = serde_json::to_string(&BTreeMap::<Role, BTreeSet<EventType>>::new()).unwrap();
    let coarse_granularity = serde_json::to_string(&Granularity::Coarse).unwrap();
    let medium_granularity = serde_json::to_string(&Granularity::Medium).unwrap();
    let fine_granularity = serde_json::to_string(&Granularity::Fine).unwrap();
    for (i, (_, interfacing_swarms)) in interfacing_swarms_random.iter().enumerate() {
        group.bench_with_input(BenchmarkId::new("coarse", i + 1), interfacing_swarms,
        |b, input| b.iter(|| overapproximated_weak_well_formed_sub(input.clone(), subs.clone(), coarse_granularity.clone())));

        group.bench_with_input(BenchmarkId::new("medium", i + 1), interfacing_swarms,
        |b, input| b.iter(|| overapproximated_weak_well_formed_sub(input.clone(), subs.clone(), medium_granularity.clone())));

        group.bench_with_input(BenchmarkId::new("fine", i + 1), interfacing_swarms,
        |b, input| b.iter(|| overapproximated_weak_well_formed_sub(input.clone(), subs.clone(), fine_granularity.clone())));

        group.bench_with_input(BenchmarkId::new("exact", i + 1), interfacing_swarms,
        |b, input| b.iter(|| exact_weak_well_formed_sub(input.clone(), subs.clone())));
    }
    group.finish();
} */

fn bench_composition_pattern_3_ir0_last(c: &mut Criterion) {
    let mut group = c.benchmark_group("Composition pattern 3 ir0 last");
    group.sample_size(50);
    let mut interfacing_swarms_random =
        prepare_files_in_directory(String::from("./benches/pattern_3/1_non_interfacing_IR0_last/"));
    interfacing_swarms_random.sort_by(|(size1, _), (size2, _)| size1.cmp(size2));
    for (size, _) in interfacing_swarms_random.iter() {
        println!("{}", size);
    }

    let subs = serde_json::to_string(&BTreeMap::<Role, BTreeSet<EventType>>::new()).unwrap();
    let coarse_granularity = serde_json::to_string(&Granularity::Coarse).unwrap();
    let medium_granularity = serde_json::to_string(&Granularity::Medium).unwrap();
    let fine_granularity = serde_json::to_string(&Granularity::Fine).unwrap();
    for (i, (_, interfacing_swarms)) in interfacing_swarms_random.iter().enumerate() {
        group.bench_with_input(BenchmarkId::new("coarse", i + 1), interfacing_swarms,
        |b, input| b.iter(|| overapproximated_weak_well_formed_sub(input.clone(), subs.clone(), coarse_granularity.clone())));

        group.bench_with_input(BenchmarkId::new("medium", i + 1), interfacing_swarms,
        |b, input| b.iter(|| overapproximated_weak_well_formed_sub(input.clone(), subs.clone(), medium_granularity.clone())));

        group.bench_with_input(BenchmarkId::new("fine", i + 1), interfacing_swarms,
        |b, input| b.iter(|| overapproximated_weak_well_formed_sub(input.clone(), subs.clone(), fine_granularity.clone())));

        group.bench_with_input(BenchmarkId::new("exact", i + 1), interfacing_swarms,
        |b, input| b.iter(|| exact_weak_well_formed_sub(input.clone(), subs.clone())));
    }
    group.finish();
}

fn bench_projection(c: &mut Criterion) {
    let mut group = c.benchmark_group("Projection benchmarks");
    group.sample_size(10);
    let mut interfacing_swarms_refine_2 =
        prepare_files_in_directory(String::from("./benches/benchmark_data_selected/refinement_pattern_2/"));
    interfacing_swarms_refine_2.sort_by(|(size1, _), (size2, _)| size1.cmp(size2));
    for (size, _) in interfacing_swarms_refine_2.iter() {
        println!("{}", size);
    }

    let subs = serde_json::to_string(&BTreeMap::<Role, BTreeSet<EventType>>::new()).unwrap();
    let medium_granularity = serde_json::to_string(&Granularity::Medium).unwrap();
    let mut sub_vec = vec![];
    println!("state_space_size,role,sub_size, cpsize, pcsize");
    for (size, interfacing_swarms) in &interfacing_swarms_refine_2 {
        let subs = overapproximated_weak_well_formed_sub(interfacing_swarms.clone(), subs.clone(), medium_granularity.clone());
        let (subs, subs_map) = match serde_json::from_str::<DataResult<Subscriptions>>(&subs) {
            Ok(DataResult::OK { data }) => (serde_json::to_string(&data).unwrap(), data),
            _ => unimplemented!()
        };
        let composition = match serde_json::from_str::<DataResult<SwarmProtocol>>(&compose_protocols(interfacing_swarms.clone())) {
            Ok(DataResult::OK { data }) => data,//{ println!("{}", serde_json::to_string_pretty(&data).unwrap()); data},
            _ => unimplemented!()
        };

        let (r_max, sub) = subs_map.clone().into_iter().max_by(|(r1, sub1),(r2, sub2)| sub1.len().cmp(&sub2.len())).unwrap();
        sub_vec.push((size, subs.clone(), r_max.to_string(), interfacing_swarms.clone(), serde_json::to_string(&composition).unwrap()));
        let proj = revised_projection(serde_json::to_string(&composition).unwrap(), subs.clone(), r_max.to_string());
        let proj = match serde_json::from_str::<DataResult<Machine>>(&proj){
            Ok(DataResult::OK{data}) => data,
            _ => unimplemented!()

        };

        let proj1 = project_combine(interfacing_swarms.clone(), subs, r_max.to_string());
        let proj1 = match serde_json::from_str::<DataResult<Machine>>(&proj1){
            Ok(DataResult::OK{data}) => data,
            _ => unimplemented!()

        };
        //println!("{}", serde_json::to_string_pretty(&proj1).unwrap());
        /* let s = 50u32 as usize;
        if size > &s {
            break;
        } */
        println!("{},{},{},{},{}",size, r_max.to_string(), sub.len(), proj.transitions.len(), proj1.transitions.len());
    }
    for (size, subs, r, interfacing_swarms, composition) in sub_vec.clone().into_iter() {
        group.bench_with_input(BenchmarkId::new("compose_then_proj", size), &(&composition, &subs, &r),
        |b, (composition, subs, r)| b.iter(|| revised_projection((*composition).to_string(), (*subs).clone(), (*r).to_string())));
        group.bench_with_input(BenchmarkId::new("proj_then_compose", size), &(&interfacing_swarms, &subs, &r),
        |b, (interfacing_swarms, subs, r)| b.iter(|| project_combine((*interfacing_swarms).to_string(), (*subs).clone(), (*r).to_string())));

    }
    group.finish();
}

criterion_group!(benches, bench_projection);
criterion_main!(benches);
