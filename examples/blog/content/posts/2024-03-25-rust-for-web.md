+++
title = "Why We Rewrote Our Core in Rust"
tags = ["rust", "performance", "engineering"]
excerpt = "The story of migrating our compression engine from C to Rust, and what we learned."
+++

Last quarter, we made the decision to rewrite Pied Piper's compression core from C to Rust. Here's why, and what we learned.

## The Problem with C

Our C codebase worked. It was fast. But we had problems:

1. **Memory bugs** - Despite careful coding, we'd find buffer overflows in edge cases
2. **Concurrency issues** - Data races that only manifested under heavy load
3. **Maintenance burden** - New engineers took months to become productive

The final straw was a production incident caused by a use-after-free bug that our tests didn't catch.

## Why Rust?

Rust gives us:

```rust
fn compress_chunk(data: &[u8]) -> Result<Vec<u8>, CompressionError> {
    let mut output = Vec::with_capacity(data.len());

    // The borrow checker ensures we can't accidentally
    // use `data` after it's been moved or freed

    for chunk in data.chunks(BLOCK_SIZE) {
        output.extend(process_block(chunk)?);
    }

    Ok(output)
}
```

- **Memory safety** without garbage collection
- **Fearless concurrency** - The compiler catches data races
- **Zero-cost abstractions** - High-level code that compiles to efficient assembly

## The Migration

We didn't rewrite everything at once. Our approach:

1. Wrap existing C code with Rust FFI
2. Rewrite one module at a time
3. Benchmark constantly to ensure no regression
4. Delete C code only after Rust replacement is proven

## Results

| Metric | C | Rust | Change |
|--------|---|------|--------|
| Throughput | 2.1 GB/s | 2.3 GB/s | +10% |
| Memory bugs (6 mo) | 12 | 0 | -100% |
| New engineer ramp-up | 3 months | 3 weeks | -75% |

The performance improvement surprised us—turns out the borrow checker enabled optimizations we were afraid to do in C.

## Lessons

1. Rewrites are risky, but sometimes necessary
2. Incremental migration beats big-bang
3. Rust's learning curve is worth it for systems code
