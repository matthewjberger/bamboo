+++
title = "Type System Patterns"
weight = 3
template = "book.html"
+++

# Type System Patterns

Rust's type system is one of the most expressive among systems programming languages. This chapter covers practical patterns that leverage the type system for safety and clarity.

## Newtype Pattern

Wrap primitive types to add semantic meaning and prevent mix-ups:

```rust
struct Meters(f64);
struct Seconds(f64);

fn speed(distance: Meters, time: Seconds) -> f64 {
    distance.0 / time.0
}

let distance = Meters(100.0);
let time = Seconds(9.58);
let velocity = speed(distance, time);
```

This prevents accidentally passing meters where seconds are expected — the compiler catches it.

## Builder Pattern

Use the builder pattern for types with many optional fields:

```rust
struct Server {
    host: String,
    port: u16,
    max_connections: usize,
    timeout_seconds: u64,
}

struct ServerBuilder {
    host: String,
    port: u16,
    max_connections: usize,
    timeout_seconds: u64,
}

impl ServerBuilder {
    fn new(host: impl Into<String>, port: u16) -> Self {
        Self {
            host: host.into(),
            port,
            max_connections: 100,
            timeout_seconds: 30,
        }
    }

    fn max_connections(mut self, count: usize) -> Self {
        self.max_connections = count;
        self
    }

    fn timeout(mut self, seconds: u64) -> Self {
        self.timeout_seconds = seconds;
        self
    }

    fn build(self) -> Server {
        Server {
            host: self.host,
            port: self.port,
            max_connections: self.max_connections,
            timeout_seconds: self.timeout_seconds,
        }
    }
}

let server = ServerBuilder::new("localhost", 8080)
    .max_connections(500)
    .timeout(60)
    .build();
```

## Typestate Pattern

Encode state machines in the type system so invalid transitions are compile-time errors:

```rust
struct Draft;
struct Review;
struct Published;

struct Article<State> {
    title: String,
    body: String,
    _state: std::marker::PhantomData<State>,
}

impl Article<Draft> {
    fn new(title: String, body: String) -> Self {
        Self {
            title,
            body,
            _state: std::marker::PhantomData,
        }
    }

    fn submit_for_review(self) -> Article<Review> {
        Article {
            title: self.title,
            body: self.body,
            _state: std::marker::PhantomData,
        }
    }
}

impl Article<Review> {
    fn approve(self) -> Article<Published> {
        Article {
            title: self.title,
            body: self.body,
            _state: std::marker::PhantomData,
        }
    }

    fn reject(self) -> Article<Draft> {
        Article {
            title: self.title,
            body: self.body,
            _state: std::marker::PhantomData,
        }
    }
}

impl Article<Published> {
    fn url(&self) -> String {
        format!("/articles/{}", self.title.to_lowercase().replace(' ', "-"))
    }
}
```

## Trait Objects vs Generics

Use generics for static dispatch (faster, monomorphized) and trait objects for dynamic dispatch (flexible, smaller binaries):

```rust
trait Renderer {
    fn render(&self, content: &str) -> String;
}

// Static dispatch — resolved at compile time
fn render_static<R: Renderer>(renderer: &R, content: &str) -> String {
    renderer.render(content)
}

// Dynamic dispatch — resolved at runtime
fn render_dynamic(renderer: &dyn Renderer, content: &str) -> String {
    renderer.render(content)
}

// Owned trait objects for collections of mixed types
fn render_all(renderers: &[Box<dyn Renderer>], content: &str) -> Vec<String> {
    renderers.iter().map(|renderer| renderer.render(content)).collect()
}
```

## Enums as Algebraic Data Types

Rust enums model sum types — values that can be one of several variants:

```rust
enum Command {
    Quit,
    Echo(String),
    Move { x: f64, y: f64 },
    Color(u8, u8, u8),
}

fn execute(command: Command) {
    match command {
        Command::Quit => std::process::exit(0),
        Command::Echo(message) => println!("{message}"),
        Command::Move { x, y } => println!("Moving to ({x}, {y})"),
        Command::Color(red, green, blue) => {
            println!("Setting color to rgb({red}, {green}, {blue})")
        }
    }
}
```

## Summary

These patterns leverage Rust's type system to catch bugs at compile time rather than runtime. The newtype and typestate patterns are particularly powerful — they encode domain rules directly in the type system, making invalid states unrepresentable.
