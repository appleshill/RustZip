mod cli;
mod compressor;

use cli::CliArgs;
use compressor::{compress_path, decompress_file};
use clap::Parser;

fn main() -> anyhow::Result<()> {
    let args = CliArgs::parse();

    match args.subcommand {
        cli::SubCommand::Compress { input, output, threads } => {
            compress_path(&input, &output, threads)?;
        }
        cli::SubCommand::Decompress {input, output } => {
            decompress_file(&input, &output)?;
        }
    }

    Ok(())
}