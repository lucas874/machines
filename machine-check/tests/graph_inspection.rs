use machine_check::{
    composition::{composition_types::{DataResult, InterfacingSwarms, InspectionStruct}, inspection_struct}, types::Role
};

use serde::{Deserialize, Serialize};
use std::{fs::{create_dir_all, File}, path::Path, io::prelude::*};

use walkdir::WalkDir;

const BENCHMARK_DIR: &str = "./bench_and_results";
const SPECIAL_SYMBOL: &str = "done-special-symbol";

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct BenchMarkInput  {
    pub state_space_size: usize,
    pub number_of_edges: usize,
    pub interfacing_swarms: InterfacingSwarms<Role>
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

fn write_output(inspection_struct: InspectionStruct, parent_path: &String) {
    let file_name = format!("{parent_path}/{:010}.json", inspection_struct.n_states);
    let out = serde_json::to_string(&inspection_struct).unwrap();
    write_file(&file_name, out);
}

#[test]
#[ignore]
fn full_inspection() {
    let input_dir = format!("{BENCHMARK_DIR}/benchmarks/general_pattern/");
    let output_dir = format!("{BENCHMARK_DIR}/benchmark_inspection/general_pattern");
    create_directory(&output_dir);
    let mut interfacing_swarms_general =
        prepare_files_in_directory(input_dir);
    interfacing_swarms_general.sort_by(|(size1, _), (size2, _)| size1.cmp(size2));

    for (_, bi) in interfacing_swarms_general.iter() {
        let swarms = &bi.interfacing_swarms;
        match inspection_struct(swarms.clone()) {
            DataResult::OK{data: inspection_result} => write_output(inspection_result, &output_dir),
            DataResult::ERROR{ .. } => panic!("Something went wrong while getting inspection struct"),
        };

        println!("{}", SPECIAL_SYMBOL);
    }
}

#[test]
#[ignore]
fn short_inspection() {
    let input_dir = format!("{BENCHMARK_DIR}/benchmarks/general_pattern/");
    let output_dir = format!("{BENCHMARK_DIR}/benchmark_inspection/general_pattern");
    create_directory(&output_dir);
    let mut interfacing_swarms_general =
        prepare_files_in_directory(input_dir);
    interfacing_swarms_general.sort_by(|(size1, _), (size2, _)| size1.cmp(size2));
    let step: usize = 120;

    for (_, bi) in interfacing_swarms_general.iter().step_by(step) {
        let swarms = &bi.interfacing_swarms;
        match inspection_struct(swarms.clone()) {
            DataResult::OK{data: inspection_result} => write_output(inspection_result, &output_dir),
            DataResult::ERROR{ .. } => panic!("Something went wrong while getting inspection struct"),
        };

        println!("{}", SPECIAL_SYMBOL);
    }
}