+++
title = "Concurrency"
weight = 10
template = "book.html"
+++

# Concurrency

Rust's ownership model prevents data races at compile time, making concurrent programming safer than in most languages. This chapter covers both thread-based and lock-based concurrency patterns.

## Spawning Threads

Create OS threads with `std::thread::spawn`:

```rust
use std::thread;

fn main() {
    let handle = thread::spawn(|| {
        let mut sum = 0u64;
        for index in 0..1_000_000 {
            sum += index;
        }
        sum
    });

    println!("Main thread is free to do other work");

    let result = handle.join().unwrap();
    println!("Sum: {result}");
}
```

## Sharing Data with Arc and Mutex

Use `Arc<Mutex<T>>` to share mutable state across threads:

```rust
use std::sync::{Arc, Mutex};
use std::thread;

fn parallel_sum(data: &[i64], chunk_count: usize) -> i64 {
    let total = Arc::new(Mutex::new(0i64));
    let chunk_size = data.len().div_ceil(chunk_count);

    let handles: Vec<_> = data
        .chunks(chunk_size)
        .map(|chunk| {
            let total = Arc::clone(&total);
            let chunk = chunk.to_vec();
            thread::spawn(move || {
                let partial: i64 = chunk.iter().sum();
                let mut locked = total.lock().unwrap();
                *locked += partial;
            })
        })
        .collect();

    for handle in handles {
        handle.join().unwrap();
    }

    *total.lock().unwrap()
}
```

## RwLock for Read-Heavy Workloads

When reads vastly outnumber writes, `RwLock` allows multiple concurrent readers:

```rust
use std::sync::{Arc, RwLock};

struct Cache {
    data: Arc<RwLock<std::collections::HashMap<String, String>>>,
}

impl Cache {
    fn new() -> Self {
        Self {
            data: Arc::new(RwLock::new(std::collections::HashMap::new())),
        }
    }

    fn get(&self, key: &str) -> Option<String> {
        let locked = self.data.read().unwrap();
        locked.get(key).cloned()
    }

    fn set(&self, key: String, value: String) {
        let mut locked = self.data.write().unwrap();
        locked.insert(key, value);
    }
}
```

## Crossbeam — Scoped Threads

The `crossbeam` crate provides scoped threads that can borrow from the enclosing scope:

```rust
use crossbeam::thread;

fn parallel_map(data: &[f64], operation: fn(f64) -> f64) -> Vec<f64> {
    let chunk_count = num_cpus::get();
    let chunk_size = data.len().div_ceil(chunk_count);

    thread::scope(|scope| {
        let handles: Vec<_> = data
            .chunks(chunk_size)
            .map(|chunk| {
                scope.spawn(move |_| {
                    chunk.iter().map(|value| operation(*value)).collect::<Vec<_>>()
                })
            })
            .collect();

        handles
            .into_iter()
            .flat_map(|handle| handle.join().unwrap())
            .collect()
    })
    .unwrap()
}
```

## Atomic Operations

For simple counters and flags, atomics avoid the overhead of mutexes:

```rust
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::thread;

fn atomic_counter() {
    let counter = Arc::new(AtomicU64::new(0));
    let mut handles = Vec::new();

    for _ in 0..8 {
        let counter = Arc::clone(&counter);
        handles.push(thread::spawn(move || {
            for _ in 0..1000 {
                counter.fetch_add(1, Ordering::Relaxed);
            }
        }));
    }

    for handle in handles {
        handle.join().unwrap();
    }

    println!("Final count: {}", counter.load(Ordering::Relaxed));
}
```

## Rayon — Parallel Iterators

The `rayon` crate provides effortless data parallelism:

```rust
use rayon::prelude::*;

fn parallel_processing(images: &[Image]) -> Vec<Thumbnail> {
    images
        .par_iter()
        .map(|image| generate_thumbnail(image))
        .collect()
}

fn parallel_search(documents: &[Document], query: &str) -> Vec<&Document> {
    documents
        .par_iter()
        .filter(|document| document.text.contains(query))
        .collect()
}
```

## Summary

| Tool | Use When |
|------|----------|
| `std::thread` | CPU-bound work, independent tasks |
| `Arc<Mutex<T>>` | Shared mutable state, write-heavy |
| `Arc<RwLock<T>>` | Shared state, read-heavy workloads |
| Atomics | Simple counters and flags |
| Crossbeam scoped threads | Need to borrow from enclosing scope |
| Rayon | Data-parallel operations on collections |
