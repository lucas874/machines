use std::collections::{BTreeMap, BTreeSet};

use clap::Parser;
use evaluation::{Cli, SPECIAL_SYMBOL, create_directory, prepare_files_in_directory, wrap_and_write_sub_out};
use machine_core::types::typescript_types::{DataResult, EventType, Granularity, Role, SubscriptionsWrapped};

fn main() {
    let cli = Cli::parse();
    let input_dir = cli.input_dir;
    let output_dir = cli.output_dir;
    create_directory(&output_dir);

    let mut interfacing_swarms_general = prepare_files_in_directory(&input_dir);
    interfacing_swarms_general.sort_by(|(size1, _), (size2, _)| size1.cmp(size2));
    let subs = BTreeMap::<Role, BTreeSet<EventType>>::new();
    let two_step_granularity = Granularity::TwoStep;
    let step: usize = 120;

    for (_, bi) in interfacing_swarms_general.iter().step_by(step) {
        let swarms = &bi.interfacing_swarms;
        let subscriptions = match machine_core::overapproximated_well_formed_sub(
            swarms.clone(),
            SubscriptionsWrapped(subs.clone()),
            two_step_granularity.clone(),
        ) {
            DataResult::OK {
                data: subscriptions,
            } => Some(subscriptions),
            DataResult::ERROR { .. } => None,
        };
        wrap_and_write_sub_out(
            &bi,
            subscriptions.unwrap(),
            serde_json::to_string(&two_step_granularity)
                .unwrap()
                .replace("\"", ""),
            &output_dir,
        );

        let subscriptions = match machine_core::exact_well_formed_sub(
            swarms.clone(),
            SubscriptionsWrapped(subs.clone()),
        ) {
            DataResult::OK {
                data: subscriptions,
            } => Some(subscriptions),
            DataResult::ERROR { .. } => None,
        };
        wrap_and_write_sub_out(
            &bi,
            subscriptions.unwrap(),
            String::from("Exact"),
            &output_dir,
        );
        println!("{}", SPECIAL_SYMBOL);
    }
}