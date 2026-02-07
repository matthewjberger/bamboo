use crate::error::{BambooError, Result};
use crate::types::{Frontmatter, TocEntry};
use chrono::NaiveDate;
use pulldown_cmark::{CodeBlockKind, Event, HeadingLevel, Options, Parser, Tag, TagEnd};
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::sync::LazyLock;
use syntect::highlighting::ThemeSet;
use syntect::html::highlighted_html_for_string;
use syntect::parsing::SyntaxSet;

static MARKDOWN_RENDERER: LazyLock<MarkdownRenderer> = LazyLock::new(MarkdownRenderer::new);

pub struct MarkdownRenderer {
    syntax_set: SyntaxSet,
    theme_set: ThemeSet,
}

impl Default for MarkdownRenderer {
    fn default() -> Self {
        Self::new()
    }
}

pub struct RenderedMarkdown {
    pub html: String,
    pub toc: Vec<TocEntry>,
}

impl MarkdownRenderer {
    pub fn new() -> Self {
        Self {
            syntax_set: SyntaxSet::load_defaults_newlines(),
            theme_set: ThemeSet::load_defaults(),
        }
    }

    pub fn render(&self, content: &str) -> RenderedMarkdown {
        let mut options = Options::empty();
        options.insert(Options::ENABLE_TABLES);
        options.insert(Options::ENABLE_FOOTNOTES);
        options.insert(Options::ENABLE_STRIKETHROUGH);
        options.insert(Options::ENABLE_TASKLISTS);
        options.insert(Options::ENABLE_HEADING_ATTRIBUTES);

        let parser = Parser::new_ext(content, options);
        let mut html_output = String::new();
        let mut in_code_block = false;
        let mut code_block_lang: Option<String> = None;
        let mut code_block_content = String::new();
        let mut toc = Vec::new();
        let mut in_heading = false;
        let mut heading_level: u32 = 0;
        let mut heading_plain_text = String::new();
        let mut heading_events: Vec<Event<'_>> = Vec::new();
        let mut used_heading_ids: HashSet<String> = HashSet::new();

        let theme = &self.theme_set.themes["base16-ocean.dark"];

        for event in parser {
            match event {
                Event::Start(Tag::Heading { level, .. }) => {
                    in_heading = true;
                    heading_level = heading_level_to_u32(level);
                    heading_plain_text.clear();
                    heading_events.clear();
                }
                Event::End(TagEnd::Heading(..)) => {
                    in_heading = false;
                    let base_id = slugify(&heading_plain_text);
                    let heading_id = if used_heading_ids.contains(&base_id) {
                        let mut suffix = 1;
                        loop {
                            let candidate = format!("{}-{}", base_id, suffix);
                            if !used_heading_ids.contains(&candidate) {
                                break candidate;
                            }
                            suffix += 1;
                        }
                    } else {
                        base_id
                    };
                    used_heading_ids.insert(heading_id.clone());

                    let mut heading_html = String::new();
                    pulldown_cmark::html::push_html(&mut heading_html, heading_events.drain(..));

                    toc.push(TocEntry {
                        level: heading_level,
                        id: heading_id.clone(),
                        title: heading_plain_text.clone(),
                    });
                    html_output.push_str(&format!(
                        "<h{level} id=\"{id}\"><a class=\"anchor\" href=\"#{id}\">#</a>{text}</h{level}>\n",
                        level = heading_level,
                        id = escape_html(&heading_id),
                        text = heading_html,
                    ));
                }
                Event::Start(Tag::CodeBlock(kind)) => {
                    in_code_block = true;
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
                    in_code_block = false;
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
                                    escape_html(lang),
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
                Event::Text(ref text) if in_heading => {
                    heading_plain_text.push_str(text);
                    heading_events.push(event);
                }
                Event::Code(ref code) if in_heading => {
                    heading_plain_text.push_str(code);
                    heading_events.push(event);
                }
                _ if in_heading => {
                    heading_events.push(event);
                }
                Event::Text(text) => {
                    if in_code_block {
                        code_block_content.push_str(&text);
                    } else {
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

        RenderedMarkdown {
            html: html_output,
            toc,
        }
    }
}

fn heading_level_to_u32(level: HeadingLevel) -> u32 {
    match level {
        HeadingLevel::H1 => 1,
        HeadingLevel::H2 => 2,
        HeadingLevel::H3 => 3,
        HeadingLevel::H4 => 4,
        HeadingLevel::H5 => 5,
        HeadingLevel::H6 => 6,
    }
}

pub fn slugify(text: &str) -> String {
    text.to_lowercase()
        .chars()
        .map(|character| {
            if character.is_alphanumeric() {
                character
            } else {
                '-'
            }
        })
        .collect::<String>()
        .split('-')
        .filter(|segment| !segment.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

fn escape_html(input: &str) -> String {
    crate::xml::escape(input)
}

pub fn parse_markdown(content: &str) -> RenderedMarkdown {
    MARKDOWN_RENDERER.render(content)
}

pub fn word_count(text: &str) -> usize {
    text.split_whitespace().count()
}

pub fn reading_time(word_count: usize) -> usize {
    let minutes = word_count / 200;
    if minutes == 0 && word_count > 0 {
        1
    } else {
        minutes
    }
}

pub fn extract_excerpt(content: &str, max_chars: usize) -> Option<String> {
    if content.trim().is_empty() {
        return None;
    }

    let first_paragraph = content
        .split("\n\n")
        .next()
        .map(|paragraph| paragraph.trim())
        .filter(|paragraph| !paragraph.is_empty())?;

    let text = strip_markdown_syntax(first_paragraph);
    let text = text.trim();

    if text.chars().count() <= max_chars {
        Some(text.to_string())
    } else {
        let truncated: String = text.chars().take(max_chars).collect();
        let last_space = truncated.rfind(' ').unwrap_or(truncated.len());
        Some(format!("{}...", &truncated[..last_space]))
    }
}

fn strip_markdown_syntax(text: &str) -> String {
    let mut output = String::with_capacity(text.len());
    let chars: Vec<char> = text.chars().collect();
    let length = chars.len();
    let mut position = 0;

    while position < length {
        if position + 1 < length && chars[position] == '!' && chars[position + 1] == '[' {
            position += 2;
            while position < length && chars[position] != ']' {
                output.push(chars[position]);
                position += 1;
            }
            if position < length {
                position += 1;
            }
            if position < length && chars[position] == '(' {
                position += 1;
                let mut paren_depth = 1;
                while position < length && paren_depth > 0 {
                    if chars[position] == '(' {
                        paren_depth += 1;
                    } else if chars[position] == ')' {
                        paren_depth -= 1;
                    }
                    position += 1;
                }
            }
            continue;
        }

        if chars[position] == '[' {
            position += 1;
            while position < length && chars[position] != ']' {
                output.push(chars[position]);
                position += 1;
            }
            if position < length {
                position += 1;
            }
            if position < length && chars[position] == '(' {
                position += 1;
                let mut paren_depth = 1;
                while position < length && paren_depth > 0 {
                    if chars[position] == '(' {
                        paren_depth += 1;
                    } else if chars[position] == ')' {
                        paren_depth -= 1;
                    }
                    position += 1;
                }
            }
            continue;
        }

        if chars[position] == '#' && (position == 0 || chars[position - 1] == '\n') {
            while position < length && chars[position] == '#' {
                position += 1;
            }
            if position < length && chars[position] == ' ' {
                position += 1;
            }
            continue;
        }

        if chars[position] == '*' || chars[position] == '_' {
            let prev_is_alnum = position > 0 && chars[position - 1].is_alphanumeric();
            let next_is_alnum = position + 1 < length && chars[position + 1].is_alphanumeric();
            if prev_is_alnum && next_is_alnum {
                output.push(chars[position]);
                position += 1;
            } else {
                position += 1;
            }
            continue;
        }

        if chars[position] == '`' {
            position += 1;
            continue;
        }

        output.push(chars[position]);
        position += 1;
    }

    output
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

    let mut search_offset = 0;
    loop {
        let end_index = find_closing_delimiter(&rest[search_offset..], "+++")
            .map(|position| search_offset + position)
            .ok_or_else(|| BambooError::InvalidFrontmatter {
                path: path.to_path_buf(),
            })?;

        let frontmatter_str = &rest[..end_index];
        match toml::from_str::<HashMap<String, Value>>(frontmatter_str) {
            Ok(raw) => {
                let body = &rest[end_index + 3..];
                return Ok((Frontmatter { raw }, body.trim().to_string()));
            }
            Err(error) => {
                let next_start = end_index + 3;
                if next_start >= rest.len() {
                    return Err(BambooError::TomlParse {
                        path: path.to_path_buf(),
                        message: error.to_string(),
                    });
                }
                search_offset = next_start;
            }
        }
    }
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
    let mut search_start = 0;
    while let Some(newline_position) = content[search_start..].find('\n') {
        let line_start = search_start;
        search_start += newline_position + 1;
        let line = &content[line_start..line_start + newline_position];
        if line.trim_end() == delimiter {
            return Some(line_start);
        }
    }
    if content[search_start..].trim_end() == delimiter {
        return Some(search_start);
    }
    None
}

pub fn parse_date_from_filename(filename: &str) -> Option<(String, String)> {
    let name = filename.strip_suffix(".md").unwrap_or(filename);

    let date_part = name.get(..10)?;
    let parts: Vec<&str> = date_part.split('-').collect();

    if parts.len() < 3 {
        return None;
    }

    if parts[0].len() != 4 || parts[1].len() != 2 || parts[2].len() != 2 {
        return None;
    }

    if NaiveDate::parse_from_str(date_part, "%Y-%m-%d").is_err() {
        return None;
    }

    let slug = if let Some(rest) = name.get(11..) {
        if name.as_bytes().get(10) == Some(&b'-') {
            rest.to_string()
        } else {
            name.to_string()
        }
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
        assert!(output.html.contains("<h1"));
        assert!(output.html.contains("Hello"));
        assert!(output.html.contains("<strong>bold</strong>"));
    }

    #[test]
    fn test_parse_markdown_with_code() {
        let input = "```rust\nfn main() {}\n```";
        let output = parse_markdown(input);
        assert!(output.html.contains("fn"));
        assert!(output.html.contains("main"));
    }

    #[test]
    fn test_heading_anchors() {
        let input = "## My Heading";
        let output = parse_markdown(input);
        assert!(output.html.contains("id=\"my-heading\""));
        assert!(output.html.contains("href=\"#my-heading\""));
    }

    #[test]
    fn test_toc_generation() {
        let input = "# Title\n## Section One\n### Subsection\n## Section Two";
        let output = parse_markdown(input);
        assert_eq!(output.toc.len(), 4);
        assert_eq!(output.toc[0].level, 1);
        assert_eq!(output.toc[0].title, "Title");
        assert_eq!(output.toc[1].level, 2);
        assert_eq!(output.toc[1].title, "Section One");
        assert_eq!(output.toc[2].level, 3);
        assert_eq!(output.toc[3].level, 2);
    }

    #[test]
    fn test_word_count_and_reading_time() {
        let text = "one two three four five";
        assert_eq!(word_count(text), 5);
        assert_eq!(reading_time(5), 1);
        assert_eq!(reading_time(400), 2);
        assert_eq!(reading_time(0), 0);
    }

    #[test]
    fn test_slugify() {
        assert_eq!(slugify("Hello World"), "hello-world");
        assert_eq!(slugify("  Multiple   Spaces  "), "multiple-spaces");
        assert_eq!(slugify("Special!@#Characters"), "special-characters");
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
    fn test_extract_excerpt_strips_links() {
        let content = "Check out [my site](https://example.com) for more.\n\nSecond paragraph.";
        let excerpt = extract_excerpt(content, 200);
        assert_eq!(excerpt, Some("Check out my site for more.".to_string()));
    }

    #[test]
    fn test_extract_excerpt_strips_images() {
        let content = "Here is ![alt text](https://example.com/img.png) inline.\n\nSecond.";
        let excerpt = extract_excerpt(content, 200);
        assert_eq!(excerpt, Some("Here is alt text inline.".to_string()));
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
