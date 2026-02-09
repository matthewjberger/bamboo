+++
title = "File I/O"
weight = 5
template = "book.html"
+++

# File I/O

Rust provides safe, cross-platform file I/O through the `std::fs` and `std::io` modules. This chapter covers common patterns for reading, writing, and manipulating files.

## Reading Files

The simplest way to read an entire file:

```rust
use std::fs;

fn main() -> std::io::Result<()> {
    let contents = fs::read_to_string("input.txt")?;
    println!("File has {} lines", contents.lines().count());
    Ok(())
}
```

For binary files, use `fs::read`:

```rust
use std::fs;

let bytes = fs::read("image.png")?;
println!("File size: {} bytes", bytes.len());
```

## Reading Line by Line

For large files, read line by line to avoid loading everything into memory:

```rust
use std::fs::File;
use std::io::{self, BufRead, BufReader};

fn count_matching_lines(path: &str, pattern: &str) -> io::Result<usize> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let count = reader
        .lines()
        .filter_map(|line| line.ok())
        .filter(|line| line.contains(pattern))
        .count();
    Ok(count)
}
```

## Writing Files

Write a string to a file (creates or overwrites):

```rust
use std::fs;

fs::write("output.txt", "Hello, world!\n")?;
```

For more control, use `File::create` with a writer:

```rust
use std::fs::File;
use std::io::{self, BufWriter, Write};

fn write_report(path: &str, data: &[(String, f64)]) -> io::Result<()> {
    let file = File::create(path)?;
    let mut writer = BufWriter::new(file);

    writeln!(writer, "Name,Score")?;
    for (name, score) in data {
        writeln!(writer, "{name},{score:.2}")?;
    }

    writer.flush()?;
    Ok(())
}
```

## Appending to Files

Open a file in append mode:

```rust
use std::fs::OpenOptions;
use std::io::Write;

fn append_log(path: &str, message: &str) -> std::io::Result<()> {
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?;

    writeln!(file, "{message}")?;
    Ok(())
}
```

## Working with Paths

Use `Path` and `PathBuf` for cross-platform path manipulation:

```rust
use std::path::{Path, PathBuf};

let base = Path::new("/home/user/documents");
let file_path = base.join("reports").join("2024-q1.csv");

println!("Extension: {:?}", file_path.extension());
println!("Filename: {:?}", file_path.file_name());
println!("Parent: {:?}", file_path.parent());
println!("Exists: {}", file_path.exists());
```

## Directory Operations

List files in a directory:

```rust
use std::fs;

fn list_rust_files(directory: &str) -> std::io::Result<Vec<String>> {
    let mut files = Vec::new();

    for entry in fs::read_dir(directory)? {
        let entry = entry?;
        let path = entry.path();

        if path.extension().map_or(false, |extension| extension == "rs") {
            if let Some(name) = path.file_name() {
                files.push(name.to_string_lossy().to_string());
            }
        }
    }

    files.sort();
    Ok(files)
}
```

## Temporary Files

Use the `tempfile` crate for safe temporary files:

```rust
use std::io::Write;
use tempfile::NamedTempFile;

fn process_with_tempfile(data: &[u8]) -> std::io::Result<()> {
    let mut temp = NamedTempFile::new()?;
    temp.write_all(data)?;

    println!("Temp file at: {:?}", temp.path());

    // File is automatically deleted when `temp` is dropped
    Ok(())
}
```

## Summary

Use `fs::read_to_string` and `fs::write` for simple cases. For large files or streaming, use `BufReader` and `BufWriter`. Always prefer `Path`/`PathBuf` over raw strings for file paths to ensure cross-platform compatibility.
