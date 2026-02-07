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

fn minify_css(source: &str) -> String {
    let mut result = String::with_capacity(source.len());
    let chars: Vec<char> = source.chars().collect();
    let length = chars.len();
    let mut position = 0;

    while position < length {
        if position + 1 < length && chars[position] == '/' && chars[position + 1] == '*' {
            position += 2;
            let mut found_end = false;
            while position + 1 < length {
                if chars[position] == '*' && chars[position + 1] == '/' {
                    position += 2;
                    found_end = true;
                    break;
                }
                position += 1;
            }
            if !found_end {
                position = length;
            }
            continue;
        }

        if chars[position] == '"' || chars[position] == '\'' {
            let quote = chars[position];
            result.push(quote);
            position += 1;
            while position < length {
                if chars[position] == '\\' && position + 1 < length {
                    result.push(chars[position]);
                    result.push(chars[position + 1]);
                    position += 2;
                    continue;
                }
                if chars[position] == quote {
                    result.push(quote);
                    position += 1;
                    break;
                }
                result.push(chars[position]);
                position += 1;
            }
            continue;
        }

        if chars[position].is_ascii_whitespace() {
            while position < length && chars[position].is_ascii_whitespace() {
                position += 1;
            }
            if !result.is_empty() {
                let last_char = result.chars().last().unwrap_or(' ');
                let next_char = if position < length {
                    chars[position]
                } else {
                    ' '
                };
                let structural = matches!(last_char, '{' | '}' | ':' | ';' | ',');
                let next_structural = matches!(next_char, '{' | '}' | ';' | ',');
                if !structural && !next_structural {
                    result.push(' ');
                }
            }
            continue;
        }

        if chars[position] == ';' && position + 1 < length {
            let mut lookahead = position + 1;
            while lookahead < length && chars[lookahead].is_ascii_whitespace() {
                lookahead += 1;
            }
            if lookahead < length && chars[lookahead] == '}' {
                position += 1;
                continue;
            }
        }

        result.push(chars[position]);
        position += 1;
    }

    result.trim().to_string()
}

fn is_regex_predecessor(character: char) -> bool {
    matches!(
        character,
        '=' | '('
            | '['
            | '!'
            | '&'
            | '|'
            | '?'
            | '{'
            | '}'
            | ';'
            | ','
            | '~'
            | '^'
            | ':'
            | '<'
            | '>'
            | '+'
            | '-'
            | '*'
            | '%'
            | '\n'
            | '\r'
    )
}

const REGEX_KEYWORDS: &[&str] = &[
    "return",
    "typeof",
    "instanceof",
    "in",
    "delete",
    "void",
    "throw",
    "new",
    "case",
    "yield",
    "await",
];

fn ends_with_regex_keyword(result: &str) -> bool {
    let trimmed = result.trim_end();
    for keyword in REGEX_KEYWORDS {
        if let Some(before) = trimmed.strip_suffix(keyword)
            && (before.is_empty()
                || !before.ends_with(|character: char| {
                    character.is_ascii_alphanumeric() || character == '_' || character == '$'
                }))
        {
            return true;
        }
    }
    false
}

fn last_significant_char(result: &str) -> Option<char> {
    result
        .chars()
        .rev()
        .find(|character| !character.is_ascii_whitespace())
}

fn minify_js(source: &str) -> String {
    let mut result = String::with_capacity(source.len());
    let chars: Vec<char> = source.chars().collect();
    let length = chars.len();
    let mut position = 0;

    while position < length {
        if chars[position] == '"' || chars[position] == '\'' {
            let quote = chars[position];
            result.push(quote);
            position += 1;
            while position < length {
                if chars[position] == '\\' && position + 1 < length {
                    result.push(chars[position]);
                    result.push(chars[position + 1]);
                    position += 2;
                    continue;
                }
                if chars[position] == quote {
                    result.push(quote);
                    position += 1;
                    break;
                }
                result.push(chars[position]);
                position += 1;
            }
            continue;
        }

        if chars[position] == '`' {
            result.push('`');
            position += 1;
            while position < length {
                if chars[position] == '\\' && position + 1 < length {
                    result.push(chars[position]);
                    result.push(chars[position + 1]);
                    position += 2;
                    continue;
                }
                if chars[position] == '$' && position + 1 < length && chars[position + 1] == '{' {
                    result.push('$');
                    result.push('{');
                    position += 2;
                    let mut brace_depth = 1;
                    while position < length && brace_depth > 0 {
                        if chars[position] == '\\' && position + 1 < length {
                            result.push(chars[position]);
                            result.push(chars[position + 1]);
                            position += 2;
                            continue;
                        }
                        if chars[position] == '\'' || chars[position] == '"' {
                            let inner_quote = chars[position];
                            result.push(inner_quote);
                            position += 1;
                            while position < length {
                                if chars[position] == '\\' && position + 1 < length {
                                    result.push(chars[position]);
                                    result.push(chars[position + 1]);
                                    position += 2;
                                    continue;
                                }
                                if chars[position] == inner_quote {
                                    result.push(inner_quote);
                                    position += 1;
                                    break;
                                }
                                result.push(chars[position]);
                                position += 1;
                            }
                            continue;
                        }
                        if chars[position] == '{' {
                            brace_depth += 1;
                        } else if chars[position] == '}' {
                            brace_depth -= 1;
                            if brace_depth == 0 {
                                result.push('}');
                                position += 1;
                                break;
                            }
                        }
                        result.push(chars[position]);
                        position += 1;
                    }
                    continue;
                }
                if chars[position] == '`' {
                    result.push('`');
                    position += 1;
                    break;
                }
                result.push(chars[position]);
                position += 1;
            }
            continue;
        }

        if chars[position] == '/'
            && position + 1 < length
            && chars[position + 1] != '/'
            && chars[position + 1] != '*'
        {
            let predecessor = last_significant_char(&result);
            if predecessor.is_none()
                || predecessor.is_some_and(is_regex_predecessor)
                || ends_with_regex_keyword(&result)
            {
                result.push('/');
                position += 1;
                while position < length {
                    if chars[position] == '\\' && position + 1 < length {
                        result.push(chars[position]);
                        result.push(chars[position + 1]);
                        position += 2;
                        continue;
                    }
                    if chars[position] == '/' {
                        result.push('/');
                        position += 1;
                        while position < length && chars[position].is_ascii_alphanumeric() {
                            result.push(chars[position]);
                            position += 1;
                        }
                        break;
                    }
                    if chars[position] == '[' {
                        result.push('[');
                        position += 1;
                        while position < length && chars[position] != ']' {
                            if chars[position] == '\\' && position + 1 < length {
                                result.push(chars[position]);
                                result.push(chars[position + 1]);
                                position += 2;
                                continue;
                            }
                            result.push(chars[position]);
                            position += 1;
                        }
                        if position < length {
                            result.push(']');
                            position += 1;
                        }
                        continue;
                    }
                    result.push(chars[position]);
                    position += 1;
                }
                continue;
            }
        }

        if position + 1 < length && chars[position] == '/' && chars[position + 1] == '/' {
            position += 2;
            while position < length && chars[position] != '\n' {
                position += 1;
            }
            if position < length {
                result.push('\n');
                position += 1;
            }
            continue;
        }

        if position + 1 < length && chars[position] == '/' && chars[position + 1] == '*' {
            position += 2;
            let mut found_end = false;
            while position + 1 < length {
                if chars[position] == '*' && chars[position + 1] == '/' {
                    position += 2;
                    found_end = true;
                    break;
                }
                position += 1;
            }
            if !found_end {
                position = length;
            }
            continue;
        }

        if chars[position].is_ascii_whitespace() {
            let mut contains_newline = false;
            while position < length && chars[position].is_ascii_whitespace() {
                if chars[position] == '\n' || chars[position] == '\r' {
                    contains_newline = true;
                }
                position += 1;
            }
            if !result.is_empty() {
                let last = result.chars().last().unwrap_or(' ');
                if last != ' ' && last != '\n' {
                    if contains_newline {
                        result.push('\n');
                    } else {
                        result.push(' ');
                    }
                }
            }
            continue;
        }

        result.push(chars[position]);
        position += 1;
    }

    result.trim().to_string()
}

fn minify_css_files(output_dir: &Path) -> Result<()> {
    let css_files = collect_files_with_extension(output_dir, "css")?;
    for file_path in css_files {
        let content = fs::read_to_string(&file_path)?;
        let minified = minify_css(&content);
        fs::write(&file_path, minified)?;
    }
    Ok(())
}

fn minify_js_files(output_dir: &Path) -> Result<()> {
    let js_files = collect_files_with_extension(output_dir, "js")?;
    for file_path in js_files {
        let content = fs::read_to_string(&file_path)?;
        let minified = minify_js(&content);
        fs::write(&file_path, minified)?;
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
