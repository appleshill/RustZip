use parallel_compressor::compressor::{compress_single_file, decompress_file, sha256_file};
use indicatif::MultiProgress;
use std::io::Write;
use tempfile::NamedTempFile;

#[test]
fn test_sha256_file() {
    let mut tmp = NamedTempFile::new().unwrap();
    write!(tmp, "hello world").unwrap();
    let hash = sha256_file(tmp.path()).unwrap();
    assert_eq!(hash, "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9");
}

fn formats() -> Vec<(&'static str, &'static str)> {
    vec![
        ("zst", "zst"),
        ("gz", "gz"),
        ("lz4", "lz4"),
    ]
}

#[test]
fn test_compress_decompress_roundtrip_all_formats() {
    let data = b"The quick brown fox jumps over the lazy dog.";
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
