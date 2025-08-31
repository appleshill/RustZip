use std::{
    fs::File,
    io::{Read, Write},
    path::{Path, PathBuf},
    fs::metadata,
    time::Instant,
};

use anyhow::Context;
use colored::*;
use flate2::write::{GzDecoder, GzEncoder};
use flate2::Compression as GzCompression;
use hex;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use lz4_flex::frame::{FrameDecoder as Lz4Decoder, FrameEncoder as Lz4Encoder};
use rayon::ThreadPoolBuilder;
use sha2::{Digest, Sha256};
use walkdir::WalkDir;
use zstd::stream::{Decoder, Encoder};

/// Zstd 
pub struct ZstdCompressor;

/// Gzip 
pub struct GzipCompressor;

/// Lz4 
pub struct Lz4Compressor;
/// Trait for multi-format compression support (object-safe)
pub trait Compressor {
    fn compress(&self, input: &mut dyn Read, output: &mut dyn Write, level: i32) -> anyhow::Result<()>;
    fn decompress(&self, input: &mut dyn Read, output: &mut dyn Write) -> anyhow::Result<()>;
    fn extension(&self) -> &'static str;
}

impl Compressor for ZstdCompressor {
    fn compress(&self, input: &mut dyn Read, output: &mut dyn Write, level: i32) -> anyhow::Result<()> {
        let mut encoder = zstd::stream::Encoder::new(output, level)?;
        std::io::copy(input, &mut encoder)?;
        encoder.finish()?;
        Ok(())
    }
    fn decompress(&self, input: &mut dyn Read, output: &mut dyn Write) -> anyhow::Result<()> {
        let mut decoder = zstd::stream::Decoder::new(input)?;
        std::io::copy(&mut decoder, output)?;
        Ok(())
    }
    fn extension(&self) -> &'static str { "zst" }
}

impl Compressor for GzipCompressor {
    fn compress(&self, input: &mut dyn Read, output: &mut dyn Write, level: i32) -> anyhow::Result<()> {
        let mut encoder = GzEncoder::new(output, GzCompression::new(level as u32));
        std::io::copy(input, &mut encoder)?;
        encoder.finish()?;
        Ok(())
    }
    fn decompress(&self, input: &mut dyn Read, output: &mut dyn Write) -> anyhow::Result<()> {
        let mut decoder = GzDecoder::new(output);
        std::io::copy(input, &mut decoder)?;
        Ok(())
    }
    fn extension(&self) -> &'static str { "gz" }
}

impl Compressor for Lz4Compressor {
    fn compress(&self, input: &mut dyn Read, output: &mut dyn Write, _level: i32) -> anyhow::Result<()> {
        let mut encoder = Lz4Encoder::new(output);
        std::io::copy(input, &mut encoder)?;
        encoder.finish()?;
        Ok(())
    }
    fn decompress(&self, input: &mut dyn Read, output: &mut dyn Write) -> anyhow::Result<()> {
        let mut decoder = Lz4Decoder::new(input);
        std::io::copy(&mut decoder, output)?;
        Ok(())
    }
    fn extension(&self) -> &'static str { "lz4" }
}

/// Compress a file or directory
pub fn compress_path_with(input_path: &str, output_path: &str, threads: usize, level: i32, compressor: &dyn Compressor) -> anyhow::Result<()> {
    ThreadPoolBuilder::new().num_threads(threads).build_global().ok();
    let input = Path::new(input_path);
    let output = Path::new(output_path);
    let mp = MultiProgress::new();

    if input.is_file() {
        let out = if output.is_dir() {
            output.join(
                PathBuf::from(input.file_name().unwrap())
                    .with_extension(compressor.extension())
            )
        } else {
            output.to_path_buf()
        };
        compress_single_file_with(input, &out, &mp, level, compressor)?;
    } else if input.is_dir() {
        let mut files = Vec::new();
        let mut total_bytes: u64 = 0;
        for entry in WalkDir::new(input).into_iter().filter_map(Result::ok) {
            if entry.file_type().is_file() {
                total_bytes += entry.metadata().map(|m| m.len()).unwrap_or(0);
                files.push(entry.path().to_path_buf());
            }
        }
        let global_bar = mp.add(ProgressBar::new(total_bytes));
        global_bar.set_style(
            ProgressStyle::with_template(
                "{msg:.bold} [{bar:40.green/black}] {bytes}/{total_bytes} {percent:>3}%"
            ).unwrap()
            .progress_chars("=> ")
        );
        global_bar.set_message("Total progress");
        for file in files {
            let rel = file.strip_prefix(input).unwrap();
            let out_file = output.join(rel).with_extension(compressor.extension());
            compress_single_file_with(&file, &out_file, &mp, level, compressor)?;
            global_bar.inc(metadata(&file)?.len());
        }
        global_bar.finish_with_message("All files done");
    } else {
        anyhow::bail!("Input path is not a file or directory");
    }
    Ok(())
}

/// Compress a single file
pub fn compress_single_file_with(
    input_path: &Path,
    output_path: &Path,
    mp: &MultiProgress,
    level: i32,
    compressor: &dyn Compressor
) -> anyhow::Result<Stats> {
    let start = Instant::now();
    let file_size = metadata(input_path)?.len();
    if let Some(parent) = output_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut tmp_path = output_path.to_path_buf();
    tmp_path.set_extension(
        match output_path.extension() {
            Some(ext) => format!("{}.part", ext.to_string_lossy()),
            None => "part".to_string(),
        }
    );
    let mut input_file = File::open(input_path)
        .with_context(|| format!("Failed to open {}", input_path.display()))?;
    let mut output_file = File::create(&tmp_path)
        .with_context(|| format!("Failed to create {}", tmp_path.display()))?;
    // Progress bar
    let bar = mp.add(ProgressBar::new(file_size));
    bar.set_prefix(format!("{}", input_path.file_name().unwrap_or_default().to_string_lossy()));
    bar.set_style(
        ProgressStyle::with_template(
            "{prefix:.dim}  [{elapsed_precise}] [{bar:40.cyan/blue}] {bytes}/{total_bytes} {percent:>3}%"
        ).unwrap()
        .progress_chars("=> ")
    );
    // Wrap input_file in a progress reader
    struct ProgressReader<'a, R: Read> {
        inner: R,
        bar: &'a ProgressBar,
        total: u64,
    }
    impl<'a, R: Read> Read for ProgressReader<'a, R> {
        fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
            let n = self.inner.read(buf)?;
            if n > 0 {
                self.total += n as u64;
                self.bar.set_position(self.total);
            }
            Ok(n)
        }
    }
    let mut progress_reader = ProgressReader { inner: &mut input_file, bar: &bar, total: 0 };
    compressor.compress(&mut progress_reader, &mut output_file, level)?;
    bar.finish_with_message("done");
    std::fs::rename(&tmp_path, output_path)?;
    let duration = start.elapsed();
    let compressed_size = metadata(output_path)?.len();
    Ok(Stats {
        original_size: file_size,
        compressed_size,
        duration_secs: duration.as_secs_f64(),
    })
}

/// Decompress a file
pub fn decompress_file_with(input_path: &str, output_path: &str, compressor: &dyn Compressor) -> anyhow::Result<()> {
    let mut input_file = File::open(input_path)?;
    let mut output_file = File::create(output_path)?;
    compressor.decompress(&mut input_file, &mut output_file)?;
    Ok(())
}

/// Choose an adaptive chunk size based on file size (256 KB to 4 MB)
fn choose_chunk_size(file_size: u64) -> usize {
    let min = 256 * 1024;
    let max = 4 * 1024 * 1024;
    let mut chunk = (file_size / 64) as usize;
    chunk = chunk.clamp(min, max);
    chunk
}

pub struct Stats {
    pub original_size: u64,
    pub compressed_size: u64,
    pub duration_secs: f64,
}

use std::io::{BufReader, BufWriter};
pub fn sha256_file(path: &Path) -> anyhow::Result<String> {
    let mut file = File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buf = [0u8; 8192];
    loop {
        let n = file.read(&mut buf)?;
        if n == 0 { break; }
        hasher.update(&buf[..n]);
    }
    Ok(hex::encode(hasher.finalize()))
}

pub fn compress_single_file(
    input_path: &Path,
    output_path: &Path,
    mp: &MultiProgress,
    level: i32
) -> anyhow::Result<Stats> {
    // streamed I/O version
    let start = Instant::now();
    let input_file = File::open(input_path)
        .with_context(|| format!("Failed to open {}", input_path.display()))?;
    let file_size = metadata(input_path)?.len();
    let chunk_size = choose_chunk_size(file_size);
    let mut reader = BufReader::with_capacity(chunk_size, input_file);

    if let Some(parent) = output_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    // Write to a temporary .part file first
    let mut tmp_path = output_path.to_path_buf();
    tmp_path.set_extension(
        match output_path.extension() {
            Some(ext) => format!("{}.part", ext.to_string_lossy()),
            None => "part".to_string(),
        }
    );
    let output_file = File::create(&tmp_path)
        .with_context(|| format!("Failed to create {}", tmp_path.display()))?;
    let mut writer = BufWriter::with_capacity(chunk_size, output_file);
    let mut encoder = Encoder::new(&mut writer, level)?;

    // Progress bar: unknown chunk count, so use bytes
    let original_size = file_size;
    let bar = mp.add(ProgressBar::new(original_size));
    bar.set_prefix(format!("{}", input_path.file_name().unwrap_or_default().to_string_lossy()));
    bar.set_style(
        ProgressStyle::with_template(
            "{prefix:.dim}  [{elapsed_precise}] [{bar:40.cyan/blue}] {bytes}/{total_bytes} {percent:>3}%"
        ).unwrap()
        .progress_chars("=> ")
    );

    let mut total_read = 0u64;
    let mut buf = vec![0u8; chunk_size];
    loop {
        let n = reader.read(&mut buf)?;
        if n == 0 { break; }
        encoder.write_all(&buf[..n])?;
        total_read += n as u64;
        bar.set_position(total_read.min(original_size));
    }
    encoder.finish()?;
    writer.flush()?;
    bar.finish_with_message("done");

    // Atomically rename .part file to final output
    std::fs::rename(&tmp_path, output_path)?;

    let duration = start.elapsed();
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

pub fn compress_path(input_path: &str, output_path: &str, threads: usize, level: i32) -> anyhow::Result<()> {
    // build the global pool once here
    ThreadPoolBuilder::new().num_threads(threads).build_global().ok();

    let input = Path::new(input_path);
    let output = Path::new(output_path);
    let mp = MultiProgress::new();

    use std::collections::BTreeMap;
    let mut manifest = BTreeMap::new();
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
        let _ = compress_single_file(input, &out, &mp, level)?;
        // SHA-256 manifest for single file
        let hash = sha256_file(&out)?;
        manifest.insert(out.file_name().unwrap().to_string_lossy().to_string(), hash);
    } else if input.is_dir() {
        // walk dir and collect files first to know count and total size
        let mut files = Vec::new();
        let mut total_bytes: u64 = 0;
        for entry in WalkDir::new(input).into_iter().filter_map(Result::ok) {
            if entry.file_type().is_file() {
                total_bytes += entry.metadata().map(|m| m.len()).unwrap_or(0);
                files.push(entry.path().to_path_buf());
            }
        }

        // global progress bar by bytes
        let global_bar = mp.add(ProgressBar::new(total_bytes));
        global_bar.set_style(
            ProgressStyle::with_template(
                "{msg:.bold} [{bar:40.green/black}] {bytes}/{total_bytes} {percent:>3}%"
            ).unwrap()
            .progress_chars("=> ")
        );
        global_bar.set_message("Total progress");

        let mut total_orig: u64 = 0;
        let mut total_comp: u64 = 0;
        let mut total_secs: f64 = 0.0;

        for file in files {
            // keep directory structure under output/
            let rel = file.strip_prefix(input).unwrap();
            let out_file = output.join(rel).with_extension("zst");

            // compress and update global progress bar as we drain input
            let stats = {
                let s = compress_single_file(&file, &out_file, &mp, level)?;
                global_bar.inc(s.original_size);
                s
            };
            // Compute hash for manifest
            let hash = sha256_file(&out_file)?;
            manifest.insert(rel.with_extension("zst").to_string_lossy().to_string(), hash);
            total_orig += stats.original_size;
            total_comp += stats.compressed_size;
            total_secs += stats.duration_secs;
        }

        global_bar.finish_with_message("All files done");

        // Write manifest
        use std::io::Write;
        let manifest_path = output.join("manifest-sha256.txt");
        let mut mf = File::create(&manifest_path)?;
        for (file, hash) in &manifest {
            writeln!(mf, "{}  {}", hash, file)?;
        }
        println!("SHA-256 manifest written to {}", manifest_path.display());
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

    // After compression, verify all hashes
    for (file, expected) in &manifest {
        let path = if input.is_file() {
            output.join(file)
        } else {
            output.join(file)
        };
        let actual = sha256_file(&path)?;
        if &actual != expected {
            println!("{}: {} != {}", file, actual, expected);
            anyhow::bail!("Hash mismatch for {}", file);
        }
    }
    println!("All files verified by SHA-256 hash.");
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

    // Integrity check: verify decompressed file against manifest if present
    use std::fs;
    let manifest_path = Path::new(input_path).parent().map(|p| p.join("manifest-sha256.txt"));
    if let Some(manifest_path) = manifest_path {
        if manifest_path.exists() {
            let manifest = fs::read_to_string(&manifest_path)?;
            for line in manifest.lines() {
                let mut parts = line.split_whitespace();
                let hash = parts.next();
                let file = parts.next();
                if let (Some(hash), Some(file)) = (hash, file) {
                    let out_file = Path::new(output_path);
                    if out_file.file_name().map(|n| n == file).unwrap_or(false) {
                        let actual = sha256_file(out_file)?;
                        if actual != hash {
                            println!("Hash mismatch for {}: {} != {}", file, actual, hash);
                            anyhow::bail!("Decompressed file hash mismatch");
                        } else {
                            println!("Verified {} by SHA-256 hash.", file);
                        }
                    }
                }
            }
        }
    }

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