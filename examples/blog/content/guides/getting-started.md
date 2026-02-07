+++
title = "Getting Started with Pied Piper"
weight = 1
+++

This guide walks you through setting up Pied Piper's compression SDK in your project.

## Prerequisites

Before you begin, make sure you have:

- Rust 1.75 or later
- A Pied Piper API key (sign up at piedpiper.com)
- At least 4GB of available RAM

## Installation

Add the Pied Piper SDK to your `Cargo.toml`:

```toml
[dependencies]
piedpiper = "3.2"
```

## Basic Usage

Here's the simplest way to compress data with Pied Piper:

```rust
use piedpiper::Compressor;

fn main() -> Result<(), piedpiper::Error> {
    let compressor = Compressor::new()?;
    let input = std::fs::read("large-file.bin")?;
    let compressed = compressor.compress(&input)?;

    println!(
        "Compressed {} bytes to {} bytes ({:.1}x ratio)",
        input.len(),
        compressed.len(),
        input.len() as f64 / compressed.len() as f64,
    );

    std::fs::write("large-file.bin.pp", &compressed)?;
    Ok(())
}
```

## Configuration

You can tune the compression level and algorithm:

```rust
use piedpiper::{Compressor, Config, Algorithm};

let config = Config::builder()
    .algorithm(Algorithm::MiddleOut)
    .compression_level(9)
    .threads(4)
    .build();

let compressor = Compressor::with_config(config)?;
```

## Next Steps

- Read about [middle-out compression](/posts/building-static-sites/) to understand how the algorithm works
- Check the [Weissman Scores post](/posts/markdown-features/) to learn about measuring compression performance
- Join our community on Discord for support and discussion
