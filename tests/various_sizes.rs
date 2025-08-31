use parallel_compressor::compressor::{compress_single_file, decompress_file};
use indicatif::MultiProgress;
use tempfile::NamedTempFile;
use std::io::Write;

fn make_data(size: usize) -> Vec<u8> {
    // Patterned data for compressibility
    (0..size).map(|i| (i % 251) as u8).collect()
}

fn formats() -> Vec<(&'static str, &'static str)> {
    vec![
        ("zst", "zst"),
        ("gz", "gz"),
        ("lz4", "lz4"),
    ]
}

fn test_size(data: &[u8]) {
    for (ext, _fmt) in formats() {
        let mut input = NamedTempFile::new().unwrap();
        input.write_all(data).unwrap();
        let input_path = input.path();
        let compressed = NamedTempFile::new().unwrap();
        let compressed_path = compressed.path().with_extension(ext);
        let mp = MultiProgress::new();
        compress_single_file(input_path, &compressed_path, &mp, 3).unwrap();
        let output = NamedTempFile::new().unwrap();
        decompress_file(compressed_path.to_str().unwrap(), output.path().to_str().unwrap()).unwrap();
        let out_bytes = std::fs::read(output.path()).unwrap();
        assert_eq!(out_bytes, data, "Failed for format: {}", ext);
    }
}

#[test]
fn test_256kb_file() {
    let data = make_data(256 * 1024);
    test_size(&data);
}

#[test]
fn test_1mb_file() {
    let data = make_data(1 * 1024 * 1024);
    test_size(&data);
}

#[test]
fn test_4mb_file() {
    let data = make_data(4 * 1024 * 1024);
    test_size(&data);
}

#[test]
fn test_10mb_file() {
    let data = make_data(10 * 1024 * 1024);
    test_size(&data);
}
