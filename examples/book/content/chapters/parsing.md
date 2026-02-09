+++
title = "Parsing Text"
weight = 6
template = "book.html"
+++

# Parsing Text

From simple string splitting to full grammar parsers, Rust has tools for every level of text parsing complexity.

## String Methods

For simple cases, the standard library is often sufficient:

```rust
let csv_line = "Alice,95,A";
let fields: Vec<&str> = csv_line.split(',').collect();
assert_eq!(fields, vec!["Alice", "95", "A"]);

let trimmed = "  hello world  ".trim();
assert_eq!(trimmed, "hello world");

let replaced = "foo-bar-baz".replace('-', "_");
assert_eq!(replaced, "foo_bar_baz");
```

## Parsing Numbers

Convert strings to numeric types with `parse`:

```rust
let port: u16 = "8080".parse().expect("valid port number");
let temperature: f64 = "-12.5".parse().expect("valid temperature");

// Handling parse errors gracefully
fn parse_optional_int(input: &str) -> Option<i64> {
    input.trim().parse().ok()
}
```

## Regular Expressions

Use the `regex` crate for pattern matching:

```rust
use regex::Regex;

let pattern = Regex::new(r"(\d{4})-(\d{2})-(\d{2})").unwrap();

if let Some(captures) = pattern.captures("Date: 2024-03-15") {
    let year = &captures[1];
    let month = &captures[2];
    let day = &captures[3];
    println!("{year}-{month}-{day}");
}

let dates: Vec<&str> = pattern
    .find_iter("Events on 2024-01-01 and 2024-06-15")
    .map(|matched| matched.as_str())
    .collect();
```

## Parsing Structured Formats

### CSV

```rust
use csv::ReaderBuilder;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct Record {
    name: String,
    score: f64,
    grade: String,
}

fn read_scores(path: &str) -> Result<Vec<Record>, Box<dyn std::error::Error>> {
    let mut reader = ReaderBuilder::new()
        .has_headers(true)
        .from_path(path)?;

    let mut records = Vec::new();
    for result in reader.deserialize() {
        let record: Record = result?;
        records.push(record);
    }
    Ok(records)
}
```

### Log Parsing

A practical example of parsing Apache-style log lines:

```rust
fn parse_log_line(line: &str) -> Option<LogEntry> {
    let mut parts = line.splitn(4, ' ');

    let ip = parts.next()?;
    let timestamp = parts.next()?.trim_matches(|character| character == '[' || character == ']');
    let method_and_path = parts.next()?;
    let status_str = parts.next()?;

    let status: u16 = status_str.trim().parse().ok()?;

    Some(LogEntry {
        ip: ip.to_string(),
        timestamp: timestamp.to_string(),
        request: method_and_path.to_string(),
        status,
    })
}

struct LogEntry {
    ip: String,
    timestamp: String,
    request: String,
    status: u16,
}
```

## Nom — Parser Combinators

For complex grammars, `nom` lets you build parsers from small, composable pieces:

```rust
use nom::{
    bytes::complete::tag,
    character::complete::{alpha1, digit1},
    sequence::separated_pair,
    IResult,
};

fn parse_key_value(input: &str) -> IResult<&str, (&str, &str)> {
    separated_pair(alpha1, tag("="), digit1)(input)
}

let (remaining, (key, value)) = parse_key_value("port=8080").unwrap();
assert_eq!(key, "port");
assert_eq!(value, "8080");
assert_eq!(remaining, "");
```

## Summary

Start with string methods for simple cases. Reach for `regex` when patterns get complex. Use `serde` with format-specific crates for structured data (CSV, JSON, TOML). For custom grammars or binary formats, parser combinator libraries like `nom` or `winnow` are the right tools.
