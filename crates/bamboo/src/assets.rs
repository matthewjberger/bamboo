use std::collections::HashMap;
use std::fs;
use std::path::Path;

use sha2::{Digest, Sha256};
use walkdir::WalkDir;

use crate::error::Result;

pub struct AssetConfig {
    pub minify: bool,
    pub fingerprint: bool,
    pub base_url: String,
}

pub fn process_assets(output_dir: &Path, config: &AssetConfig) -> Result<HashMap<String, String>> {
    if config.minify {
        minify_css_files(output_dir)?;
        minify_js_files(output_dir)?;
    }

    let mut path_mapping = HashMap::new();

    if config.fingerprint {
        path_mapping = fingerprint_assets(output_dir)?;
        update_html_references(output_dir, &path_mapping, &config.base_url)?;
    }

    if config.minify {
        minify_html_files(output_dir)?;
    }

    Ok(path_mapping)
}

fn collect_files_with_extension(
    directory: &Path,
    extension: &str,
) -> Result<Vec<std::path::PathBuf>> {
    let mut files = Vec::new();
    for entry in WalkDir::new(directory) {
        let entry = entry.map_err(|error| crate::error::BambooError::WalkDir {
            path: directory.to_path_buf(),
            message: error.to_string(),
        })?;
        if entry.file_type().is_file()
            && let Some(file_extension) = entry.path().extension()
            && file_extension == extension
        {
            files.push(entry.path().to_path_buf());
        }
    }
    Ok(files)
}

fn fingerprint_assets(output_dir: &Path) -> Result<HashMap<String, String>> {
    let mut path_mapping = HashMap::new();

    let css_files = collect_files_with_extension(output_dir, "css")?;
    let js_files = collect_files_with_extension(output_dir, "js")?;

    let all_files = css_files.into_iter().chain(js_files);

    for file_path in all_files {
        let content = fs::read(&file_path)?;

        let mut hasher = Sha256::new();
        hasher.update(&content);
        let hash_result = hasher.finalize();
        let hash_hex = format!("{:x}", hash_result);
        let short_hash = &hash_hex[..8];

        let stem = file_path
            .file_stem()
            .and_then(|stem| stem.to_str())
            .unwrap_or("unknown");
        let extension = file_path
            .extension()
            .and_then(|extension| extension.to_str())
            .unwrap_or("");

        let fingerprinted_name = format!("{}.{}.{}", stem, short_hash, extension);
        let fingerprinted_path = file_path.with_file_name(&fingerprinted_name);

        let original_relative = file_path
            .strip_prefix(output_dir)
            .unwrap_or(&file_path)
            .to_string_lossy()
            .replace('\\', "/");

        let fingerprinted_relative = fingerprinted_path
            .strip_prefix(output_dir)
            .unwrap_or(&fingerprinted_path)
            .to_string_lossy()
            .replace('\\', "/");

        fs::rename(&file_path, &fingerprinted_path)?;

        path_mapping.insert(original_relative, fingerprinted_relative);
    }

    Ok(path_mapping)
}

fn html_escape_url(url: &str) -> String {
    url.replace('/', "&#x2F;")
}

fn update_html_references(
    output_dir: &Path,
    path_mapping: &HashMap<String, String>,
    base_url: &str,
) -> Result<()> {
    if path_mapping.is_empty() {
        return Ok(());
    }

    let base_url = base_url.trim_end_matches('/');
    let escaped_base_url = html_escape_url(base_url);

    let mut sorted_mappings: Vec<(&String, &String)> = path_mapping.iter().collect();
    sorted_mappings.sort_by(|a, b| b.0.len().cmp(&a.0.len()));

    let html_files = collect_files_with_extension(output_dir, "html")?;
    let xml_files = collect_files_with_extension(output_dir, "xml")?;
    let all_files = html_files.into_iter().chain(xml_files);

    for file_path in all_files {
        let content = fs::read_to_string(&file_path)?;
        let mut updated = content.clone();

        for (original_path, fingerprinted_path) in &sorted_mappings {
            for delimiter in ['"', '\''] {
                let search_escaped_base_url =
                    format!("={delimiter}{escaped_base_url}/{original_path}{delimiter}");
                let replacement_escaped_base_url =
                    format!("={delimiter}{escaped_base_url}/{fingerprinted_path}{delimiter}");
                updated = updated.replace(&search_escaped_base_url, &replacement_escaped_base_url);

                let search_base_url = format!("={delimiter}{base_url}/{original_path}{delimiter}");
                let replacement_base_url =
                    format!("={delimiter}{base_url}/{fingerprinted_path}{delimiter}");
                updated = updated.replace(&search_base_url, &replacement_base_url);

                let search_absolute = format!("={delimiter}/{original_path}{delimiter}");
                let replacement_absolute = format!("={delimiter}/{fingerprinted_path}{delimiter}");
                updated = updated.replace(&search_absolute, &replacement_absolute);

                let search_relative = format!("={delimiter}{original_path}{delimiter}");
                let replacement_relative = format!("={delimiter}{fingerprinted_path}{delimiter}");
                updated = updated.replace(&search_relative, &replacement_relative);
            }
        }

        if updated != content {
            fs::write(&file_path, updated)?;
        }
    }

    Ok(())
}

fn minify_css_files(output_dir: &Path) -> Result<()> {
    use lightningcss::stylesheet::{MinifyOptions, ParserOptions, PrinterOptions, StyleSheet};
    let css_files = collect_files_with_extension(output_dir, "css")?;
    for file_path in css_files {
        let source = fs::read_to_string(&file_path)?;
        let mut stylesheet = StyleSheet::parse(&source, ParserOptions::default())
            .map_err(|error| std::io::Error::other(error.to_string()))?;
        stylesheet
            .minify(MinifyOptions::default())
            .map_err(|error| std::io::Error::other(error.to_string()))?;
        let result = stylesheet
            .to_css(PrinterOptions {
                minify: true,
                ..Default::default()
            })
            .map_err(|error| std::io::Error::other(error.to_string()))?;
        fs::write(&file_path, result.code)?;
    }
    Ok(())
}

fn minify_js_files(output_dir: &Path) -> Result<()> {
    let js_files = collect_files_with_extension(output_dir, "js")?;
    let session = minify_js::Session::new();
    for file_path in js_files {
        let source = fs::read(&file_path)?;
        let mut output = Vec::new();
        match minify_js::minify(
            &session,
            minify_js::TopLevelMode::Global,
            &source,
            &mut output,
        ) {
            Ok(()) => {
                fs::write(&file_path, output)?;
            }
            Err(error) => {
                eprintln!(
                    "Warning: failed to minify {}: {}",
                    file_path.display(),
                    error
                );
            }
        }
    }
    Ok(())
}

fn minify_html_files(output_dir: &Path) -> Result<()> {
    let html_files = collect_files_with_extension(output_dir, "html")?;

    let mut cfg = minify_html::Cfg::new();
    cfg.minify_css = true;
    cfg.minify_js = true;
    cfg.keep_closing_tags = true;

    for file_path in html_files {
        let content = fs::read(&file_path)?;
        let minified = minify_html::minify(&content, &cfg);
        fs::write(&file_path, minified)?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_fingerprint_renaming() {
        let dir = tempfile::TempDir::new().unwrap();
        fs::write(dir.path().join("style.css"), "body { color: red; }").unwrap();

        let mapping = fingerprint_assets(dir.path()).unwrap();
        assert_eq!(mapping.len(), 1);

        let (original, fingerprinted) = mapping.iter().next().unwrap();
        assert_eq!(original, "style.css");
        assert!(fingerprinted.starts_with("style."));
        assert!(fingerprinted.ends_with(".css"));
        assert!(fingerprinted.len() > "style..css".len());
    }

    #[test]
    fn test_html_reference_rewriting() {
        let dir = tempfile::TempDir::new().unwrap();
        fs::write(dir.path().join("style.css"), "body{}").unwrap();
        fs::write(
            dir.path().join("index.html"),
            r#"<link rel="stylesheet" href="/style.css">"#,
        )
        .unwrap();

        let mapping = fingerprint_assets(dir.path()).unwrap();
        update_html_references(dir.path(), &mapping, "https://example.com").unwrap();

        let html = fs::read_to_string(dir.path().join("index.html")).unwrap();
        assert!(!html.contains("style.css\""));
        let (_, fingerprinted) = mapping.iter().next().unwrap();
        assert!(html.contains(fingerprinted.as_str()));
    }

    #[test]
    fn test_css_minification() {
        let dir = tempfile::TempDir::new().unwrap();
        fs::write(
            dir.path().join("test.css"),
            "body {\n  color: red;\n  margin: 0;\n}\n",
        )
        .unwrap();

        minify_css_files(dir.path()).unwrap();

        let minified = fs::read_to_string(dir.path().join("test.css")).unwrap();
        assert!(!minified.contains('\n'));
        assert!(minified.contains("color"));
    }

    #[test]
    fn test_js_minification() {
        let dir = tempfile::TempDir::new().unwrap();
        fs::write(
            dir.path().join("test.js"),
            "function hello() {\n  var x = 1;\n  return x;\n}\n",
        )
        .unwrap();

        minify_js_files(dir.path()).unwrap();

        let minified = fs::read_to_string(dir.path().join("test.js")).unwrap();
        assert!(minified.len() < "function hello() {\n  var x = 1;\n  return x;\n}\n".len());
    }

    #[test]
    fn test_html_minification() {
        let dir = tempfile::TempDir::new().unwrap();
        fs::write(
            dir.path().join("test.html"),
            "<html>\n  <body>\n    <p>Hello</p>\n  </body>\n</html>",
        )
        .unwrap();

        minify_html_files(dir.path()).unwrap();

        let minified = fs::read_to_string(dir.path().join("test.html")).unwrap();
        assert!(minified.len() < "<html>\n  <body>\n    <p>Hello</p>\n  </body>\n</html>".len());
        assert!(minified.contains("Hello"));
    }
}
