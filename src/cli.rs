use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "Parallel Compressor", version)]
pub struct CliArgs {
    #[command(subcommand)]
    pub subcommand: SubCommand,
}

#[derive(Subcommand)]
pub enum SubCommand {
    Compress {
        #[arg(short, long)]
        input: String,
        #[arg(short, long)]
        output: String,
        #[arg(short, long, default_value_t = 4)]
        threads: usize,
        #[arg(long, default_value_t = 3)]
        level: i32, // Zstd supports 1..=22, default=3
        #[arg(long, default_value = "zst")]
        format: String, // zst, gz, lz4
    },
    Decompress {
        #[arg(short, long)]
        input: String,
        #[arg(short, long)]
        output: String,
    },
}