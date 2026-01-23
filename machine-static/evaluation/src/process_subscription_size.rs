use std::collections::BTreeSet;

use anyhow::{Error, Result};
use average::Mean;
use clap::Parser;
use evaluation::{
    BenchmarkSubSizeOutput, Cli, SubSizeProcessed, create_directory,
    read_sub_size_outputs_in_directory,
};
use machine_core::types::typescript_types::{EventType, Subscriptions};

fn avg_subscription_size(subscriptions: &Subscriptions) -> Option<f64> {
    if subscriptions.is_empty() {
        return None;
    }
    let numerator: f64 = subscriptions
        .values()
        .map(|event_types| event_types.len() as f64)
        .sum();
    let denominator = subscriptions.keys().len() as f64;

    Some(numerator / denominator)
}

fn avg_subscription_size1(subscriptions: &Subscriptions) -> Option<f64> {
    if subscriptions.is_empty() {
        return None;
    }
    let sub_sizes: Mean = subscriptions
        .values()
        .map(|event_types| event_types.len() as f64)
        .collect();

    Some(sub_sizes.mean())
}

fn process_subscription_size_ouput(
    sub_result: &BenchmarkSubSizeOutput,
) -> Result<SubSizeProcessed> {
    // Subscription is well-formed so union of subscriptions for each role is the set of all events of the protocol
    let num_event_types = sub_result
        .subscriptions
        .values()
        .cloned()
        .flatten()
        .collect::<BTreeSet<EventType>>()
        .len() as f64;

    let avg_sub_size = avg_subscription_size1(&sub_result.subscriptions)
        .ok_or(Error::msg("Unable to compute average subscription size"))?;
    let efrac = avg_sub_size / num_event_types;
    Ok(SubSizeProcessed {
        state_space_size: sub_result.state_space_size,
        number_of_edges: sub_result.number_of_edges,
        efrac,
        version: sub_result.version.clone(),
    })
}

fn main() {
    let cli = Cli::parse();
    let input_dir = cli.input_dir;
    let output_dir = cli.output_dir;
    create_directory(&output_dir);

    let results = read_sub_size_outputs_in_directory(&input_dir);
    if results.is_ok() {
        let a: Mean = (1..6).map(f64::from).collect();
        println!("The mean is {}.", a.mean());
        let _: Vec<_> = results
            .unwrap()
            .iter()
            .map(process_subscription_size_ouput)
            .collect();
    } else {
        println!("oh {:#?}", results.err().unwrap())
    }
}
