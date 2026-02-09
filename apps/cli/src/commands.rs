use axum::Router;
use axum::body::Body;
use axum::http::{Request, Response, StatusCode};
use axum::middleware::{self, Next};
use bamboo_ssg::{
    BuildState, SiteBuilder, ThemeEngine, classify_changes, clean_output_dir,
    compute_content_hashes, expand_targets, load_cache, save_cache,
};
use notify::{Config, RecommendedWatcher, RecursiveMode, Watcher};
use std::fs;
use std::net::SocketAddr;
use std::path::Path;
use std::sync::mpsc::{RecvTimeoutError, channel};
use std::sync::{Arc, Mutex};
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

fn build_site_incremental(
    theme: &str,
    input: &Path,
    output: &Path,
    drafts: bool,
    base_url: Option<&str>,
    cached_state: Option<&BuildState>,
) -> std::result::Result<BuildState, Box<dyn std::error::Error>> {
    let start = Instant::now();

    let new_hashes = compute_content_hashes(input)?;

    let targets = if let Some(previous_state) = cached_state {
        let classification = classify_changes(&previous_state.content_hashes, &new_hashes);
        let target_set = expand_targets(&classification);
        if target_set.is_empty() {
            println!("No changes detected, skipping rebuild.");
            return Ok(BuildState {
                content_hashes: new_hashes,
            });
        }
        if target_set.contains(&bamboo_ssg::RenderTarget::All) {
            clean_output_dir(output)?;
        }
        Some(target_set)
    } else {
        None
    };

    let is_incremental = targets.is_some()
        && !targets
            .as_ref()
            .is_some_and(|t| t.contains(&bamboo_ssg::RenderTarget::All));

    if is_incremental {
        println!("Incremental rebuild...");
    } else {
        println!("Building site...");
    }

    let mut builder = SiteBuilder::new(input).include_drafts(drafts);

    if let Some(url) = base_url {
        builder = builder.base_url(url);
    }

    let mut shortcode_dirs = Vec::new();
    let site_shortcodes = input.join("templates").join("shortcodes");
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

    let override_dir = input.to_path_buf();
    let theme_engine = ThemeEngine::new_with_overrides(theme, &override_dir)?;
    theme_engine.render_site_with_targets(&site, output, targets.as_ref())?;

    let elapsed = start.elapsed();
    println!(
        "Built {} pages, {} posts to {} in {:.2?}",
        site.pages.len(),
        site.posts.len(),
        output.display(),
        elapsed
    );

    Ok(BuildState {
        content_hashes: new_hashes,
    })
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
    let error_state: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));

    let input_dir = input.unwrap_or(Path::new(".")).to_path_buf();

    if clean {
        clean_output_dir(output)?;
    }

    let initial_cache = if clean { None } else { load_cache(&input_dir) };
    let cached_state: Arc<Mutex<Option<BuildState>>> = Arc::new(Mutex::new(None));

    match build_site_incremental(
        theme,
        &input_dir,
        output,
        drafts,
        Some(&serve_base_url),
        initial_cache.as_ref(),
    ) {
        Ok(new_state) => {
            let _ = save_cache(&input_dir, &new_state);
            if let Ok(mut guard) = cached_state.lock() {
                *guard = Some(new_state);
            }
        }
        Err(error) => {
            eprintln!("Initial build error: {error}");
            if let Ok(mut guard) = error_state.lock() {
                *guard = Some(error.to_string());
            }
        }
    }

    let output_dir = output.to_path_buf();
    let theme_str = theme.to_string();

    let (reload_tx, _) = broadcast::channel::<()>(16);
    let reload_tx = Arc::new(reload_tx);
    let reload_tx_clone = reload_tx.clone();
    let error_state_clone = error_state.clone();
    let cached_state_clone = cached_state.clone();
    let input_dir_clone = input_dir.clone();

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

                    let previous_state = cached_state_clone
                        .lock()
                        .ok()
                        .and_then(|guard| guard.clone());

                    match build_site_incremental(
                        &theme_str,
                        &input_dir_clone,
                        &output_dir,
                        drafts,
                        Some(&serve_url),
                        previous_state.as_ref(),
                    ) {
                        Ok(new_state) => {
                            let _ = save_cache(&input_dir_clone, &new_state);
                            if let Ok(mut guard) = cached_state_clone.lock() {
                                *guard = Some(new_state);
                            }
                            if let Ok(mut guard) = error_state_clone.lock() {
                                *guard = None;
                            }
                        }
                        Err(error) => {
                            eprintln!("Rebuild error: {error}");
                            if let Ok(mut guard) = error_state_clone.lock() {
                                *guard = Some(error.to_string());
                            }
                        }
                    }
                    let _ = reload_tx_clone.send(());
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

    let app = Router::new()
        .fallback_service(serve_dir)
        .layer(middleware::from_fn(move |request, next| {
            error_overlay_middleware(error_state.clone(), request, next)
        }))
        .layer(livereload);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

fn build_error_overlay(error_message: &str) -> String {
    let escaped_message = error_message
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;");
    format!(
        r#"<!DOCTYPE html>
<html>
<head>
<meta charset="UTF-8">
<title>Build Error</title>
<style>
body {{
    margin: 0;
    padding: 0;
    background: #1a1a2e;
    color: #e0e0e0;
    font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
    display: flex;
    align-items: center;
    justify-content: center;
    min-height: 100vh;
}}
.overlay {{
    max-width: 800px;
    width: 90%;
    padding: 2rem;
}}
.header {{
    display: flex;
    align-items: center;
    gap: 0.75rem;
    margin-bottom: 1.5rem;
}}
.indicator {{
    width: 12px;
    height: 12px;
    background: #e74c3c;
    border-radius: 50%;
    animation: pulse 2s infinite;
}}
@keyframes pulse {{
    0%, 100% {{ opacity: 1; }}
    50% {{ opacity: 0.5; }}
}}
h1 {{
    margin: 0;
    font-size: 1.5rem;
    color: #e74c3c;
    font-weight: 600;
}}
.error-box {{
    background: #16213e;
    border: 1px solid #e74c3c33;
    border-left: 4px solid #e74c3c;
    border-radius: 6px;
    padding: 1.5rem;
    overflow-x: auto;
}}
.error-box pre {{
    margin: 0;
    font-family: 'JetBrains Mono', 'Fira Code', 'Cascadia Code', monospace;
    font-size: 0.875rem;
    line-height: 1.6;
    white-space: pre-wrap;
    word-break: break-word;
}}
.hint {{
    margin-top: 1.5rem;
    color: #888;
    font-size: 0.875rem;
}}
</style>
</head>
<body>
<div class="overlay">
    <div class="header">
        <div class="indicator"></div>
        <h1>Build Error</h1>
    </div>
    <div class="error-box">
        <pre>{escaped_message}</pre>
    </div>
    <p class="hint">This page will automatically refresh when the error is fixed.</p>
</div>
</body>
</html>
"#
    )
}

async fn error_overlay_middleware(
    error_state: Arc<Mutex<Option<String>>>,
    request: Request<Body>,
    next: Next,
) -> Response<Body> {
    if let Ok(guard) = error_state.lock()
        && let Some(ref error_message) = *guard
    {
        let html = build_error_overlay(error_message);
        return Response::builder()
            .status(StatusCode::INTERNAL_SERVER_ERROR)
            .header("content-type", "text/html; charset=utf-8")
            .body(Body::from(html))
            .unwrap();
    }
    next.run(request).await
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
