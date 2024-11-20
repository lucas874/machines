use criterion::{criterion_group, criterion_main, Criterion, BenchmarkId};
extern crate machine_check;
use std::fs::{self, File};
use std::io::prelude::*;
use std::path::Path;
use std::time::Duration;

fn prepare_input(file_name: String) -> (String, String) {
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

    /* let protocols = match serde_json::from_str::<machine_check::CompositionInputVec>(&protos) {
        Ok(p) => p,
        Err(e) => panic!("error parsing composition input: {}", e),
    };

    protocols */

    (file_name, protos)
}

fn prepare_files_in_directory(directory: String) -> Vec<(String, String)> {
    let mut inputs: Vec<(String, String)> = vec![];
    let paths = fs::read_dir(directory).unwrap();
    for path in paths {
        inputs.push(prepare_input(path.unwrap().path().into_os_string().into_string().unwrap()));
    }

    inputs
}

fn bench_composition(c: &mut Criterion) {
    let mut group = c.benchmark_group("Composition");
    let composition_input_1 = prepare_files_in_directory(String::from("./benches/test_data/3_protos_10_roles_10_commands/"));
    let composition_input_2 = prepare_files_in_directory(String::from("./benches/test_data/5_protos_10_roles_10_commands/"));
    let composition_input_3 = prepare_files_in_directory(String::from("./benches/test_data/7_protos_10_roles_10_commands/"));
    let composition_input_4 = prepare_files_in_directory(String::from("./benches/test_data/10_protos_10_roles_10_commands/"));
    let mut composition_input: Vec<(String, String)> = vec![composition_input_1[1..5].to_vec(), composition_input_2[1..5].to_vec(), composition_input_3[1..5].to_vec(), composition_input_4[1..5].to_vec()].concat();
    composition_input.sort_by(|(_, a), (_, b)| a.len().cmp(&b.len()));
    for (i, _) in &composition_input {
        println!("{}", i);
    }
    //group.measurement_time(Duration::new(20, 0));

    for (i, (id, composition_input)) in composition_input.iter().enumerate() {
        group.bench_with_input(BenchmarkId::new("Overapproximation", i), composition_input,
        |b, input| b.iter(|| machine_check::compose_subs(input.clone())));
        group.bench_with_input(BenchmarkId::new("Exact", i), composition_input,
        |b, input| b.iter(|| machine_check::get_wwf_sub(machine_check::compose_protocols(input.clone()))));

    }
    group.finish();
}

/* fn bench_fibs(c: &mut Criterion) {
    let mut group = c.benchmark_group("Fibonacci");
    for i in [20u64, 21u64].iter() {
        group.bench_with_input(BenchmarkId::new("Recursive", i), i,
            |b, i| b.iter(|| fibonacci_slow(*i)));
        group.bench_with_input(BenchmarkId::new("Iterative", i), i,
            |b, i| b.iter(|| fibonacci_fast(*i)));
    }
    group.finish();
}

criterion_group!(benches, bench_fibs);
criterion_main!(benches);
 */
criterion_group!(benches, bench_composition);
criterion_main!(benches);