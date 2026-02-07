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

fn escape_toml_string(input: &str) -> String {
    let mut output = String::with_capacity(input.len());
    for character in input.chars() {
        match character {
            '\\' => output.push_str("\\\\"),
            '"' => output.push_str("\\\""),
            '\n' => output.push_str("\\n"),
            '\r' => output.push_str("\\r"),
            '\t' => output.push_str("\\t"),
            '\u{0008}' => output.push_str("\\b"),
            '\u{000C}' => output.push_str("\\f"),
            control if control < '\u{0020}' => {
                output.push_str(&format!("\\u{:04X}", control as u32));
            }
            other => output.push(other),
        }
    }
    output
}

pub fn new_site(name: &str) -> Result<(), Box<dyn std::error::Error>> {
    let site_dir = Path::new(name);

    if site_dir.exists() {
        return Err(format!("Directory '{}' already exists", name).into());
    }

    fs::create_dir_all(site_dir.join("content").join("posts"))?;
    fs::create_dir_all(site_dir.join("data"))?;
    fs::create_dir_all(site_dir.join("static").join("images"))?;

    let escaped_name = escape_toml_string(name);
    let config = format!(
        r#"title = "{escaped_name}"
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

    let escaped_name = escape_toml_string(&name);
    let config = format!(
        r#"title = "{escaped_name}"
base_url = "http://localhost:3000"
description = "A new Bamboo site"
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

    let mut shortcode_dirs = Vec::new();
    let site_shortcodes = input_dir.join("templates").join("shortcodes");
    if site_shortcodes.is_dir() {
        shortcode_dirs.push(site_shortcodes);
    }
    let theme_path = std::path::Path::new(theme);
    let theme_shortcodes = theme_path.join("templates").join("shortcodes");
    if theme_shortcodes.is_dir() {
        shortcode_dirs.push(theme_shortcodes);
    }
    if !shortcode_dirs.is_empty() {
        builder = builder.shortcode_dirs(&shortcode_dirs)?;
    }

    let site = builder.build()?;

    let override_dir = input_dir.to_path_buf();
    let theme_engine = ThemeEngine::new_with_overrides(theme, &override_dir)?;
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
    open_browser: bool,
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

    let theme_path = Path::new(&theme_str);
    if theme_path.exists() && theme_path.is_dir() {
        watcher.watch(theme_path, RecursiveMode::Recursive)?;
    }

    let templates_dir = input_dir.join("templates");
    if templates_dir.exists() {
        watcher.watch(&templates_dir, RecursiveMode::Recursive)?;
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
                        true,
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

    if open_browser {
        let url = format!("http://localhost:{}", port);
        if let Err(error) = open::that(&url) {
            eprintln!("Failed to open browser: {error}");
        }
    }

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_escape_toml_string_plain() {
        assert_eq!(escape_toml_string("hello world"), "hello world");
    }

    #[test]
    fn test_escape_toml_string_backslash() {
        assert_eq!(escape_toml_string("path\\to\\file"), "path\\\\to\\\\file");
    }

    #[test]
    fn test_escape_toml_string_quotes() {
        assert_eq!(escape_toml_string("say \"hello\""), "say \\\"hello\\\"");
    }

    #[test]
    fn test_escape_toml_string_newline() {
        assert_eq!(escape_toml_string("line1\nline2"), "line1\\nline2");
    }

    #[test]
    fn test_escape_toml_string_tab() {
        assert_eq!(escape_toml_string("col1\tcol2"), "col1\\tcol2");
    }

    #[test]
    fn test_escape_toml_string_carriage_return() {
        assert_eq!(escape_toml_string("line\r"), "line\\r");
    }

    #[test]
    fn test_escape_toml_string_backspace() {
        assert_eq!(escape_toml_string("back\u{0008}space"), "back\\bspace");
    }

    #[test]
    fn test_escape_toml_string_form_feed() {
        assert_eq!(escape_toml_string("form\u{000C}feed"), "form\\ffeed");
    }

    #[test]
    fn test_escape_toml_string_control_char() {
        assert_eq!(escape_toml_string("null\u{0000}byte"), "null\\u0000byte");
    }

    #[test]
    fn test_escape_toml_string_bell() {
        assert_eq!(escape_toml_string("bell\u{0007}char"), "bell\\u0007char");
    }
}
