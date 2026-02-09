+++
title = "Building Servers"
weight = 8
template = "book.html"
+++

# Building Servers

Rust's async ecosystem makes it well-suited for building high-performance web servers. This chapter covers building HTTP APIs with `axum`, the most popular async web framework.

## Hello World Server

```rust
use axum::{routing::get, Router};

#[tokio::main]
async fn main() {
    let app = Router::new().route("/", get(|| async { "Hello, World!" }));

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
```

## Route Handlers

Handlers are async functions that extract data from requests and return responses:

```rust
use axum::{extract::Path, http::StatusCode, Json};
use serde::{Deserialize, Serialize};

#[derive(Serialize)]
struct User {
    id: u64,
    name: String,
}

async fn get_user(Path(id): Path<u64>) -> Json<User> {
    Json(User {
        id,
        name: "Alice".to_string(),
    })
}

#[derive(Deserialize)]
struct CreateUserRequest {
    name: String,
    email: String,
}

async fn create_user(
    Json(payload): Json<CreateUserRequest>,
) -> (StatusCode, Json<User>) {
    let user = User {
        id: 1,
        name: payload.name,
    };
    (StatusCode::CREATED, Json(user))
}
```

## Application State

Share state across handlers using `State`:

```rust
use axum::{extract::State, routing::get, Router};
use std::sync::Arc;
use tokio::sync::RwLock;

struct AppState {
    counter: RwLock<u64>,
    db_pool: sqlx::PgPool,
}

async fn increment(State(state): State<Arc<AppState>>) -> String {
    let mut counter = state.counter.write().await;
    *counter += 1;
    format!("Count: {counter}")
}

#[tokio::main]
async fn main() {
    let state = Arc::new(AppState {
        counter: RwLock::new(0),
        db_pool: create_pool().await,
    });

    let app = Router::new()
        .route("/increment", get(increment))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
```

## Middleware

Add cross-cutting concerns with middleware layers:

```rust
use axum::{middleware, Router};
use std::time::Instant;

async fn timing_middleware(
    request: axum::extract::Request,
    next: middleware::Next,
) -> axum::response::Response {
    let start = Instant::now();
    let response = next.run(request).await;
    let duration = start.elapsed();
    println!("Request took {duration:?}");
    response
}

let app = Router::new()
    .route("/", get(handler))
    .layer(middleware::from_fn(timing_middleware));
```

## Error Handling

Return proper error responses:

```rust
use axum::{http::StatusCode, response::IntoResponse, Json};
use serde_json::json;

enum AppError {
    NotFound(String),
    Internal(String),
}

impl IntoResponse for AppError {
    fn into_response(self) -> axum::response::Response {
        let (status, message) = match self {
            AppError::NotFound(resource) => {
                (StatusCode::NOT_FOUND, format!("{resource} not found"))
            }
            AppError::Internal(message) => {
                (StatusCode::INTERNAL_SERVER_ERROR, message)
            }
        };

        let body = Json(json!({ "error": message }));
        (status, body).into_response()
    }
}

async fn get_user(Path(id): Path<u64>) -> Result<Json<User>, AppError> {
    find_user(id)
        .await
        .map(Json)
        .ok_or_else(|| AppError::NotFound(format!("User {id}")))
}
```

## Summary

Axum provides a type-safe, composable approach to building web servers. Its extractor system makes request parsing explicit and its tower middleware integration gives access to a rich ecosystem of reusable middleware layers.
