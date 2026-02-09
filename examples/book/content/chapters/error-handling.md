+++
title = "Error Handling"
weight = 1
template = "book.html"
+++

# Error Handling

Rust's approach to error handling is one of its most distinctive features. Instead of exceptions, Rust uses `Result<T, E>` and `Option<T>` types to make error cases explicit and impossible to ignore.

## The Result Type

The `Result` enum represents either success or failure:

```rust
use std::fs;
use std::io;

fn read_config(path: &str) -> Result<String, io::Error> {
    fs::read_to_string(path)
}

fn main() {
    match read_config("config.toml") {
        Ok(contents) => println!("Config loaded: {} bytes", contents.len()),
        Err(error) => eprintln!("Failed to read config: {error}"),
    }
}
```

## The ? Operator

The `?` operator provides concise error propagation. It returns early from a function if the `Result` is `Err`, automatically converting the error type if needed:

```rust
use std::fs;
use std::io;

fn read_username() -> Result<String, io::Error> {
    let contents = fs::read_to_string("username.txt")?;
    Ok(contents.trim().to_string())
}
```

## Custom Error Types

For libraries and larger applications, define your own error types:

```rust
use std::fmt;

#[derive(Debug)]
enum AppError {
    NotFound(String),
    PermissionDenied,
    InvalidInput { field: String, message: String },
}

impl fmt::Display for AppError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AppError::NotFound(resource) => write!(formatter, "{resource} not found"),
            AppError::PermissionDenied => write!(formatter, "permission denied"),
            AppError::InvalidInput { field, message } => {
                write!(formatter, "invalid {field}: {message}")
            }
        }
    }
}

impl std::error::Error for AppError {}
```

## Using thiserror

The `thiserror` crate reduces boilerplate when defining error types:

```rust
use thiserror::Error;

#[derive(Error, Debug)]
enum DataError {
    #[error("failed to read file: {0}")]
    Io(#[from] std::io::Error),

    #[error("failed to parse JSON: {0}")]
    Parse(#[from] serde_json::Error),

    #[error("record not found: {id}")]
    NotFound { id: u64 },
}
```

## Using anyhow for Applications

For application code where you don't need callers to match on specific error variants, `anyhow` provides a convenient catch-all:

```rust
use anyhow::{Context, Result};

fn load_config() -> Result<Config> {
    let contents = std::fs::read_to_string("config.toml")
        .context("failed to read config file")?;
    let config: Config = toml::from_str(&contents)
        .context("failed to parse config")?;
    Ok(config)
}
```

## Converting Between Error Types

Implement `From` to enable automatic error conversion with `?`:

```rust
impl From<std::io::Error> for AppError {
    fn from(error: std::io::Error) -> Self {
        match error.kind() {
            std::io::ErrorKind::NotFound => AppError::NotFound("file".to_string()),
            std::io::ErrorKind::PermissionDenied => AppError::PermissionDenied,
            _ => AppError::InvalidInput {
                field: "io".to_string(),
                message: error.to_string(),
            },
        }
    }
}
```

## Summary

| Approach | Best For |
|----------|----------|
| `Result<T, E>` with `match` | When you need to handle each case explicitly |
| `?` operator | Propagating errors up the call stack |
| `thiserror` | Library code with structured error types |
| `anyhow` | Application code where error details matter less |
