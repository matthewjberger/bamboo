+++
title = "Async Patterns"
weight = 9
template = "book.html"
+++

# Async Patterns

Rust's async/await system provides zero-cost abstractions for concurrent I/O. This chapter covers common patterns for working with async code effectively.

## Spawning Tasks

Run independent work concurrently with `tokio::spawn`:

```rust
use tokio;

#[tokio::main]
async fn main() {
    let handle = tokio::spawn(async {
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        42
    });

    println!("Doing other work...");

    let result = handle.await.unwrap();
    println!("Task returned: {result}");
}
```

## Running Tasks Concurrently

Use `tokio::join!` to run multiple futures concurrently and wait for all of them:

```rust
async fn fetch_user(id: u64) -> User { /* ... */ }
async fn fetch_orders(user_id: u64) -> Vec<Order> { /* ... */ }
async fn fetch_preferences(user_id: u64) -> Preferences { /* ... */ }

async fn load_dashboard(user_id: u64) -> Dashboard {
    let (user, orders, preferences) = tokio::join!(
        fetch_user(user_id),
        fetch_orders(user_id),
        fetch_preferences(user_id),
    );

    Dashboard { user, orders, preferences }
}
```

## Select — Racing Futures

Use `tokio::select!` to race multiple futures and act on whichever completes first:

```rust
use tokio::time::{sleep, Duration};

async fn fetch_with_timeout(url: &str) -> Result<String, &'static str> {
    tokio::select! {
        result = reqwest::get(url) => {
            match result {
                Ok(response) => Ok(response.text().await.unwrap_or_default()),
                Err(_) => Err("request failed"),
            }
        }
        _ = sleep(Duration::from_secs(5)) => {
            Err("timeout")
        }
    }
}
```

## Streams

Process async sequences with `Stream`, the async equivalent of `Iterator`:

```rust
use tokio_stream::{self as stream, StreamExt};

async fn process_stream() {
    let mut numbers = stream::iter(vec![1, 2, 3, 4, 5])
        .filter(|number| *number % 2 == 0)
        .map(|number| number * 10);

    while let Some(value) = numbers.next().await {
        println!("Got: {value}");
    }
}
```

## Channels

Communicate between tasks with channels:

```rust
use tokio::sync::mpsc;

#[tokio::main]
async fn main() {
    let (sender, mut receiver) = mpsc::channel::<String>(100);

    let producer = tokio::spawn(async move {
        for index in 0..5 {
            sender
                .send(format!("message {index}"))
                .await
                .unwrap();
        }
    });

    let consumer = tokio::spawn(async move {
        while let Some(message) = receiver.recv().await {
            println!("Received: {message}");
        }
    });

    let _ = tokio::join!(producer, consumer);
}
```

## Semaphores for Concurrency Limiting

Limit concurrent operations with a semaphore:

```rust
use std::sync::Arc;
use tokio::sync::Semaphore;

async fn process_urls(urls: Vec<String>, max_concurrent: usize) {
    let semaphore = Arc::new(Semaphore::new(max_concurrent));
    let mut handles = Vec::new();

    for url in urls {
        let permit = semaphore.clone().acquire_owned().await.unwrap();
        handles.push(tokio::spawn(async move {
            let result = reqwest::get(&url).await;
            drop(permit);
            result
        }));
    }

    for handle in handles {
        let _ = handle.await;
    }
}
```

## Graceful Shutdown

Handle shutdown signals cleanly:

```rust
use tokio::signal;

async fn run_server() {
    let app = build_app();

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .unwrap();
}

async fn shutdown_signal() {
    signal::ctrl_c()
        .await
        .expect("failed to install Ctrl+C handler");
    println!("Shutting down gracefully...");
}
```

## Summary

Use `tokio::join!` for concurrent independent operations, `tokio::select!` for racing futures, and channels for task communication. Semaphores control concurrency limits. Always plan for graceful shutdown in production services.
