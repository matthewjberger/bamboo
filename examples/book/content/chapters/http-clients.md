+++
title = "HTTP Clients"
weight = 7
template = "book.html"
+++

# HTTP Clients

The `reqwest` crate is the most popular HTTP client for Rust, supporting both blocking and async interfaces, TLS, cookies, and connection pooling.

## Simple GET Request

```rust
use reqwest;

#[tokio::main]
async fn main() -> Result<(), reqwest::Error> {
    let body = reqwest::get("https://httpbin.org/get")
        .await?
        .text()
        .await?;

    println!("Response: {body}");
    Ok(())
}
```

## JSON Requests and Responses

```rust
use reqwest;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize)]
struct CreateUser {
    name: String,
    email: String,
}

#[derive(Debug, Deserialize)]
struct UserResponse {
    id: u64,
    name: String,
    email: String,
}

async fn create_user(user: &CreateUser) -> Result<UserResponse, reqwest::Error> {
    let client = reqwest::Client::new();
    let response = client
        .post("https://api.example.com/users")
        .json(user)
        .send()
        .await?
        .json::<UserResponse>()
        .await?;

    Ok(response)
}
```

## Setting Headers and Authentication

```rust
use reqwest::header::{self, HeaderMap, HeaderValue};

fn build_client(api_key: &str) -> reqwest::Client {
    let mut headers = HeaderMap::new();
    headers.insert(
        header::AUTHORIZATION,
        HeaderValue::from_str(&format!("Bearer {api_key}")).unwrap(),
    );
    headers.insert(
        header::ACCEPT,
        HeaderValue::from_static("application/json"),
    );

    reqwest::Client::builder()
        .default_headers(headers)
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .unwrap()
}
```

## Error Handling for HTTP

Check status codes and handle errors:

```rust
use reqwest::StatusCode;

async fn fetch_user(id: u64) -> Result<User, AppError> {
    let response = reqwest::get(format!("https://api.example.com/users/{id}"))
        .await
        .map_err(AppError::Network)?;

    match response.status() {
        StatusCode::OK => {
            let user = response.json().await.map_err(AppError::Parse)?;
            Ok(user)
        }
        StatusCode::NOT_FOUND => Err(AppError::NotFound(id)),
        status => Err(AppError::UnexpectedStatus(status.as_u16())),
    }
}
```

## Downloading Files

Stream large files to disk without loading them entirely into memory:

```rust
use std::io::Write;
use reqwest;
use std::fs::File;

async fn download_file(url: &str, path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let response = reqwest::get(url).await?;
    let bytes = response.bytes().await?;

    let mut file = File::create(path)?;
    file.write_all(&bytes)?;

    Ok(())
}
```

## Retry Logic

Implement simple retry with exponential backoff:

```rust
use std::time::Duration;
use tokio::time::sleep;

async fn fetch_with_retry(url: &str, max_retries: u32) -> Result<String, reqwest::Error> {
    let mut last_error = None;

    for attempt in 0..max_retries {
        match reqwest::get(url).await {
            Ok(response) if response.status().is_success() => {
                return response.text().await;
            }
            Ok(response) => {
                eprintln!("Attempt {}: status {}", attempt + 1, response.status());
            }
            Err(error) => {
                eprintln!("Attempt {}: {error}", attempt + 1);
                last_error = Some(error);
            }
        }

        let delay = Duration::from_millis(100 * 2u64.pow(attempt));
        sleep(delay).await;
    }

    Err(last_error.unwrap())
}
```

## Summary

Use `reqwest` for HTTP clients in Rust. Create a `Client` instance and reuse it across requests for connection pooling. For production code, always handle status codes explicitly, set timeouts, and consider retry logic for transient failures.
