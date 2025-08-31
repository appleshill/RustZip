mod cli;
mod compressor;

use cli::CliArgs;
use compressor::{Compressor, ZstdCompressor, GzipCompressor, Lz4Compressor};
use std::path::Path;
use clap::Parser;

fn main() -> anyhow::Result<()> {
    let args = CliArgs::parse();

    match args.subcommand {
        cli::SubCommand::Compress { input, output, threads, level, format } => {
            let format = format.to_lowercase();
            let compressor: Box<dyn Compressor> = match format.as_str() {
                "zst" => Box::new(ZstdCompressor),
                "gz" => Box::new(GzipCompressor),
                "lz4" => Box::new(Lz4Compressor),
                _ => anyhow::bail!("Unknown format: {}", format),
            };
            compressor::compress_path_with(&input, &output, threads, level, &*compressor)?;
        }
        cli::SubCommand::Decompress {input, output } => {
            let ext = Path::new(&input).extension().and_then(|e| e.to_str()).unwrap_or("");
            let compressor: Box<dyn Compressor> = match ext {
                "zst" => Box::new(ZstdCompressor),
                "gz" => Box::new(GzipCompressor),
                "lz4" => Box::new(Lz4Compressor),
                _ => anyhow::bail!("Unknown file extension: {}", ext),
            };
            compressor::decompress_file_with(&input, &output, &*compressor)?;
        }
    }

    Ok(())
}