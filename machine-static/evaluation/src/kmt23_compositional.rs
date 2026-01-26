use std::collections::{BTreeMap, BTreeSet};

use clap::Parser;
use evaluation::{Cli, SPECIAL_SYMBOL, Version, create_directory, prepare_simple_inputs_in_directory, wrap_and_write_sub_out_simple};
use machine_check::{CheckResult, check_composed_swarm, check_swarm, well_formed_sub};
use machine_core::types::typescript_types::{DataResult, EventType, Granularity, InterfacingProtocols, Role, SubscriptionsWrapped};

fn main() {
    let cli = Cli::parse();
    let input_dir = cli.input_dir;
    let output_dir = cli.output_dir;
    create_directory(&output_dir);

    let inputs = prepare_simple_inputs_in_directory(&input_dir);
    let subs = BTreeMap::<Role, BTreeSet<EventType>>::new();
    let two_step_granularity = Granularity::TwoStep;

    for input in inputs.iter() {
        let subscriptions_wf_kmt =
            match well_formed_sub(input.proto.clone(), SubscriptionsWrapped(subs.clone())) {
                DataResult::OK {
                    data: subscriptions,
                } => Some(subscriptions),
                DataResult::ERROR { .. } => None,
            };
        match check_swarm(
            input.proto.clone(),
            SubscriptionsWrapped(subscriptions_wf_kmt.clone().unwrap()),
        ) {
            CheckResult::OK => (),
            CheckResult::ERROR { errors } => {
                println!(
                    "id: {}, cause: {},\n subscriptions: {}",
                    input.id.clone().unwrap_or(String::from("")),
                    errors.join(", "),
                    serde_json::to_string_pretty(&subscriptions_wf_kmt.clone()).unwrap()
                );
                panic!("Not ok compositional")
            }
        }
        wrap_and_write_sub_out_simple(
            &input,
            subscriptions_wf_kmt.unwrap(),
            Version::KMT23,
            &output_dir,
        );

        let subscriptions_compositional_exact = match machine_core::exact_well_formed_sub(
            InterfacingProtocols(vec![input.proto.clone()]),
            SubscriptionsWrapped(subs.clone()),
        ) {
            DataResult::OK {
                data: subscriptions,
            } => Some(subscriptions),
            DataResult::ERROR { .. } => None,
        };
        match check_composed_swarm(
            InterfacingProtocols(vec![input.proto.clone()]),
            SubscriptionsWrapped(subscriptions_compositional_exact.clone().unwrap()),
        ) {
            CheckResult::OK => (),
            CheckResult::ERROR { errors } => {
                println!(
                    "id: {}, cause: {},\n subscriptions: {}",
                    input.id.clone().unwrap_or(String::from("")),
                    errors.join(", "),
                    serde_json::to_string_pretty(&subscriptions_compositional_exact.clone())
                        .unwrap()
                );
                panic!("Not ok compositional")
            }
        }
        wrap_and_write_sub_out_simple(
            &input,
            subscriptions_compositional_exact.unwrap(),
            Version::CompositionalExact,
            &output_dir,
        );

        let subscriptions_compositional_approx =
            match machine_core::overapproximated_well_formed_sub(
                InterfacingProtocols(vec![input.proto.clone()]),
                SubscriptionsWrapped(subs.clone()),
                two_step_granularity.clone(),
            ) {
                DataResult::OK {
                    data: subscriptions,
                } => Some(subscriptions),
                DataResult::ERROR { .. } => None,
            };
        match check_composed_swarm(
            InterfacingProtocols(vec![input.proto.clone()]),
            SubscriptionsWrapped(subscriptions_compositional_approx.clone().unwrap()),
        ) {
            CheckResult::OK => (),
            CheckResult::ERROR { errors } => {
                println!(
                    "id: {}, cause: {},\n subscriptions: {}",
                    input.id.clone().unwrap_or(String::from("")),
                    errors.join(", "),
                    serde_json::to_string_pretty(&subscriptions_compositional_approx.clone())
                        .unwrap()
                );
                panic!("Not ok compositional")
            }
        }
        wrap_and_write_sub_out_simple(
            &input,
            subscriptions_compositional_approx.unwrap(),
            Version::CompositionalOverapprox,
            &output_dir,
        );

        println!("{}", SPECIAL_SYMBOL);
    }
}