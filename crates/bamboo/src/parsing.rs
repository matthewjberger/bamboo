use crate::error::{BambooError, Result};
use crate::types::Frontmatter;
use pulldown_cmark::{CodeBlockKind, Event, Options, Parser, Tag, TagEnd};
use serde_json::Value;
use std::collections::HashMap;
use std::path::Path;
use syntect::highlighting::ThemeSet;
use syntect::html::highlighted_html_for_string;
use syntect::parsing::SyntaxSet;

pub struct MarkdownRenderer {
    syntax_set: SyntaxSet,
    theme_set: ThemeSet,
}

impl Default for MarkdownRenderer {
    fn default() -> Self {
        Self::new()
    }
}

impl MarkdownRenderer {
    pub fn new() -> Self {
        Self {
            syntax_set: SyntaxSet::load_defaults_newlines(),
            theme_set: ThemeSet::load_defaults(),
        }
    }

    pub fn render(&self, content: &str) -> String {
        let mut options = Options::empty();
        options.insert(Options::ENABLE_TABLES);
        options.insert(Options::ENABLE_FOOTNOTES);
        options.insert(Options::ENABLE_STRIKETHROUGH);
        options.insert(Options::ENABLE_TASKLISTS);
        options.insert(Options::ENABLE_HEADING_ATTRIBUTES);

        let parser = Parser::new_ext(content, options);
        let mut html_output = String::new();
        let mut code_block_lang: Option<String> = None;
        let mut code_block_content = String::new();

        let theme = &self.theme_set.themes["base16-ocean.dark"];

        for event in parser {
            match event {
                Event::Start(Tag::CodeBlock(kind)) => {
                    code_block_lang = match kind {
                        CodeBlockKind::Fenced(lang) => {
                            let lang_str = lang.as_ref();
                            if lang_str.is_empty() {
                                None
                            } else {
                                Some(lang_str.to_string())
                            }
                        }
                        CodeBlockKind::Indented => None,
                    };
                    code_block_content.clear();
                }
                Event::End(TagEnd::CodeBlock) => {
                    let highlighted = if let Some(ref lang) = code_block_lang {
                        self.syntax_set
                            .find_syntax_by_token(lang)
                            .map(|syntax| {
                                highlighted_html_for_string(
                                    &code_block_content,
                                    &self.syntax_set,
                                    syntax,
                                    theme,
                                )
                                .unwrap_or_else(|_| escape_html(&code_block_content))
                            })
                            .unwrap_or_else(|| {
                                format!(
                                    "<pre><code class=\"language-{}\">{}</code></pre>",
                                    lang,
                                    escape_html(&code_block_content)
                                )
                            })
                    } else {
                        format!(
                            "<pre><code>{}</code></pre>",
                            escape_html(&code_block_content)
                        )
                    };
                    html_output.push_str(&highlighted);
                    code_block_lang = None;
                }
                Event::Text(text) => {
                    if code_block_lang.is_some()
                        || !code_block_content.is_empty() && code_block_lang.is_none()
                    {
                        code_block_content.push_str(&text);
                    } else if code_block_lang.is_none() && code_block_content.is_empty() {
                        let mut temp = String::new();
                        pulldown_cmark::html::push_html(
                            &mut temp,
                            std::iter::once(Event::Text(text)),
                        );
                        html_output.push_str(&temp);
                    }
                }
                Event::Code(code) => {
                    html_output.push_str("<code>");
                    html_output.push_str(&escape_html(&code));
                    html_output.push_str("</code>");
                }
                other => {
                    let mut temp = String::new();
                    pulldown_cmark::html::push_html(&mut temp, std::iter::once(other));
                    html_output.push_str(&temp);
                }
            }
        }

        html_output
    }
}

fn escape_html(input: &str) -> String {
    input
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

pub fn parse_markdown(content: &str) -> String {
    let renderer = MarkdownRenderer::new();
    renderer.render(content)
}

pub fn extract_excerpt(content: &str, max_chars: usize) -> Option<String> {
    if content.trim().is_empty() {
        return None;
    }

    let first_paragraph = content
        .split("\n\n")
        .next()
        .map(|p| p.trim())
        .filter(|p| !p.is_empty())?;

    let text: String = first_paragraph
        .chars()
        .filter(|c| !['#', '*', '_', '`', '[', ']', '(', ')'].contains(c))
        .collect();

    let text = text.trim();

    if text.len() <= max_chars {
        Some(text.to_string())
    } else {
        let truncated: String = text.chars().take(max_chars).collect();
        let last_space = truncated.rfind(' ').unwrap_or(max_chars);
        Some(format!("{}...", &truncated[..last_space]))
    }
}

pub fn extract_frontmatter(content: &str, path: &Path) -> Result<(Frontmatter, String)> {
    let content = content.replace("\r\n", "\n");
    let content = content.trim_start();

    if content.starts_with("+++") {
        parse_toml_frontmatter(content, path)
    } else if content.starts_with("---") {
        parse_yaml_frontmatter(content, path)
    } else {
        Ok((Frontmatter::default(), content.to_string()))
    }
}

fn parse_toml_frontmatter(content: &str, path: &Path) -> Result<(Frontmatter, String)> {
    let rest = &content[3..];

    let end_index =
        find_closing_delimiter(rest, "+++").ok_or_else(|| BambooError::InvalidFrontmatter {
            path: path.to_path_buf(),
        })?;

    let frontmatter_str = &rest[..end_index];
    let body = &rest[end_index + 3..];

    let raw: HashMap<String, Value> =
        toml::from_str(frontmatter_str).map_err(|error| BambooError::TomlParse {
            path: path.to_path_buf(),
            message: error.to_string(),
        })?;

    Ok((Frontmatter { raw }, body.trim().to_string()))
}

fn parse_yaml_frontmatter(content: &str, path: &Path) -> Result<(Frontmatter, String)> {
    let rest = &content[3..];

    let end_index =
        find_closing_delimiter(rest, "---").ok_or_else(|| BambooError::InvalidFrontmatter {
            path: path.to_path_buf(),
        })?;

    let frontmatter_str = &rest[..end_index];
    let body = &rest[end_index + 3..];

    let raw: HashMap<String, Value> =
        serde_yaml::from_str(frontmatter_str).map_err(|error| BambooError::YamlParse {
            path: path.to_path_buf(),
            message: error.to_string(),
        })?;

    Ok((Frontmatter { raw }, body.trim().to_string()))
}

fn find_closing_delimiter(content: &str, delimiter: &str) -> Option<usize> {
    let mut position = 0;

    for line in content.lines() {
        if line.trim() == delimiter {
            return Some(position);
        }
        position += line.len() + 1;
    }

    None
}

pub fn parse_date_from_filename(filename: &str) -> Option<(String, String)> {
    let name = filename.strip_suffix(".md").unwrap_or(filename);

    if name.len() < 11 {
        return None;
    }

    let date_part = &name[..10];
    let parts: Vec<&str> = date_part.split('-').collect();

    if parts.len() < 3 {
        return None;
    }

    if parts[0].len() != 4 || parts[1].len() != 2 || parts[2].len() != 2 {
        return None;
    }

    if parts[0].parse::<u32>().is_err()
        || parts[1].parse::<u32>().is_err()
        || parts[2].parse::<u32>().is_err()
    {
        return None;
    }

    let slug = if name.len() > 11 && name.chars().nth(10) == Some('-') {
        name[11..].to_string()
    } else {
        name.to_string()
    };

    Some((date_part.to_string(), slug))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_parse_markdown() {
        let input = "# Hello\n\nThis is **bold**.";
        let output = parse_markdown(input);
        assert!(output.contains("<h1>"));
        assert!(output.contains("Hello"));
        assert!(output.contains("<strong>bold</strong>"));
    }

    #[test]
    fn test_parse_markdown_with_code() {
        let input = "```rust\nfn main() {}\n```";
        let output = parse_markdown(input);
        assert!(output.contains("fn"));
        assert!(output.contains("main"));
    }

    #[test]
    fn test_parse_date_from_filename() {
        assert_eq!(
            parse_date_from_filename("2024-01-15-hello-world.md"),
            Some(("2024-01-15".to_string(), "hello-world".to_string()))
        );
        assert_eq!(parse_date_from_filename("about.md"), None);
    }

    #[test]
    fn test_extract_excerpt() {
        let content = "This is the first paragraph.\n\nThis is the second.";
        let excerpt = extract_excerpt(content, 100);
        assert_eq!(excerpt, Some("This is the first paragraph.".to_string()));
    }

    #[test]
    fn test_extract_excerpt_truncation() {
        let content = "This is a very long paragraph that should be truncated at some point.";
        let excerpt = extract_excerpt(content, 30);
        assert!(excerpt.unwrap().ends_with("..."));
    }

    #[test]
    fn test_yaml_frontmatter_with_dashes_in_content() {
        let content = "---\ntitle: Test\n---\n\nContent with --- dashes";
        let path = PathBuf::from("test.md");
        let (fm, body) = extract_frontmatter(content, &path).unwrap();
        assert_eq!(fm.get_string("title"), Some("Test".to_string()));
        assert!(body.contains("---"));
    }

    #[test]
    fn test_toml_frontmatter() {
        let content = "+++\ntitle = \"Test\"\n+++\n\nBody content";
        let path = PathBuf::from("test.md");
        let (fm, body) = extract_frontmatter(content, &path).unwrap();
        assert_eq!(fm.get_string("title"), Some("Test".to_string()));
        assert_eq!(body, "Body content");
    }
}
