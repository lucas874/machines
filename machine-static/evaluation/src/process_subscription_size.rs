use std::{collections::BTreeSet, path::Path};
// duct
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

fn compute_efrac(subscriptions: &Subscriptions) -> Result<f64> {
    // Subscription is well-formed so union of subscriptions for each role is the set of all events of the protocol
    let num_event_types = subscriptions
        .values()
        .cloned()
        .flatten()
        .collect::<BTreeSet<EventType>>()
        .len() as f64;

    let avg_sub_size = avg_subscription_size1(&subscriptions).ok_or(Error::msg("Unable to compute average subscription size"))?;
    Ok(avg_sub_size / num_event_types)
}

fn process_subscription_size_ouput(
    sub_result: &BenchmarkSubSizeOutput,
) -> Result<SubSizeProcessed> {
    let efrac = compute_efrac(&sub_result.subscriptions)?;
    Ok(SubSizeProcessed {
        state_space_size: sub_result.state_space_size,
        number_of_edges: sub_result.number_of_edges,
        efrac,
        version: sub_result.version.clone(),
    })
}

fn to_processed(sub_size_outputs: Vec<BenchmarkSubSizeOutput>) -> Result<Vec<SubSizeProcessed>> {
    sub_size_outputs
        .iter()
        .map(process_subscription_size_ouput)
        .collect()
}

fn write_processed(processed: Vec<SubSizeProcessed>, output_file: &Path) -> Result<()> {
    let mut wtr = csv::Writer::from_path(output_file)?;

    for record in processed {
        wtr.serialize(record)?;
    }
    wtr.flush()?;
    Ok(())
}

fn process_results(input_path: &Path, output_path: &Path) -> Result<()> {
    let prefix = output_path.parent().ok_or(Error::msg("Error: invalid output parent directory"))?;
    create_directory(prefix);
    let mut sub_size_outputs = read_sub_size_outputs_in_directory(&input_path)?;
    sub_size_outputs.sort_by_key(|sub_size_outpout| sub_size_outpout.number_of_edges);
    let processed_sub_size_outputs = to_processed(sub_size_outputs)?;

    write_processed(processed_sub_size_outputs, Path::new(output_path))
}


fn main() {
    let cli = Cli::parse();
    let input_path = cli.input_dir;
    let output_path = cli.output_dir;

    let result = process_results(&input_path, &output_path);
    if result.is_err() {
        println!("Error processing results: {:#?}", result);
    }



    /* if results.is_ok() {
        let a: Mean = (1..6).map(f64::from).collect();
        println!("The mean is {}.", a.mean());
        let _: Vec<_> = results
            .unwrap()
            .iter()
            .map(process_subscription_size_ouput)
            .collect();
    } else {
        println!("oh {:#?}", results.err().unwrap())
    } */
}
