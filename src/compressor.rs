use rayon::prelude::*;
use rayon::ThreadPoolBuilder;
use std::path::PathBuf;
use std::{fs::File, io::{Read, Write, copy}, path::Path};
use zstd::stream::{Encoder, Decoder};
use std::time::Instant;
use std::fs::metadata;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use colored::*;
use anyhow::Context;
use walkdir::WalkDir;

const CHUNK_SIZE: usize = 1_000_000; // 1MB

pub struct Stats {
    pub original_size: u64,
    pub compressed_size: u64,
    pub duration_secs: f64,
}

fn compress_single_file(
    input_path: &Path,
    output_path: &Path,
    mp: &MultiProgress,
) -> anyhow::Result<Stats> {
    // read whole input
    let start = Instant::now();
    let mut input_file = File::open(input_path)
        .with_context(|| format!("Failed to open {}", input_path.display()))?;
    let mut buffer = Vec::new();
    input_file.read_to_end(&mut buffer)?;

    let chunks: Vec<&[u8]> = buffer.chunks(CHUNK_SIZE).collect();

    // per‑file progress bar
    let bar = mp.add(ProgressBar::new(chunks.len() as u64));
    bar.set_prefix(format!("{}", input_path.file_name().unwrap_or_default().to_string_lossy()));
    bar.set_style(
        ProgressStyle::with_template(
            "{prefix:.dim}  [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} {percent:>3}%"
        ).unwrap()
        .progress_chars("=> ")
    );

    // parallel compress chunks 
    let compressed_chunks: Vec<Vec<u8>> = chunks
        .into_par_iter()
        .map_init(
            || bar.clone(),
            |bar, chunk| {
                let mut encoder = Encoder::new(Vec::new(), 0).unwrap();
                encoder.write_all(chunk).unwrap();
                let result = encoder.finish().unwrap();
                bar.inc(1);
                result
            },
        )
        .collect();

    // write output
    if let Some(parent) = output_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut output_file = File::create(output_path)
        .with_context(|| format!("Failed to create {}", output_path.display()))?;
    for chunk in compressed_chunks {
        output_file.write_all(&chunk)?;
    }

    bar.finish_with_message("done");

    // stats
    let duration = start.elapsed();
    let original_size = metadata(input_path)?.len();
    let compressed_size = metadata(output_path)?.len();

    // pretty per‑file stats
    println!("\n{}", "📊 Compression complete!".bold().green());
    println!(
        "{} {:.2} MB",
        "Original size:   ".blue(),
        original_size as f64 / 1_048_576.0
    );
    println!(
        "{} {:.2} MB",
        "Compressed size: ".blue(),
        compressed_size as f64 / 1_048_576.0
    );
    let ratio = compressed_size as f64 / original_size as f64;
    println!("{} {:.2}%", "Compression ratio:".yellow(), ratio * 100.0);
    println!("{} {:.2} s", "Time taken:".magenta(), duration.as_secs_f64());
    let speed = (original_size as f64 / 1_048_576.0) / duration.as_secs_f64();
    println!("{} {:.2} MB/s", "Throughput:".cyan(), speed);

    Ok(Stats {
        original_size,
        compressed_size,
        duration_secs: duration.as_secs_f64(),
    })
}

pub fn compress_path(input_path: &str, output_path: &str, threads: usize) -> anyhow::Result<()> {
    // build the global pool once here
    ThreadPoolBuilder::new().num_threads(threads).build_global().ok();

    let input = Path::new(input_path);
    let output = Path::new(output_path);
    let mp = MultiProgress::new();

    if input.is_file() {
        // single file
        let out = if output.is_dir() {
            // if output is a dir, mirror file name with .zst
            output.join(
                PathBuf::from(input.file_name().unwrap())
                    .with_extension("zst")
            )
        } else {
            output.to_path_buf()
        };
        let _ = compress_single_file(input, &out, &mp)?;
    } else if input.is_dir() {
        // walk dir and collect files first to know count
        let mut files = Vec::new();
        for entry in WalkDir::new(input).into_iter().filter_map(Result::ok) {
            if entry.file_type().is_file() {
                files.push(entry.path().to_path_buf());
            }
        }

        // overall files progress bar
        let files_bar = mp.add(ProgressBar::new(files.len() as u64));
        files_bar.set_style(
            ProgressStyle::with_template(
                "{msg:.bold} [{bar:40.green/black}] {pos}/{len} {percent:>3}%"
            ).unwrap()
            .progress_chars("=> ")
        );
        files_bar.set_message("Compressing files");

        let mut total_orig: u64 = 0;
        let mut total_comp: u64 = 0;
        let mut total_secs: f64 = 0.0;

        for file in files {
            // keep directory structure under output/
            let rel = file.strip_prefix(input).unwrap();
            let out_file = output.join(rel).with_extension("zst");

            let stats = compress_single_file(&file, &out_file, &mp)?;
            total_orig += stats.original_size;
            total_comp += stats.compressed_size;
            total_secs += stats.duration_secs;

            files_bar.inc(1);
        }

        files_bar.finish_with_message("All files done");

        // overall summary
        println!("\n{}", "📦 Folder compression summary".bold().green());
        println!(
            "{} {:.2} MB",
            "Total original:   ".blue(),
            total_orig as f64 / 1_048_576.0
        );
        println!(
            "{} {:.2} MB",
            "Total compressed: ".blue(),
            total_comp as f64 / 1_048_576.0
        );
        let ratio = total_comp as f64 / total_orig as f64;
        println!("{} {:.2}%", "Overall ratio:    ".yellow(), ratio * 100.0);
        println!("{} {:.2} s", "Total time:       ".magenta(), total_secs);
        let throughput = (total_orig as f64 / 1_048_576.0) / total_secs.max(1e-9);
        println!("{} {:.2} MB/s", "Avg throughput:   ".cyan(), throughput);
    } else {
        anyhow::bail!("Input path is not a file or directory");
    }

    Ok(())
}

pub fn decompress_file(input_path: &str, output_path: &str) -> anyhow::Result<()> {
    let start = Instant::now();

    let mut input_file = File::open(input_path)?;
    let mut decoder = Decoder::new(&mut input_file)?;
    
    let mut output_file = File::create(output_path)?;
    std::io::copy(&mut decoder, &mut output_file)?;
    
    let duration = start.elapsed();

    // Get sizes
    let compressed_size = metadata(input_path)?.len();
    let decompressed_size = metadata(output_path)?.len();

    let ratio = decompressed_size as f64 / compressed_size as f64;
    let speed = (decompressed_size as f64 / 1_048_576.0) / duration.as_secs_f64(); // MB/s

    println!("\n{}", "📊 Decompression complete!".bold().green());

    println!(
        "{} {:.2} MB",
        "Compressed size:     ".blue(),
        compressed_size as f64 / 1_048_576.0
    );
    println!(
        "{} {:.2} MB",
        "Decompressed size:   ".blue(),
        decompressed_size as f64 / 1_048_576.0
    );
    println!(
        "{} {:.2}%",
        "Expansion ratio:     ".yellow(),
        ratio * 100.0
    );
    println!(
        "{} {:.2} s",
        "Time taken:".magenta(),
        duration.as_secs_f64()
    );
    println!(
        "{} {:.2} MB/s",
        "Throughput:".cyan(),
        speed
    );

    Ok(())
}