use std::collections::HashMap;

use tera::Tera;

use crate::error::{BambooError, Result};
use crate::parsing::{MarkdownRenderer, parse_markdown};

const BUILTIN_YOUTUBE: &str = include_str!("../themes/default/templates/shortcodes/youtube.html");
const BUILTIN_FIGURE: &str = include_str!("../themes/default/templates/shortcodes/figure.html");
const BUILTIN_NOTE: &str = include_str!("../themes/default/templates/shortcodes/note.html");
const BUILTIN_DETAILS: &str = include_str!("../themes/default/templates/shortcodes/details.html");
const BUILTIN_GIST: &str = include_str!("../themes/default/templates/shortcodes/gist.html");

pub struct ShortcodeProcessor {
    tera: Tera,
    ref_registry: HashMap<String, String>,
}

impl ShortcodeProcessor {
    pub fn new(shortcode_dirs: &[std::path::PathBuf]) -> Result<Self> {
        let mut tera = Tera::default();

        tera.add_raw_template("shortcodes/youtube.html", BUILTIN_YOUTUBE)
            .map_err(BambooError::Template)?;
        tera.add_raw_template("shortcodes/figure.html", BUILTIN_FIGURE)
            .map_err(BambooError::Template)?;
        tera.add_raw_template("shortcodes/note.html", BUILTIN_NOTE)
            .map_err(BambooError::Template)?;
        tera.add_raw_template("shortcodes/details.html", BUILTIN_DETAILS)
            .map_err(BambooError::Template)?;
        tera.add_raw_template("shortcodes/gist.html", BUILTIN_GIST)
            .map_err(BambooError::Template)?;

        for directory in shortcode_dirs {
            if directory.is_dir()
                && let Ok(entries) = std::fs::read_dir(directory)
            {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.extension().and_then(|extension| extension.to_str()) == Some("html")
                        && let Some(file_name) = path.file_name().and_then(|name| name.to_str())
                    {
                        let template_name = format!("shortcodes/{}", file_name);
                        let template_content = std::fs::read_to_string(&path)?;
                        tera.add_raw_template(&template_name, &template_content)
                            .map_err(BambooError::Template)?;
                    }
                }
            }
        }

        Ok(Self {
            tera,
            ref_registry: HashMap::new(),
        })
    }

    pub fn set_ref_registry(&mut self, registry: HashMap<String, String>) {
        self.ref_registry = registry;
    }

    pub fn process(&self, content: &str, renderer: Option<&MarkdownRenderer>) -> Result<String> {
        let mut output = String::with_capacity(content.len());
        let mut remaining = content;

        while !remaining.is_empty() {
            let next_fence = find_next_code_fence(remaining);
            let next_inline = remaining.find("{{<");
            let next_block = remaining.find("{{%");
            let next_shortcode = match (next_inline, next_block) {
                (Some(a), Some(b)) => Some(a.min(b)),
                (Some(a), None) => Some(a),
                (None, Some(b)) => Some(b),
                (None, None) => None,
            };

            if let Some(fence_position) = next_fence
                && (next_shortcode.is_none() || fence_position < next_shortcode.unwrap())
            {
                let fence_str = &remaining[fence_position..];
                let fence_marker = if fence_str.starts_with("```") {
                    "```"
                } else {
                    "~~~"
                };
                let after_fence_start = &remaining[fence_position + fence_marker.len()..];
                if let Some(end_of_opening_line) = after_fence_start.find('\n') {
                    let after_opening_line = &after_fence_start[end_of_opening_line + 1..];
                    if let Some(closing_fence) =
                        find_closing_code_fence(after_opening_line, fence_marker)
                    {
                        let end_position = fence_position
                            + fence_marker.len()
                            + end_of_opening_line
                            + 1
                            + closing_fence
                            + fence_marker.len();
                        let skip_to = remaining[end_position..]
                            .find('\n')
                            .map(|newline| end_position + newline + 1)
                            .unwrap_or(remaining.len());
                        output.push_str(&remaining[..skip_to]);
                        remaining = &remaining[skip_to..];
                        continue;
                    }
                }
                output.push_str(&remaining[..fence_position + fence_marker.len()]);
                remaining = &remaining[fence_position + fence_marker.len()..];
                continue;
            }

            if let Some(inline_start) = next_inline {
                if let Some(block_start) = next_block {
                    if block_start < inline_start {
                        output.push_str(&remaining[..block_start]);
                        remaining = &remaining[block_start..];
                        remaining =
                            self.process_block_shortcode(remaining, &mut output, renderer)?;
                    } else {
                        output.push_str(&remaining[..inline_start]);
                        remaining = &remaining[inline_start..];
                        remaining = self.process_inline_shortcode(remaining, &mut output)?;
                    }
                } else {
                    output.push_str(&remaining[..inline_start]);
                    remaining = &remaining[inline_start..];
                    remaining = self.process_inline_shortcode(remaining, &mut output)?;
                }
            } else if let Some(block_start) = next_block {
                output.push_str(&remaining[..block_start]);
                remaining = &remaining[block_start..];
                remaining = self.process_block_shortcode(remaining, &mut output, renderer)?;
            } else {
                output.push_str(remaining);
                break;
            }
        }

        Ok(output)
    }

    fn process_inline_shortcode<'a>(&self, input: &'a str, output: &mut String) -> Result<&'a str> {
        let after_open = &input[3..];

        let close_position = after_open
            .find(">}}")
            .ok_or_else(|| BambooError::ShortcodeParse {
                message: "unclosed inline shortcode, expected >}}".to_string(),
            })?;

        let inner = after_open[..close_position].trim();
        let (name, arguments) = parse_shortcode_args(inner)?;

        if name == "ref" {
            let reference = arguments
                .get("_positional")
                .or_else(|| arguments.get("path"))
                .ok_or_else(|| BambooError::ShortcodeParse {
                    message: "ref shortcode requires a path argument".to_string(),
                })?;

            let url = self.ref_registry.get(reference.as_str()).ok_or_else(|| {
                BambooError::BrokenReference {
                    reference: reference.clone(),
                }
            })?;

            output.push_str(url);
            return Ok(&after_open[close_position + 3..]);
        }

        let template_name = format!("shortcodes/{}.html", name);
        let mut context = tera::Context::new();
        for (key, value) in &arguments {
            context.insert(key.as_str(), value);
        }

        let rendered = self
            .tera
            .render(&template_name, &context)
            .map_err(|error| BambooError::ShortcodeRender {
                name: name.clone(),
                message: error.to_string(),
            })?;

        output.push_str(&rendered);

        Ok(&after_open[close_position + 3..])
    }

    fn process_block_shortcode<'a>(
        &self,
        input: &'a str,
        output: &mut String,
        renderer: Option<&MarkdownRenderer>,
    ) -> Result<&'a str> {
        let after_open = &input[3..];

        let close_position = after_open
            .find("%}}")
            .ok_or_else(|| BambooError::ShortcodeParse {
                message: "unclosed block shortcode opening tag, expected %}}".to_string(),
            })?;

        let inner = after_open[..close_position].trim();
        let (name, arguments) = parse_shortcode_args(inner)?;

        let after_opening_tag = &after_open[close_position + 3..];

        let opening_with_args = format!("{{{{% {} ", name);
        let opening_without_args = format!("{{{{% {} %}}}}", name);
        let closing_tag = format!("{{{{% /{} %}}}}", name);
        let closing_position = find_matching_closing_tag(
            after_opening_tag,
            &opening_with_args,
            &opening_without_args,
            &closing_tag,
        )
        .ok_or_else(|| BambooError::ShortcodeParse {
            message: format!("missing closing tag for block shortcode '{}'", name),
        })?;

        let body_raw = &after_opening_tag[..closing_position];
        let body_processed = self.process(body_raw.trim(), renderer)?;
        let body_rendered = if let Some(renderer) = renderer {
            renderer.render(&body_processed)
        } else {
            parse_markdown(&body_processed)
        };

        let template_name = format!("shortcodes/{}.html", name);
        let mut context = tera::Context::new();
        for (key, value) in &arguments {
            context.insert(key.as_str(), value);
        }
        context.insert("body", &body_rendered.html);

        let rendered = self
            .tera
            .render(&template_name, &context)
            .map_err(|error| BambooError::ShortcodeRender {
                name: name.clone(),
                message: error.to_string(),
            })?;

        output.push_str(&rendered);

        Ok(&after_opening_tag[closing_position + closing_tag.len()..])
    }
}

fn parse_shortcode_args(input: &str) -> Result<(String, HashMap<String, String>)> {
    let mut arguments = HashMap::new();
    let mut name = String::new();
    let mut chars = input.chars().peekable();

    skip_whitespace(&mut chars);

    while let Some(&character) = chars.peek() {
        if character.is_alphanumeric() || character == '_' || character == '-' {
            name.push(character);
            chars.next();
        } else {
            break;
        }
    }

    if name.is_empty() {
        return Err(BambooError::ShortcodeParse {
            message: "shortcode name is empty".to_string(),
        });
    }

    loop {
        skip_whitespace(&mut chars);

        if chars.peek().is_none() {
            break;
        }

        if chars.peek() == Some(&'"') {
            chars.next();
            let mut value = String::new();
            let mut found_closing_quote = false;
            while let Some(&character) = chars.peek() {
                chars.next();
                if character == '\\'
                    && let Some(&escaped) = chars.peek()
                {
                    chars.next();
                    value.push(escaped);
                    continue;
                }
                if character == '"' {
                    found_closing_quote = true;
                    break;
                }
                value.push(character);
            }
            if !found_closing_quote {
                return Err(BambooError::ShortcodeParse {
                    message: format!("unclosed positional string value in shortcode '{}'", name),
                });
            }
            arguments.insert("_positional".to_string(), value);
            continue;
        }

        let mut key = String::new();
        while let Some(&character) = chars.peek() {
            if character.is_alphanumeric() || character == '_' || character == '-' {
                key.push(character);
                chars.next();
            } else {
                break;
            }
        }

        if key.is_empty() {
            return Err(BambooError::ShortcodeParse {
                message: format!("expected argument key in shortcode '{}'", name),
            });
        }

        skip_whitespace(&mut chars);

        match chars.peek() {
            Some(&'=') => {
                chars.next();
            }
            _ => {
                return Err(BambooError::ShortcodeParse {
                    message: format!("expected '=' after key '{}' in shortcode '{}'", key, name),
                });
            }
        }

        skip_whitespace(&mut chars);

        match chars.peek() {
            Some(&'"') => {
                chars.next();
            }
            _ => {
                return Err(BambooError::ShortcodeParse {
                    message: format!(
                        "expected '\"' to begin value for key '{}' in shortcode '{}'",
                        key, name
                    ),
                });
            }
        }

        let mut value = String::new();
        let mut found_closing_quote = false;
        while let Some(&character) = chars.peek() {
            chars.next();
            if character == '\\'
                && let Some(&escaped) = chars.peek()
            {
                chars.next();
                value.push(escaped);
                continue;
            }
            if character == '"' {
                found_closing_quote = true;
                break;
            }
            value.push(character);
        }

        if !found_closing_quote {
            return Err(BambooError::ShortcodeParse {
                message: format!(
                    "unclosed string value for key '{}' in shortcode '{}'",
                    key, name
                ),
            });
        }

        arguments.insert(key, value);
    }

    Ok((name, arguments))
}

fn find_matching_closing_tag(
    content: &str,
    opening_with_args: &str,
    opening_without_args: &str,
    closing_tag: &str,
) -> Option<usize> {
    let mut depth = 0usize;
    let mut search_from = 0;

    while search_from < content.len() {
        let next_open_with_args = content[search_from..]
            .find(opening_with_args)
            .map(|position| search_from + position);
        let next_open_without_args = content[search_from..]
            .find(opening_without_args)
            .map(|position| search_from + position);
        let next_open = match (next_open_with_args, next_open_without_args) {
            (Some(a), Some(b)) => Some(a.min(b)),
            (Some(a), None) => Some(a),
            (None, Some(b)) => Some(b),
            (None, None) => None,
        };
        let next_close = content[search_from..]
            .find(closing_tag)
            .map(|position| search_from + position);

        match (next_open, next_close) {
            (Some(open_position), Some(close_position)) if open_position < close_position => {
                depth += 1;
                let advance = if next_open_with_args == Some(open_position) {
                    opening_with_args.len()
                } else {
                    opening_without_args.len()
                };
                search_from = open_position + advance;
            }
            (_, Some(close_position)) => {
                if depth == 0 {
                    return Some(close_position);
                }
                depth -= 1;
                search_from = close_position + closing_tag.len();
            }
            _ => return None,
        }
    }

    None
}

fn skip_whitespace(chars: &mut std::iter::Peekable<std::str::Chars>) {
    while let Some(&character) = chars.peek() {
        if character.is_whitespace() {
            chars.next();
        } else {
            break;
        }
    }
}

fn find_next_code_fence(content: &str) -> Option<usize> {
    let mut search_from = 0;
    while search_from < content.len() {
        let backtick_fence = content[search_from..]
            .find("```")
            .map(|position| search_from + position);
        let tilde_fence = content[search_from..]
            .find("~~~")
            .map(|position| search_from + position);
        let fence_position = match (backtick_fence, tilde_fence) {
            (Some(a), Some(b)) => a.min(b),
            (Some(a), None) => a,
            (None, Some(b)) => b,
            (None, None) => return None,
        };
        if fence_position == 0 || content.as_bytes()[fence_position - 1] == b'\n' {
            return Some(fence_position);
        }
        search_from = fence_position + 3;
    }
    None
}

fn find_closing_code_fence(content: &str, fence_marker: &str) -> Option<usize> {
    let mut search_from = 0;
    while search_from < content.len() {
        if let Some(position) = content[search_from..].find(fence_marker) {
            let absolute = search_from + position;
            if absolute == 0 || content.as_bytes()[absolute - 1] == b'\n' {
                let after_marker = &content[absolute + fence_marker.len()..];
                let rest_of_line = after_marker.split('\n').next().unwrap_or("");
                if rest_of_line.trim().is_empty() {
                    return Some(absolute);
                }
            }
            search_from = absolute + fence_marker.len();
        } else {
            return None;
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn processor() -> ShortcodeProcessor {
        ShortcodeProcessor::new(&[]).unwrap()
    }

    #[test]
    fn test_parse_shortcode_args_simple() {
        let (name, args) = parse_shortcode_args("youtube id=\"abc123\"").unwrap();
        assert_eq!(name, "youtube");
        assert_eq!(args.get("id").unwrap(), "abc123");
    }

    #[test]
    fn test_parse_shortcode_args_multiple() {
        let (name, args) = parse_shortcode_args("figure src=\"img.png\" alt=\"test\"").unwrap();
        assert_eq!(name, "figure");
        assert_eq!(args.get("src").unwrap(), "img.png");
        assert_eq!(args.get("alt").unwrap(), "test");
    }

    #[test]
    fn test_parse_shortcode_args_escape_sequences() {
        let (_, args) = parse_shortcode_args("test key=\"value with \\\"quotes\\\"\"").unwrap();
        assert_eq!(args.get("key").unwrap(), "value with \"quotes\"");
    }

    #[test]
    fn test_parse_shortcode_args_empty_name() {
        assert!(parse_shortcode_args("").is_err());
    }

    #[test]
    fn test_inline_shortcode() {
        let processor = processor();
        let input = "before {{< youtube id=\"abc\" >}} after";
        let result = processor.process(input, None).unwrap();
        assert!(result.contains("before"));
        assert!(result.contains("after"));
        assert!(result.contains("abc"));
    }

    #[test]
    fn test_block_shortcode_with_body() {
        let processor = processor();
        let input = "before {{% note type=\"info\" %}}This is a note{{% /note %}} after";
        let result = processor.process(input, None).unwrap();
        assert!(result.contains("before"));
        assert!(result.contains("after"));
        assert!(result.contains("note"));
    }

    #[test]
    fn test_code_fence_skipping() {
        let processor = processor();
        let input = "```\n{{< youtube id=\"skip\" >}}\n```\n\noutside";
        let result = processor.process(input, None).unwrap();
        assert!(result.contains("{{< youtube id=\"skip\" >}}"));
        assert!(result.contains("outside"));
    }

    #[test]
    fn test_no_shortcodes() {
        let processor = processor();
        let input = "just plain text";
        let result = processor.process(input, None).unwrap();
        assert_eq!(result, "just plain text");
    }

    #[test]
    fn test_multiple_inline_shortcodes() {
        let processor = processor();
        let input = "{{< youtube id=\"abc\" >}} and {{< youtube id=\"def\" >}}";
        let result = processor.process(input, None).unwrap();
        assert!(result.contains("abc"));
        assert!(result.contains("def"));
    }

    #[test]
    fn test_nested_block_shortcodes() {
        let processor = processor();
        let input = "{{% note type=\"info\" %}}Outer {{% details summary=\"Click\" %}}Inner{{% /details %}}{{% /note %}}";
        let result = processor.process(input, None).unwrap();
        assert!(result.contains("Outer"));
        assert!(result.contains("Inner"));
    }

    #[test]
    fn test_unclosed_inline_shortcode_error() {
        let processor = processor();
        let input = "{{< youtube id=\"abc\"";
        let result = processor.process(input, None);
        assert!(result.is_err());
    }

    #[test]
    fn test_missing_closing_tag_error() {
        let processor = processor();
        let input = "{{% note type=\"info\" %}}content without closing";
        let result = processor.process(input, None);
        assert!(result.is_err());
    }

    #[test]
    fn test_shortcode_args_missing_equals() {
        let result = parse_shortcode_args("test key");
        assert!(result.is_err());
    }

    #[test]
    fn test_shortcode_args_missing_quote() {
        let result = parse_shortcode_args("test key=value");
        assert!(result.is_err());
    }

    #[test]
    fn test_shortcode_args_unclosed_string() {
        let result = parse_shortcode_args("test key=\"unclosed");
        assert!(result.is_err());
    }

    #[test]
    fn test_mixed_inline_and_block() {
        let processor = processor();
        let input = "{{< youtube id=\"vid\" >}} then {{% note type=\"warning\" %}}Warning text{{% /note %}}";
        let result = processor.process(input, None).unwrap();
        assert!(result.contains("vid"));
        assert!(result.contains("Warning"));
    }

    #[test]
    fn test_tilde_code_fence_skipping() {
        let processor = processor();
        let input = "~~~\n{{< youtube id=\"skip\" >}}\n~~~\n\noutside";
        let result = processor.process(input, None).unwrap();
        assert!(result.contains("{{< youtube id=\"skip\" >}}"));
        assert!(result.contains("outside"));
    }

    #[test]
    fn test_ref_shortcode_positional() {
        let mut processor = processor();
        let mut registry = HashMap::new();
        registry.insert("about.md".to_string(), "/about/".to_string());
        processor.set_ref_registry(registry);

        let input = r#"[About]({{< ref "about.md" >}})"#;
        let result = processor.process(input, None).unwrap();
        assert_eq!(result, "[About](/about/)");
    }

    #[test]
    fn test_ref_shortcode_with_path_key() {
        let mut processor = processor();
        let mut registry = HashMap::new();
        registry.insert("posts/hello.md".to_string(), "/posts/hello/".to_string());
        processor.set_ref_registry(registry);

        let input = r#"{{< ref path="posts/hello.md" >}}"#;
        let result = processor.process(input, None).unwrap();
        assert_eq!(result, "/posts/hello/");
    }

    #[test]
    fn test_ref_shortcode_broken_reference() {
        let processor = processor();
        let input = r#"{{< ref "nonexistent.md" >}}"#;
        let result = processor.process(input, None);
        assert!(result.is_err());
        let error = result.unwrap_err().to_string();
        assert!(error.contains("nonexistent.md"));
    }

    #[test]
    fn test_positional_arg_parsing() {
        let (name, args) = parse_shortcode_args(r#"ref "about.md""#).unwrap();
        assert_eq!(name, "ref");
        assert_eq!(args.get("_positional").unwrap(), "about.md");
    }
}
