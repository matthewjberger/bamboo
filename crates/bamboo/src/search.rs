use std::path::Path;

use serde::Serialize;

use crate::error::Result;
use crate::types::Site;

#[derive(Serialize)]
pub struct SearchEntry {
    pub title: String,
    pub url: String,
    pub tags: Vec<String>,
    pub date: String,
    pub excerpt: String,
    pub content: String,
}

fn decode_numeric_entities(input: &str) -> String {
    let mut result = String::with_capacity(input.len());
    let mut remaining = input;

    while let Some(ampersand_position) = remaining.find("&#") {
        result.push_str(&remaining[..ampersand_position]);
        remaining = &remaining[ampersand_position..];

        if let Some(semicolon_position) = remaining[..remaining.len().min(12)].find(';') {
            let entity = &remaining[2..semicolon_position];
            let codepoint = if entity.starts_with('x') || entity.starts_with('X') {
                u32::from_str_radix(&entity[1..], 16).ok()
            } else {
                entity.parse::<u32>().ok()
            };

            if let Some(codepoint) = codepoint.and_then(char::from_u32) {
                result.push(codepoint);
                remaining = &remaining[semicolon_position + 1..];
                continue;
            }
        }

        result.push_str("&#");
        remaining = &remaining[2..];
    }

    result.push_str(remaining);
    result
}

pub fn strip_html_tags(html: &str) -> String {
    let mut output = String::with_capacity(html.len());
    let mut inside_tag = false;
    let mut skip_content = false;
    let mut tag_name_buffer = String::new();
    let mut collecting_tag_name = false;
    let mut is_closing_tag = false;

    for character in html.chars() {
        match character {
            '<' => {
                inside_tag = true;
                collecting_tag_name = true;
                is_closing_tag = false;
                tag_name_buffer.clear();
            }
            '>' => {
                inside_tag = false;
                collecting_tag_name = false;
                let tag_lower = tag_name_buffer.to_lowercase();
                if !is_closing_tag && (tag_lower == "script" || tag_lower == "style") {
                    skip_content = true;
                } else if is_closing_tag && (tag_lower == "script" || tag_lower == "style") {
                    skip_content = false;
                }
            }
            '/' if inside_tag && tag_name_buffer.is_empty() => {
                is_closing_tag = true;
            }
            ' ' | '\t' | '\n' | '\r' if inside_tag => {
                collecting_tag_name = false;
            }
            _ if inside_tag && collecting_tag_name => {
                tag_name_buffer.push(character);
            }
            _ if !inside_tag && !skip_content => output.push(character),
            _ => {}
        }
    }

    let named_decoded = output
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
        .replace("&amp;", "&");

    let decoded = decode_numeric_entities(&named_decoded);

    let mut result = String::with_capacity(decoded.len());
    let mut previous_was_whitespace = false;

    for character in decoded.chars() {
        if character.is_whitespace() {
            if !previous_was_whitespace {
                result.push(' ');
            }
            previous_was_whitespace = true;
        } else {
            result.push(character);
            previous_was_whitespace = false;
        }
    }

    result.trim().to_string()
}

const MAX_SEARCH_CONTENT_CHARS: usize = 5000;

fn truncate_content(content: &str, max_chars: usize) -> String {
    if content.chars().count() <= max_chars {
        return content.to_string();
    }
    content.chars().take(max_chars).collect()
}

pub fn generate_search_index(site: &Site, output_dir: &Path) -> Result<()> {
    let mut entries: Vec<SearchEntry> = Vec::new();

    if let Some(ref home) = site.home {
        entries.push(SearchEntry {
            title: home.title.clone(),
            url: home.url.clone(),
            tags: Vec::new(),
            date: String::new(),
            excerpt: String::new(),
            content: truncate_content(&strip_html_tags(&home.content), MAX_SEARCH_CONTENT_CHARS),
        });
    }

    for post in &site.posts {
        entries.push(SearchEntry {
            title: post.title.clone(),
            url: post.url.clone(),
            tags: post.tags.clone(),
            date: post.date.format("%Y-%m-%d").to_string(),
            excerpt: post.excerpt.clone().unwrap_or_default(),
            content: truncate_content(&strip_html_tags(&post.content), MAX_SEARCH_CONTENT_CHARS),
        });
    }

    for page in &site.pages {
        if page.slug == "404" {
            continue;
        }
        entries.push(SearchEntry {
            title: page.title.clone(),
            url: page.url.clone(),
            tags: Vec::new(),
            date: String::new(),
            excerpt: String::new(),
            content: truncate_content(&strip_html_tags(&page.content), MAX_SEARCH_CONTENT_CHARS),
        });
    }

    for collection in site.collections.values() {
        for item in &collection.items {
            entries.push(SearchEntry {
                title: item.title.clone(),
                url: item.url.clone(),
                tags: Vec::new(),
                date: String::new(),
                excerpt: String::new(),
                content: truncate_content(
                    &strip_html_tags(&item.content),
                    MAX_SEARCH_CONTENT_CHARS,
                ),
            });
        }
    }

    let json = serde_json::to_string_pretty(&entries).map_err(std::io::Error::other)?;
    std::fs::write(output_dir.join("search-index.json"), json)?;

    Ok(())
}
