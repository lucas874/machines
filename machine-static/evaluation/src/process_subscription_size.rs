use clap::Parser;
use evaluation::{Cli, create_directory, read_sub_size_outputs_in_directory};

fn main() {
    let cli = Cli::parse();
    let input_dir = cli.input_dir;
    let output_dir = cli.output_dir;
    create_directory(&output_dir);

    let results = read_sub_size_outputs_in_directory(&input_dir);
    if results.is_ok() {
        println!("{}", results.unwrap().len())
    } else {
        println!("error: {}", results.unwrap_err())
    }

}