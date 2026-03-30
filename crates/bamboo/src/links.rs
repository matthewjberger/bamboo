use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

pub struct LinkWarning {
    pub source: PathBuf,
    pub href: String,
}

impl std::fmt::Display for LinkWarning {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let source_display = self.source.to_string_lossy().replace('\\', "/");
        write!(
            formatter,
            "broken link '{}' in {}",
            self.href, source_display
        )
    }
}

pub fn validate_internal_links(output_dir: &Path, base_url: &str) -> Vec<LinkWarning> {
    let mut warnings = Vec::new();
    let mut seen: HashSet<(PathBuf, String)> = HashSet::new();
    let base_url_trimmed = base_url.trim_end_matches('/');

    for entry in WalkDir::new(output_dir)
        .into_iter()
        .filter_map(|entry| entry.ok())
    {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        if path
            .extension()
            .map(|extension| extension != "html")
            .unwrap_or(true)
        {
            continue;
        }

        let content = match fs::read_to_string(path) {
            Ok(content) => content,
            Err(_) => continue,
        };

        let relative = path.strip_prefix(output_dir).unwrap_or(path).to_path_buf();

        for href in extract_hrefs(&content) {
            let normalized = normalize_href(&href, base_url_trimmed);
            let normalized = match normalized {
                Some(normalized) => normalized,
                None => continue,
            };

            let clean = normalized.split('#').next().unwrap_or(normalized);
            if clean.is_empty() {
                continue;
            }

            let key = (relative.clone(), clean.to_string());
            if seen.contains(&key) {
                continue;
            }
            seen.insert(key);

            if !link_resolves(output_dir, clean) {
                warnings.push(LinkWarning {
                    source: relative.clone(),
                    href: clean.to_string(),
                });
            }
        }
    }

    warnings.sort_by(|a, b| a.source.cmp(&b.source).then_with(|| a.href.cmp(&b.href)));

    warnings
}

fn normalize_href<'a>(href: &'a str, base_url: &str) -> Option<&'a str> {
    if href.is_empty() || href.starts_with('#') || href.starts_with("//") {
        return None;
    }
    if href.starts_with("mailto:")
        || href.starts_with("tel:")
        || href.starts_with("javascript:")
        || href.starts_with("data:")
    {
        return None;
    }
    if !base_url.is_empty()
        && let Some(path) = href.strip_prefix(base_url)
        && (path.is_empty() || path.starts_with('/') || path.starts_with('#'))
    {
        return if path.is_empty() {
            Some("/")
        } else {
            Some(path)
        };
    }
    if href.starts_with("http://") || href.starts_with("https://") {
        return None;
    }
    if href.starts_with('/') {
        return Some(href);
    }
    None
}

fn link_resolves(output_dir: &Path, href: &str) -> bool {
    let clean = href.trim_start_matches('/');

    if clean.is_empty() {
        return output_dir.join("index.html").exists();
    }

    let direct = output_dir.join(clean);
    if direct.exists() && direct.is_file() {
        return true;
    }

    let trimmed = clean.trim_end_matches('/');
    output_dir.join(trimmed).join("index.html").exists()
}

fn extract_hrefs(html: &str) -> Vec<String> {
    let mut hrefs = Vec::new();
    let bytes = html.as_bytes();
    let length = bytes.len();
    let mut position = 0;

    while position < length {
        if let Some(offset) = find_subsequence(&bytes[position..], b"href=") {
            position += offset + 5;
            if position >= length {
                break;
            }
            let quote = bytes[position];
            if quote != b'"' && quote != b'\'' {
                continue;
            }
            position += 1;
            let start = position;
            while position < length && bytes[position] != quote {
                position += 1;
            }
            if position < length {
                let href = String::from_utf8_lossy(&bytes[start..position]).to_string();
                hrefs.push(href);
                position += 1;
            }
        } else {
            break;
        }
    }

    hrefs
}

fn find_subsequence(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_extract_hrefs_double_quotes() {
        let html = r#"<a href="/about/">About</a>"#;
        let hrefs = extract_hrefs(html);
        assert_eq!(hrefs, vec!["/about/"]);
    }

    #[test]
    fn test_extract_hrefs_single_quotes() {
        let html = "<a href='/about/'>About</a>";
        let hrefs = extract_hrefs(html);
        assert_eq!(hrefs, vec!["/about/"]);
    }

    #[test]
    fn test_extract_hrefs_multiple() {
        let html = r#"<a href="/a/">A</a><a href="/b/">B</a>"#;
        let hrefs = extract_hrefs(html);
        assert_eq!(hrefs, vec!["/a/", "/b/"]);
    }

    #[test]
    fn test_extract_hrefs_with_external() {
        let html = r#"<a href="https://example.com">Ext</a><a href="/local/">Local</a>"#;
        let hrefs = extract_hrefs(html);
        assert_eq!(hrefs, vec!["https://example.com", "/local/"]);
    }

    #[test]
    fn test_normalize_href_bare_path() {
        assert_eq!(
            normalize_href("/about/", "https://example.com"),
            Some("/about/")
        );
    }

    #[test]
    fn test_normalize_href_with_base_url() {
        assert_eq!(
            normalize_href("https://example.com/about/", "https://example.com"),
            Some("/about/")
        );
    }

    #[test]
    fn test_normalize_href_base_url_root() {
        assert_eq!(
            normalize_href("https://example.com", "https://example.com"),
            Some("/")
        );
        assert_eq!(
            normalize_href("https://example.com/", "https://example.com"),
            Some("/")
        );
    }

    #[test]
    fn test_normalize_href_external() {
        assert_eq!(
            normalize_href("https://other.com/path", "https://example.com"),
            None
        );
    }

    #[test]
    fn test_normalize_href_skips_special_schemes() {
        assert_eq!(normalize_href("mailto:user@example.com", ""), None);
        assert_eq!(normalize_href("tel:+1234567890", ""), None);
        assert_eq!(normalize_href("javascript:void(0)", ""), None);
        assert_eq!(normalize_href("data:text/html,hi", ""), None);
    }

    #[test]
    fn test_normalize_href_anchor_only() {
        assert_eq!(normalize_href("#section", ""), None);
    }

    #[test]
    fn test_normalize_href_empty() {
        assert_eq!(normalize_href("", ""), None);
    }

    #[test]
    fn test_normalize_href_protocol_relative() {
        assert_eq!(normalize_href("//cdn.example.com/js/app.js", ""), None);
    }

    #[test]
    fn test_normalize_href_base_url_with_fragment() {
        assert_eq!(
            normalize_href("https://example.com/about/#team", "https://example.com"),
            Some("/about/#team")
        );
    }

    #[test]
    fn test_link_resolves_root() {
        let dir = tempfile::TempDir::new().unwrap();
        fs::write(dir.path().join("index.html"), "<html></html>").unwrap();
        assert!(link_resolves(dir.path(), "/"));
    }

    #[test]
    fn test_link_resolves_directory_with_index() {
        let dir = tempfile::TempDir::new().unwrap();
        fs::create_dir_all(dir.path().join("about")).unwrap();
        fs::write(dir.path().join("about/index.html"), "<html></html>").unwrap();
        assert!(link_resolves(dir.path(), "/about/"));
        assert!(link_resolves(dir.path(), "/about"));
    }

    #[test]
    fn test_link_resolves_direct_file() {
        let dir = tempfile::TempDir::new().unwrap();
        fs::write(dir.path().join("style.css"), "body {}").unwrap();
        assert!(link_resolves(dir.path(), "/style.css"));
    }

    #[test]
    fn test_link_does_not_resolve() {
        let dir = tempfile::TempDir::new().unwrap();
        fs::write(dir.path().join("index.html"), "<html></html>").unwrap();
        assert!(!link_resolves(dir.path(), "/nonexistent/"));
    }

    #[test]
    fn test_validate_finds_broken() {
        let dir = tempfile::TempDir::new().unwrap();
        fs::write(
            dir.path().join("index.html"),
            r#"<a href="/about/">About</a>"#,
        )
        .unwrap();

        let warnings = validate_internal_links(dir.path(), "");
        assert_eq!(warnings.len(), 1);
        assert_eq!(warnings[0].href, "/about/");
    }

    #[test]
    fn test_validate_no_false_positives() {
        let dir = tempfile::TempDir::new().unwrap();
        fs::create_dir_all(dir.path().join("about")).unwrap();
        fs::write(dir.path().join("about/index.html"), "<html></html>").unwrap();
        fs::write(
            dir.path().join("index.html"),
            r#"<a href="/about/">About</a>"#,
        )
        .unwrap();

        let warnings = validate_internal_links(dir.path(), "");
        assert!(warnings.is_empty());
    }

    #[test]
    fn test_validate_skips_external_links() {
        let dir = tempfile::TempDir::new().unwrap();
        fs::write(
            dir.path().join("index.html"),
            r#"<a href="https://other.com">External</a>"#,
        )
        .unwrap();

        let warnings = validate_internal_links(dir.path(), "https://example.com");
        assert!(warnings.is_empty());
    }

    #[test]
    fn test_validate_skips_anchor_links() {
        let dir = tempfile::TempDir::new().unwrap();
        fs::write(
            dir.path().join("index.html"),
            "<a href=\"#section\">Section</a>",
        )
        .unwrap();

        let warnings = validate_internal_links(dir.path(), "");
        assert!(warnings.is_empty());
    }

    #[test]
    fn test_validate_strips_fragment_before_checking() {
        let dir = tempfile::TempDir::new().unwrap();
        fs::create_dir_all(dir.path().join("about")).unwrap();
        fs::write(dir.path().join("about/index.html"), "<html></html>").unwrap();
        fs::write(
            dir.path().join("index.html"),
            r#"<a href="/about/#team">Team</a>"#,
        )
        .unwrap();

        let warnings = validate_internal_links(dir.path(), "");
        assert!(warnings.is_empty());
    }

    #[test]
    fn test_validate_deduplicates_per_file() {
        let dir = tempfile::TempDir::new().unwrap();
        fs::write(
            dir.path().join("index.html"),
            r#"<a href="/missing/">A</a><a href="/missing/">B</a>"#,
        )
        .unwrap();

        let warnings = validate_internal_links(dir.path(), "");
        assert_eq!(warnings.len(), 1);
    }

    #[test]
    fn test_validate_checks_base_url_prefixed_links() {
        let dir = tempfile::TempDir::new().unwrap();
        fs::write(
            dir.path().join("index.html"),
            r#"<a href="https://example.com/missing/">Link</a>"#,
        )
        .unwrap();

        let warnings = validate_internal_links(dir.path(), "https://example.com");
        assert_eq!(warnings.len(), 1);
        assert_eq!(warnings[0].href, "/missing/");
    }

    #[test]
    fn test_validate_base_url_prefixed_link_resolves() {
        let dir = tempfile::TempDir::new().unwrap();
        fs::create_dir_all(dir.path().join("about")).unwrap();
        fs::write(dir.path().join("about/index.html"), "<html></html>").unwrap();
        fs::write(
            dir.path().join("index.html"),
            r#"<a href="https://example.com/about/">About</a>"#,
        )
        .unwrap();

        let warnings = validate_internal_links(dir.path(), "https://example.com");
        assert!(warnings.is_empty());
    }
}
