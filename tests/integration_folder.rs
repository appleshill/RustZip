use indicatif::MultiProgress;
use tempfile::tempdir;
use std::fs::{self, File};
use std::io::Write;
use parallel_compressor::compressor::{compress_path, decompress_file, sha256_file};

#[test]
fn test_folder_compress_and_manifest() {
    let dir = tempdir().unwrap();
    let input_dir = dir.path().join("input");
    let output_dir = dir.path().join("output");
    fs::create_dir(&input_dir).unwrap();
    // Create test files
    for i in 0..3 {
        let mut f = File::create(input_dir.join(format!("file{}.txt", i))).unwrap();
        write!(f, "testdata{}", i).unwrap();
    }
    compress_path(input_dir.to_str().unwrap(), output_dir.to_str().unwrap(), 2, 3).unwrap();
    // Check manifest exists
    let manifest = output_dir.join("manifest-sha256.txt");
    assert!(manifest.exists());
    // Check hashes in manifest match actual files
    let manifest_str = fs::read_to_string(&manifest).unwrap();
    for line in manifest_str.lines() {
        let mut parts = line.split_whitespace();
        let hash = parts.next().unwrap();
        let file = parts.next().unwrap();
        let file_path = output_dir.join(file);
        assert_eq!(sha256_file(&file_path).unwrap(), hash);
    }
}
