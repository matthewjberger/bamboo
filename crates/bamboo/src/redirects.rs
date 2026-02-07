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

pub fn generate_redirects(site: &Site, output_dir: &Path) -> Result<()> {
    let base_url = site.config.base_url.trim_end_matches('/');

    for post in &site.posts {
        for redirect_path in &post.redirect_from {
            let clean_path = redirect_path.trim_matches('/');
            if !is_safe_redirect_path(clean_path) {
                continue;
            }
            let redirect_dir = output_dir.join(clean_path);
            let redirect_file = redirect_dir.join("index.html");
            if redirect_file.exists() {
                continue;
            }
            fs::create_dir_all(&redirect_dir)?;

            let target_url = format!("{}/posts/{}/", base_url, post.slug);
            let redirect_html = format!(
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
                url = crate::xml::escape(&target_url)
            );

            fs::write(redirect_dir.join("index.html"), redirect_html)?;
        }
    }

    for page in &site.pages {
        for redirect_path in &page.redirect_from {
            let clean_path = redirect_path.trim_matches('/');
            if !is_safe_redirect_path(clean_path) {
                continue;
            }
            let redirect_dir = output_dir.join(clean_path);
            let redirect_file = redirect_dir.join("index.html");
            if redirect_file.exists() {
                continue;
            }
            fs::create_dir_all(&redirect_dir)?;

            let target_url = format!("{}/{}/", base_url, page.slug);
            let redirect_html = format!(
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
                url = crate::xml::escape(&target_url)
            );

            fs::write(redirect_dir.join("index.html"), redirect_html)?;
        }
    }

    Ok(())
}
