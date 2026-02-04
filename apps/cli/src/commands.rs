use axum::Router;
use bamboo_ssg::{SiteBuilder, ThemeEngine, clean_output_dir};
use notify::{Config, RecommendedWatcher, RecursiveMode, Watcher};
use std::fs;
use std::net::SocketAddr;
use std::path::Path;
use std::sync::Arc;
use std::sync::mpsc::{RecvTimeoutError, channel};
use std::time::{Duration, Instant};
use tokio::sync::broadcast;
use tower_http::services::ServeDir;

const DEBOUNCE_DURATION: Duration = Duration::from_millis(300);

pub fn new_site(name: &str) -> Result<(), Box<dyn std::error::Error>> {
    let site_dir = Path::new(name);

    if site_dir.exists() {
        return Err(format!("Directory '{}' already exists", name).into());
    }

    fs::create_dir_all(site_dir.join("content").join("posts"))?;
    fs::create_dir_all(site_dir.join("data"))?;
    fs::create_dir_all(site_dir.join("static").join("images"))?;

    let config = format!(
        r#"title = "{name}"
base_url = "http://localhost:3000"
description = "A new Bamboo site"
language = "en"
"#
    );
    fs::write(site_dir.join("bamboo.toml"), config)?;

    let index_content = r#"+++
title = "Home"
+++

Welcome to your new Bamboo site!
"#;
    fs::write(site_dir.join("content").join("_index.md"), index_content)?;

    let about_content = r#"+++
title = "About"
weight = 10
+++

This is the about page.
"#;
    fs::write(site_dir.join("content").join("about.md"), about_content)?;

    let post_content = r#"+++
title = "Hello World"
tags = ["welcome", "first-post"]
+++

This is your first blog post. Start writing!

You can use **markdown** formatting, including:

- Lists
- Code blocks
- And more!

```rust
fn main() {
    println!("Hello, world!");
}
```
"#;
    fs::write(
        site_dir
            .join("content")
            .join("posts")
            .join("2024-01-01-hello-world.md"),
        post_content,
    )?;

    println!("Created new site: {name}");
    println!("  cd {name}");
    println!("  bamboo serve");

    Ok(())
}

pub fn init_site() -> Result<(), Box<dyn std::error::Error>> {
    let current_dir = std::env::current_dir()?;

    if current_dir.join("bamboo.toml").exists() {
        return Err("bamboo.toml already exists in this directory".into());
    }

    fs::create_dir_all(current_dir.join("content").join("posts"))?;
    fs::create_dir_all(current_dir.join("data"))?;
    fs::create_dir_all(current_dir.join("static"))?;

    let name = current_dir
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "My Site".to_string());

    let config = format!(
        r#"title = "{name}"
base_url = "http://localhost:3000"
language = "en"
"#
    );
    fs::write(current_dir.join("bamboo.toml"), config)?;

    println!("Initialized Bamboo site in current directory");

    Ok(())
}

pub fn build_site(
    theme: &str,
    input: Option<&Path>,
    output: &Path,
    drafts: bool,
    base_url: Option<&str>,
    clean: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let input_dir = input.unwrap_or(Path::new("."));

    if clean {
        clean_output_dir(output)?;
    }

    println!("Building site...");
    let start = Instant::now();

    let mut builder = SiteBuilder::new(input_dir).include_drafts(drafts);

    if let Some(url) = base_url {
        builder = builder.base_url(url);
    }

    let site = builder.build()?;

    let theme_engine = ThemeEngine::new(theme)?;
    theme_engine.render_site(&site, output)?;

    let elapsed = start.elapsed();
    println!(
        "Built {} pages, {} posts to {} in {:.2?}",
        site.pages.len(),
        site.posts.len(),
        output.display(),
        elapsed
    );

    Ok(())
}

pub async fn serve_site(
    theme: &str,
    input: Option<&Path>,
    output: &Path,
    drafts: bool,
    port: u16,
    clean: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let serve_base_url = format!("http://localhost:{}", port);
    build_site(theme, input, output, drafts, Some(&serve_base_url), clean)?;

    let input_dir = input.unwrap_or(Path::new(".")).to_path_buf();
    let output_dir = output.to_path_buf();
    let theme_str = theme.to_string();

    let (reload_tx, _) = broadcast::channel::<()>(16);
    let reload_tx = Arc::new(reload_tx);
    let reload_tx_clone = reload_tx.clone();

    let (notify_tx, notify_rx) = channel();

    let mut watcher = RecommendedWatcher::new(
        notify_tx,
        Config::default().with_poll_interval(Duration::from_millis(200)),
    )?;

    let content_dir = input_dir.join("content");
    let data_dir = input_dir.join("data");
    let static_dir = input_dir.join("static");

    if content_dir.exists() {
        watcher.watch(&content_dir, RecursiveMode::Recursive)?;
    }
    if data_dir.exists() {
        watcher.watch(&data_dir, RecursiveMode::Recursive)?;
    }
    if static_dir.exists() {
        watcher.watch(&static_dir, RecursiveMode::Recursive)?;
    }

    let config_path = input_dir.join("bamboo.toml");
    if config_path.exists() {
        watcher.watch(&config_path, RecursiveMode::NonRecursive)?;
    }

    let serve_url = format!("http://localhost:{}", port);

    std::thread::spawn(move || {
        loop {
            match notify_rx.recv() {
                Ok(_event) => {
                    loop {
                        match notify_rx.recv_timeout(DEBOUNCE_DURATION) {
                            Ok(_) => continue,
                            Err(RecvTimeoutError::Timeout) => break,
                            Err(RecvTimeoutError::Disconnected) => return,
                        }
                    }

                    println!("Changes detected, rebuilding...");

                    if let Err(error) = build_site(
                        &theme_str,
                        Some(&input_dir),
                        &output_dir,
                        drafts,
                        Some(&serve_url),
                        false,
                    ) {
                        eprintln!("Rebuild error: {error}");
                    } else {
                        let _ = reload_tx_clone.send(());
                    }
                }
                Err(error) => {
                    eprintln!("Watch error: {error}");
                    break;
                }
            }
        }
    });

    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    println!("Serving at http://{addr}");
    println!("Press Ctrl+C to stop");

    let livereload = tower_livereload::LiveReloadLayer::new();
    let reloader = livereload.reloader();

    let mut reload_rx = reload_tx.subscribe();
    tokio::spawn(async move {
        loop {
            if reload_rx.recv().await.is_ok() {
                reloader.reload();
            }
        }
    });

    let serve_dir = ServeDir::new(output).append_index_html_on_directories(true);

    let app = Router::new().fallback_service(serve_dir).layer(livereload);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
