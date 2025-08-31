use parallel_compressor::compressor::{compress_single_file, decompress_file, sha256_file};
use indicatif::MultiProgress;
use tempfile::NamedTempFile;
use std::io::Write;

fn formats() -> Vec<(&'static str, &'static str)> {
    vec![
        ("zst", "zst"),
        ("gz", "gz"),
        ("lz4", "lz4"),
    ]
}

#[test]
fn test_empty_file_all_formats() {
    for (ext, _fmt) in formats() {
        let input = NamedTempFile::new().unwrap();
        let input_path = input.path();
        let compressed = NamedTempFile::new().unwrap();
        let compressed_path = compressed.path().with_extension(ext);
        let mp = MultiProgress::new();
        compress_single_file(input_path, &compressed_path, &mp, 3).unwrap();
        let output = NamedTempFile::new().unwrap();
        decompress_file(compressed_path.to_str().unwrap(), output.path().to_str().unwrap()).unwrap();
        let out_bytes = std::fs::read(output.path()).unwrap();
        assert!(out_bytes.is_empty(), "Failed for format: {}", ext);
    }
}

#[test]
fn test_small_file_all_formats() {
    let data = b"a";
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
fn test_non_utf8_file_all_formats() {
    let data = [0, 159, 146, 150]; // Invalid UTF-8
    for (ext, _fmt) in formats() {
        let mut input = NamedTempFile::new().unwrap();
        input.write_all(&data).unwrap();
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
