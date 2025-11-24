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
fn measure_exact() {
    let input_dir = format!("{BENCHMARK_DIR}/benchmarks/general_pattern/");
    let files: Vec<String> = vec![
        format!("{input_dir}max_5_roles_max_5_commands_6_protos/0000024973_0000002722_max_5_roles_max_5_commands_6_protos.json"),
        format!("{input_dir}max_10_roles_max_10_commands_2_protos/0000000884_0000000829_max_10_roles_max_10_commands_commands_2_protos.json"),
        format!("{input_dir}max_10_roles_max_10_commands_4_protos/0000004027_0000000040_max_10_roles_max_10_commands_commands_4_protos.json"),
        format!("{input_dir}max_5_roles_max_5_commands_9_protos/0000128347_0000002494_max_5_roles_max_5_commands_9_protos.json"),
        format!("{input_dir}max_5_roles_max_5_commands_5_protos/0000005888_0000002429_max_5_roles_max_5_commands_5_protos.json"),
    ];
    let mut interfacing_swarms_general =
        prepare_files_in_directory(input_dir, files);
    interfacing_swarms_general.sort_by(|(size1, _), (size2, _)| size1.cmp(size2));
    let subs = serde_json::to_string(&BTreeMap::<Role, BTreeSet<EventType>>::new()).unwrap();

    for (_, bi) in interfacing_swarms_general.iter() {
        let swarms = &bi.interfacing_swarms;
        println!("State space size: {}", bi.state_space_size);
        println!("Number of edges: {}", bi.number_of_edges);
        /* let subscriptions = match overapproximated_well_formed_sub(to_interfacing_protocols(swarms.clone()), subs.clone(), two_step_granularity.clone()) {
            DataResult::OK{data: subscriptions} => Some(subscriptions),
            DataResult::ERROR{ .. } => None,
        }; */

        let subscriptions = match exact_well_formed_sub(to_interfacing_protocols(swarms.clone()), subs.clone()) {
            DataResult::OK{data: subscriptions} => {
                Some(subscriptions) },
            DataResult::ERROR{ .. } => None,
        };
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

fn prepare_files_in_directory(directory: String, files: Vec<String>) -> Vec<(usize, BenchMarkInput)> {
    let mut inputs: Vec<(usize, BenchMarkInput)> = vec![];

    for entry in WalkDir::new(directory) {
        match entry {
            Ok(entry) => {
                if entry.file_type().is_file() && files.contains(&entry.path().as_os_str().to_str().unwrap().to_string()) {
                    println!("file: {}", entry.path().as_os_str().to_str().unwrap().to_string());
                    println!("Is contained? {}", files.contains(&entry.path().as_os_str().to_str().unwrap().to_string()));
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