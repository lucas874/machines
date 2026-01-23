use clap::Parser;
use evaluation::{Cli, create_directory};

fn main() {
    let cli = Cli::parse();
    let input_dir = cli.input_dir;
    let output_dir = cli.output_dir;
    create_directory(&output_dir);

}