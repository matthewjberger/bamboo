use crate::error::Result;
use crate::types::Site;
use std::fs;
use std::path::Path;

const WINDOWS_RESERVED_NAMES: &[&str] = &[
    "con", "prn", "aux", "nul", "com1", "com2", "com3", "com4", "com5", "com6", "com7", "com8",
    "com9", "lpt1", "lpt2", "lpt3", "lpt4", "lpt5", "lpt6", "lpt7", "lpt8", "lpt9",
];

fn has_reserved_component(path_str: &str) -> bool {
    for component in path_str.split('/') {
        let name = component.split('.').next().unwrap_or(component);
        if WINDOWS_RESERVED_NAMES.contains(&name.to_lowercase().as_str()) {
            return true;
        }
    }
    false
}

fn is_safe_redirect_path(clean_path: &str) -> bool {
    if clean_path.is_empty() {
        return false;
    }
    if clean_path.contains("..") {
        return false;
    }
    if clean_path.contains(':') {
        return false;
    }
    let path = Path::new(clean_path);
    if path.is_absolute() {
        return false;
    }
    if clean_path.starts_with('\\') {
        return false;
    }
    if has_reserved_component(clean_path) {
        return false;
    }
    if clean_path.bytes().any(|byte| byte < 0x20) {
        return false;
    }
    true
}

fn build_redirect_html(target_url: &str) -> String {
    let escaped_url = crate::xml::escape(target_url);
    format!(
        r#"<!DOCTYPE html>
<html>
<head>
<meta charset="UTF-8">
<meta http-equiv="refresh" content="0; url={url}">
<link rel="canonical" href="{url}">
<title>Redirecting...</title>
</head>
<body>
<p>Redirecting to <a href="{url}">{url}</a></p>
</body>
</html>
"#,
        url = escaped_url
    )
}

fn write_redirect(output_dir: &Path, redirect_path: &str, target_url: &str) -> Result<()> {
    let clean_path = redirect_path.trim_matches('/');
    if !is_safe_redirect_path(clean_path) {
        return Ok(());
    }
    let redirect_dir = output_dir.join(clean_path);
    if redirect_dir.join("index.html").exists() {
        return Ok(());
    }
    fs::create_dir_all(&redirect_dir)?;
    fs::write(
        redirect_dir.join("index.html"),
        build_redirect_html(target_url),
    )?;
    Ok(())
}

pub fn generate_redirects(site: &Site, output_dir: &Path) -> Result<()> {
    let base_url = site.config.base_url.trim_end_matches('/');

    for post in &site.posts {
        let target_url = format!("{}/posts/{}/", base_url, post.content.slug);
        for redirect_path in &post.redirect_from {
            write_redirect(output_dir, redirect_path, &target_url)?;
        }
    }

    for page in &site.pages {
        let target_url = format!("{}/{}/", base_url, page.content.slug);
        for redirect_path in &page.redirect_from {
            write_redirect(output_dir, redirect_path, &target_url)?;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::*;
    use chrono::{NaiveDate, NaiveTime, TimeZone, Utc};
    use std::collections::HashMap;
    use std::path::PathBuf;

    fn minimal_site() -> Site {
        Site {
            config: SiteConfig {
                title: "Test".to_string(),
                base_url: "https://example.com".to_string(),
                description: None,
                author: None,
                language: None,
                posts_per_page: 10,
                minify: false,
                fingerprint: false,
                images: None,
                extra: HashMap::new(),
            },
            home: None,
            pages: vec![],
            posts: vec![],
            collections: HashMap::new(),
            data: HashMap::new(),
            assets: vec![],
        }
    }

    fn make_date() -> chrono::DateTime<Utc> {
        Utc.from_utc_datetime(
            &NaiveDate::from_ymd_opt(2024, 1, 1)
                .unwrap()
                .and_time(NaiveTime::MIN),
        )
    }

    #[test]
    fn test_post_redirects() {
        let mut site = minimal_site();
        site.posts.push(Post {
            content: Content {
                slug: "new-post".to_string(),
                title: "New Post".to_string(),
                html: String::new(),
                raw_content: String::new(),
                frontmatter: Frontmatter::default(),
                path: PathBuf::from("posts/new-post/index.html"),
                template: None,
                weight: 0,
                word_count: 0,
                reading_time: 0,
                toc: vec![],
                url: "/posts/new-post/".to_string(),
            },
            date: make_date(),
            excerpt: None,
            draft: false,
            tags: vec![],
            categories: vec![],
            redirect_from: vec!["/old-post/".to_string()],
        });

        let output_dir = tempfile::TempDir::new().unwrap();
        generate_redirects(&site, output_dir.path()).unwrap();

        let redirect_file = output_dir.path().join("old-post").join("index.html");
        assert!(redirect_file.exists());
        let content = std::fs::read_to_string(redirect_file).unwrap();
        assert!(content.contains("https://example.com/posts/new-post/"));
        assert!(content.contains("meta http-equiv=\"refresh\""));
    }

    #[test]
    fn test_page_redirects() {
        let mut site = minimal_site();
        site.pages.push(Page {
            content: Content {
                slug: "new-page".to_string(),
                title: "New Page".to_string(),
                html: String::new(),
                raw_content: String::new(),
                frontmatter: Frontmatter::default(),
                path: PathBuf::from("new-page/index.html"),
                template: None,
                weight: 0,
                word_count: 0,
                reading_time: 0,
                toc: vec![],
                url: "/new-page/".to_string(),
            },
            draft: false,
            redirect_from: vec!["/old-page/".to_string()],
        });

        let output_dir = tempfile::TempDir::new().unwrap();
        generate_redirects(&site, output_dir.path()).unwrap();

        let redirect_file = output_dir.path().join("old-page").join("index.html");
        assert!(redirect_file.exists());
    }

    #[test]
    fn test_unsafe_path_rejection() {
        assert!(!is_safe_redirect_path("../etc/passwd"));
        assert!(!is_safe_redirect_path("foo:bar"));
        assert!(!is_safe_redirect_path(""));
        assert!(!is_safe_redirect_path("nul"));
        assert!(!is_safe_redirect_path("con/test"));
    }

    #[test]
    fn test_safe_paths() {
        assert!(is_safe_redirect_path("old-post"));
        assert!(is_safe_redirect_path("blog/old-post"));
    }

    #[test]
    fn test_skip_existing_files() {
        let mut site = minimal_site();
        site.posts.push(Post {
            content: Content {
                slug: "post".to_string(),
                title: "Post".to_string(),
                html: String::new(),
                raw_content: String::new(),
                frontmatter: Frontmatter::default(),
                path: PathBuf::from("posts/post/index.html"),
                template: None,
                weight: 0,
                word_count: 0,
                reading_time: 0,
                toc: vec![],
                url: "/posts/post/".to_string(),
            },
            date: make_date(),
            excerpt: None,
            draft: false,
            tags: vec![],
            categories: vec![],
            redirect_from: vec!["/existing/".to_string()],
        });

        let output_dir = tempfile::TempDir::new().unwrap();
        std::fs::create_dir_all(output_dir.path().join("existing")).unwrap();
        std::fs::write(
            output_dir.path().join("existing").join("index.html"),
            "original",
        )
        .unwrap();

        generate_redirects(&site, output_dir.path()).unwrap();

        let content =
            std::fs::read_to_string(output_dir.path().join("existing").join("index.html")).unwrap();
        assert_eq!(content, "original");
    }
}
