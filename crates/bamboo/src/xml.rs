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
