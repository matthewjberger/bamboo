pub fn escape(input: &str) -> String {
    input
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

pub fn unescape(input: &str) -> String {
    let partial = input
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&apos;", "'");

    let after_numeric = if partial.contains("&#") {
        let mut result = String::with_capacity(partial.len());
        let mut remaining = partial.as_str();

        while let Some(position) = remaining.find("&#") {
            result.push_str(&remaining[..position]);
            remaining = &remaining[position..];

            if let Some(semicolon) = remaining[..remaining.len().min(12)].find(';') {
                let entity = &remaining[2..semicolon];
                let codepoint = if entity.starts_with('x') || entity.starts_with('X') {
                    u32::from_str_radix(&entity[1..], 16).ok()
                } else {
                    entity.parse::<u32>().ok()
                };

                if let Some(character) = codepoint.and_then(char::from_u32) {
                    result.push(character);
                    remaining = &remaining[semicolon + 1..];
                    continue;
                }
            }

            result.push_str("&#");
            remaining = &remaining[2..];
        }

        result.push_str(remaining);
        result
    } else {
        partial
    };

    after_numeric.replace("&amp;", "&")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_escape_special_chars() {
        assert_eq!(escape("&"), "&amp;");
        assert_eq!(escape("<"), "&lt;");
        assert_eq!(escape(">"), "&gt;");
        assert_eq!(escape("\""), "&quot;");
        assert_eq!(escape("'"), "&apos;");
    }

    #[test]
    fn test_escape_combined() {
        assert_eq!(
            escape("<a href=\"test\">foo & bar</a>"),
            "&lt;a href=&quot;test&quot;&gt;foo &amp; bar&lt;/a&gt;"
        );
    }

    #[test]
    fn test_escape_plain_text() {
        assert_eq!(escape("hello world"), "hello world");
    }

    #[test]
    fn test_unescape_named_entities() {
        assert_eq!(unescape("&amp;"), "&");
        assert_eq!(unescape("&lt;"), "<");
        assert_eq!(unescape("&gt;"), ">");
        assert_eq!(unescape("&quot;"), "\"");
        assert_eq!(unescape("&apos;"), "'");
    }

    #[test]
    fn test_unescape_numeric_decimal() {
        assert_eq!(unescape("&#65;"), "A");
        assert_eq!(unescape("&#97;"), "a");
    }

    #[test]
    fn test_unescape_numeric_hex() {
        assert_eq!(unescape("&#x41;"), "A");
        assert_eq!(unescape("&#X61;"), "a");
    }

    #[test]
    fn test_roundtrip() {
        let original = "Hello <world> & \"friends\" 'always'";
        assert_eq!(unescape(&escape(original)), original);
    }

    #[test]
    fn test_unescape_invalid_numeric() {
        assert_eq!(unescape("&#xZZZ;"), "&#xZZZ;");
        assert_eq!(unescape("&#abc;"), "&#abc;");
    }
}
