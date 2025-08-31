# Parallel File Compressor

A high-performance CLI tool for compressing and decompressing files and folders using [Rust](https://www.rust-lang.org/), multithreading, and multiple compression formats: [Zstandard (zst)](https://facebook.github.io/zstd/), [gzip (gz)](https://www.gnu.org/software/gzip/), and [LZ4 (lz4)](https://lz4.github.io/lz4/).

Designed to showcase:
- Safe concurrency in Rust
- Efficient file I/O
- Real-time progress bars
- Compression statistics (ratio, speed, throughput)
- Recursive folder handling

---

## ✨ Features

- **Multithreaded chunk compression** for large files using [`rayon`](https://docs.rs/rayon)
- **Multi-format compression**: `.zst` (Zstandard), `.gz` (gzip), and `.lz4` (LZ4)
- **Recursive folder compression** with preserved directory structure
- **Decompression** of `.zst`, `.gz`, and `.lz4` files (auto-detected)
- **Colorized CLI output** for readability
- **Progress bars** for file, folder, and global byte progress
- **Detailed compression stats**: original size, compressed size, ratio, time, throughput
- **Integrity checks**: Per-file SHA-256 manifest is written on compression and verified after compression and decompression
- **Safer writes**: Output is written to a temporary `.part` file and atomically renamed to avoid corruption on crash/interruption
- **Adaptive chunk size**: Automatically chooses chunk size (256 KB–4 MB) based on file size for optimal performance and memory use

---

## 📦 Installation

### Prerequisites
- Rust (1.70+ recommended)
- Cargo

### Clone and build
```bash
git clone https://github.com/yourusername/parallel-file-compressor.git
cd parallel-file-compressor
cargo build --release
```

The compiled binary will be in `target/release/parallel_compressor`.

---

## 🚀 Usage

### Single File Compression
```bash
cargo run --release -- compress -i bigfile.txt -o bigfile.zst -t 8 --level 22
```

### Single File Decompression
```bash
cargo run --release -- decompress -i bigfile.zst -o bigfile.txt
```

### Recursive Folder Compression
```bash
cargo run --release -- compress -i ./data -o ./compressed -t 8
```
This will compress each file in `data/` into a `.zst` file in `compressed/`, preserving the folder structure. A `manifest-sha256.txt` will be written in the output folder.
### Single File Compression (all formats)
```bash
# Zstandard (default)
cargo run --release -- compress -i bigfile.txt -o bigfile.zst -t 8 --level 22
# Gzip
cargo run --release -- compress -i bigfile.txt -o bigfile.gz --format gz
# LZ4
cargo run --release -- compress -i bigfile.txt -o bigfile.lz4 --format lz4
```

### Single File Decompression (auto-detects format)
```bash
cargo run --release -- decompress -i bigfile.zst -o bigfile.txt
cargo run --release -- decompress -i bigfile.gz -o bigfile.txt
cargo run --release -- decompress -i bigfile.lz4 -o bigfile.txt
```

### Recursive Folder Compression (all formats)
```bash
# Zstandard (default)
cargo run --release -- compress -i ./data -o ./compressed -t 8
# Gzip
cargo run --release -- compress -i ./data -o ./compressed_gz -t 8 --format gz
# LZ4
cargo run --release -- compress -i ./data -o ./compressed_lz4 -t 8 --format lz4
```
This will compress each file in `data/` into the chosen format in the output folder, preserving the folder structure. A `manifest-sha256.txt` will be written in the output folder.
### Integrity Verification
After compression, a `manifest-sha256.txt` is created in the output directory, listing each file and its SHA-256 hash. After both compression and decompression, all files are verified against this manifest. If a file is corrupted or tampered with, decompression will fail with a hash mismatch error.

To test integrity, try modifying a `.zst` file and then decompressing it—the tool will detect the corruption.

---

## ⚙️ Command-line Options


### `compress`
| Option | Description | Example |
|--------|-------------|---------|
| `-i`, `--input` | Input file or folder | `-i bigfile.txt` |
| `-o`, `--output` | Output file or folder | `-o bigfile.zst` |
| `-t`, `--threads` | Number of threads (default: 4) | `-t 8` |
| `--level` | Compression level (1-22, default: 3, zstd only) | `--level 9` |
| `--format` | Compression format: `zst` (default), `gz`, or `lz4` | `--format gz` |


### `decompress`
| Option | Description | Example |
|--------|-------------|---------|
| `-i`, `--input` | Compressed file (`.zst`, `.gz`, `.lz4`) | `-i bigfile.zst` |
| `-o`, `--output` | Output file | `-o bigfile.txt` |

---

## 📊 Example Output

```text
[00:01.2] [=========================>         ] 15/20 (75%)
✅ Done!

📊 Compression complete!
Original size:   12.53 MB
Compressed size: 4.91 MB
Compression ratio: 39.20%
Time taken: 0.72 s
Throughput: 17.32 MB/s
```

---

## 🛠️ Development

### Generate Test Data
You can generate a test directory with multiple files using the provided Python script:
```bash
python3 scripts/generate_test_data.py
```

### Run with Cargo
```bash
cargo run -- compress -i test_data -o compressed_data -t 8
```

---

## ⚡️ Technical Notes

- **Adaptive chunk size**: The chunk size for reading/writing is chosen automatically for each file, between 256 KB and 4 MB, based on file size. This balances memory usage and throughput for both small and large files.
- **Safer writes**: All output is written to a temporary `.part` file and atomically renamed to the final name, so incomplete/corrupt files are never left behind after a crash or interruption.
- **SHA-256 manifest**: The manifest is a text file with lines like `hash  filename.zst`. It is used to verify file integrity after compression and decompression.

## 📈 Possible Enhancements

- `.tar.zst` single-file archives for folder compression
- File encryption before compression
- Parallel file-level and chunk-level compression
- Configurable chunk size

---

## 📜 License
This project is licensed under the MIT License.
