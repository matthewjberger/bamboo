+++
title = "Serialization"
weight = 4
template = "book.html"
+++

# Serialization

Serde is Rust's standard framework for serializing and deserializing data. It supports JSON, TOML, YAML, MessagePack, and dozens of other formats through a unified interface.

## Deriving Serialize and Deserialize

Add `serde` to your `Cargo.toml` and derive the traits:

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
struct User {
    name: String,
    email: String,
    age: u32,
    #[serde(default)]
    admin: bool,
}
```

## Working with JSON

```rust
use serde::{Deserialize, Serialize};
use serde_json;

#[derive(Debug, Serialize, Deserialize)]
struct Config {
    database_url: String,
    port: u16,
    debug: bool,
}

fn main() -> Result<(), serde_json::Error> {
    let config = Config {
        database_url: "postgres://localhost/mydb".to_string(),
        port: 5432,
        debug: true,
    };

    let json = serde_json::to_string_pretty(&config)?;
    println!("{json}");

    let parsed: Config = serde_json::from_str(&json)?;
    println!("{parsed:?}");

    Ok(())
}
```

## Working with TOML

TOML is popular for configuration files:

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
struct AppConfig {
    title: String,
    #[serde(default = "default_port")]
    port: u16,
    database: DatabaseConfig,
}

#[derive(Debug, Serialize, Deserialize)]
struct DatabaseConfig {
    url: String,
    max_connections: u32,
}

fn default_port() -> u16 {
    8080
}

fn load_config(path: &str) -> Result<AppConfig, Box<dyn std::error::Error>> {
    let contents = std::fs::read_to_string(path)?;
    let config: AppConfig = toml::from_str(&contents)?;
    Ok(config)
}
```

## Serde Attributes

Control serialization behavior with attributes:

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ApiResponse {
    status_code: u16,
    error_message: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    retry_after: Option<u64>,

    #[serde(rename = "type")]
    response_type: String,

    #[serde(flatten)]
    metadata: std::collections::HashMap<String, serde_json::Value>,
}
```

## Enum Serialization

Serde supports multiple enum representations:

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
enum Event {
    #[serde(rename = "user_created")]
    UserCreated { id: u64, name: String },

    #[serde(rename = "order_placed")]
    OrderPlaced { id: u64, total: f64 },

    #[serde(rename = "system_alert")]
    SystemAlert(String),
}
```

## Custom Serialization

For special cases, implement custom serialization logic:

```rust
use serde::{self, Deserialize, Deserializer, Serializer};

fn serialize_uppercase<S>(value: &str, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_str(&value.to_uppercase())
}

fn deserialize_lowercase<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    let value = String::deserialize(deserializer)?;
    Ok(value.to_lowercase())
}
```

## Summary

Serde's derive macros handle the vast majority of serialization needs. Use attributes to customize field names, skip fields, and control enum representations. Only reach for custom serialization when the built-in options aren't sufficient.
