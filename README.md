# Parallel File Compressor

A high-performance CLI tool for compressing and decompressing files and folders using [Rust](https://www.rust-lang.org/), multithreading, and the [Zstandard (zstd)](https://facebook.github.io/zstd/) compression algorithm.

Designed to showcase:
- Safe concurrency in Rust
- Efficient file I/O
- Real-time progress bars
- Compression statistics (ratio, speed, throughput)
- Recursive folder handling

---

## ✨ Features

- **Multithreaded chunk compression** for large files using [`rayon`](https://docs.rs/rayon)
- **Zstandard** compression (`.zst`) via [`zstd`](https://docs.rs/zstd)
- **Recursive folder compression** with preserved directory structure
- **Decompression** of `.zst` files
- **Colorized CLI output** for readability
- **Progress bars** for file and chunk processing
- Detailed **compression stats**:
  - Original size
  - Compressed size
  - Compression ratio
  - Time taken
  - Throughput (MB/s)

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
cargo run -- compress -i bigfile.txt -o bigfile.zst -t 8
```

### Single File Decompression
```bash
cargo run -- decompress -i bigfile.zst -o bigfile.txt
```

### Recursive Folder Compression
```bash
cargo run -- compress -i ./data -o ./compressed -t 8
```
This will compress each file in `data/` into a `.zst` file in `compressed/`, preserving the folder structure.

---

## ⚙️ Command-line Options

### `compress`
| Option | Description | Example |
|--------|-------------|---------|
| `-i`, `--input` | Input file or folder | `-i bigfile.txt` |
| `-o`, `--output` | Output file or folder | `-o bigfile.zst` |
| `-t`, `--threads` | Number of threads (default: 4) | `-t 8` |

### `decompress`
| Option | Description | Example |
|--------|-------------|---------|
| `-i`, `--input` | Compressed file (`.zst`) | `-i bigfile.zst` |
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

## 📈 Possible Enhancements

- `.tar.zst` single-file archives for folder compression
- Additional compression formats (`gzip`, `lz4`, etc.)
- File encryption before compression
- Parallel file-level and chunk-level compression
- Configurable chunk size

---

## 📜 License
This project is licensed under the MIT License.
