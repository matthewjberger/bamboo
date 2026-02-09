use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::fs::File;
use std::path::Path;
use walkdir::WalkDir;

use image::codecs::jpeg::JpegEncoder;
use image::imageops::FilterType;
use image::{ImageEncoder, ImageReader};
use rayon::prelude::*;

use crate::error::Result;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ImageConfig {
    #[serde(default = "default_widths")]
    pub widths: Vec<u32>,
    #[serde(default = "default_quality")]
    pub quality: u8,
    #[serde(default = "default_formats")]
    pub formats: Vec<String>,
}

fn default_widths() -> Vec<u32> {
    vec![320, 640, 1024, 1920]
}

fn default_quality() -> u8 {
    80
}

fn default_formats() -> Vec<String> {
    vec!["webp".to_string(), "jpg".to_string()]
}

impl Default for ImageConfig {
    fn default() -> Self {
        Self {
            widths: default_widths(),
            quality: default_quality(),
            formats: default_formats(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ImageVariant {
    pub path: String,
    pub width: u32,
    pub format: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ImageManifest {
    pub variants: HashMap<String, Vec<ImageVariant>>,
}

const IMAGE_EXTENSIONS: &[&str] = &["jpg", "jpeg", "png", "gif", "webp"];

fn is_image_file(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .map(|extension| IMAGE_EXTENSIONS.contains(&extension.to_lowercase().as_str()))
        .unwrap_or(false)
}

fn is_generated_variant(path: &Path, configured_widths: &[u32]) -> bool {
    let stem = match path.file_stem().and_then(|stem| stem.to_str()) {
        Some(stem) => stem,
        None => return false,
    };
    if let Some(suffix_start) = stem.rfind('-') {
        let suffix = &stem[suffix_start + 1..];
        if let Some(digits) = suffix.strip_suffix('w')
            && let Ok(width) = digits.parse::<u32>()
        {
            return configured_widths.contains(&width);
        }
    }
    false
}

pub fn process_images(output_dir: &Path, config: &ImageConfig) -> Result<ImageManifest> {
    let image_paths: Vec<_> = WalkDir::new(output_dir)
        .into_iter()
        .filter_map(|entry| entry.ok())
        .filter(|entry| {
            let path = entry.path();
            path.is_file() && is_image_file(path) && !is_generated_variant(path, &config.widths)
        })
        .map(|entry| entry.path().to_path_buf())
        .collect();

    let results: Vec<Option<(String, Vec<ImageVariant>)>> = image_paths
        .par_iter()
        .map(|path| -> Option<(String, Vec<ImageVariant>)> {
            let source_image = match ImageReader::open(path) {
                Ok(reader) => match reader.decode() {
                    Ok(image) => image,
                    Err(error) => {
                        eprintln!(
                            "Warning: failed to decode image {}: {}",
                            path.display(),
                            error
                        );
                        return None;
                    }
                },
                Err(error) => {
                    eprintln!(
                        "Warning: failed to open image {}: {}",
                        path.display(),
                        error
                    );
                    return None;
                }
            };

            let original_width = source_image.width();
            let original_height = source_image.height();
            let stem = path
                .file_stem()
                .and_then(|stem| stem.to_str())
                .unwrap_or("image");
            let parent_directory = path.parent().unwrap_or(output_dir);

            let relative_original = path
                .strip_prefix(output_dir)
                .unwrap_or(path)
                .to_string_lossy()
                .replace('\\', "/");

            let mut image_variants = Vec::new();

            for &target_width in &config.widths {
                if target_width >= original_width {
                    continue;
                }

                let scale_factor = target_width as f64 / original_width as f64;
                let target_height = (original_height as f64 * scale_factor).round() as u32;
                let resized =
                    source_image.resize_exact(target_width, target_height, FilterType::Lanczos3);

                for format in &config.formats {
                    let variant_filename = format!("{}-{}w.{}", stem, target_width, format);
                    let variant_path = parent_directory.join(&variant_filename);

                    let write_result: std::result::Result<(), Box<dyn std::error::Error + Send + Sync>> = match format.as_str() {
                        "webp" => {
                            let rgba_image = resized.to_rgba8();
                            let encoder = webp::Encoder::from_rgba(
                                rgba_image.as_raw(),
                                resized.width(),
                                resized.height(),
                            );
                            let encoded = encoder.encode(config.quality as f32);
                            fs::write(&variant_path, &*encoded).map_err(|error| error.into())
                        }
                        "jpg" | "jpeg" => {
                            (|| -> std::result::Result<(), Box<dyn std::error::Error + Send + Sync>> {
                                let file = File::create(&variant_path)?;
                                let encoder = JpegEncoder::new_with_quality(&file, config.quality);
                                let rgb_image = resized.to_rgb8();
                                encoder.write_image(
                                    rgb_image.as_raw(),
                                    resized.width(),
                                    resized.height(),
                                    image::ExtendedColorType::Rgb8,
                                )?;
                                Ok(())
                            })()
                        }
                        _ => {
                            resized
                                .save(&variant_path)
                                .map_err(|error| error.into())
                        }
                    };

                    if let Err(error) = write_result {
                        eprintln!(
                            "Warning: failed to write image variant {}: {}",
                            variant_path.display(),
                            error
                        );
                        continue;
                    }

                    let relative_variant = variant_path
                        .strip_prefix(output_dir)
                        .unwrap_or(&variant_path)
                        .to_string_lossy()
                        .replace('\\', "/");

                    image_variants.push(ImageVariant {
                        path: relative_variant,
                        width: target_width,
                        format: format.clone(),
                    });
                }
            }

            if !image_variants.is_empty() {
                Some((relative_original, image_variants))
            } else {
                None
            }
        })
        .collect();

    let mut variants: HashMap<String, Vec<ImageVariant>> = HashMap::new();
    for result in results.into_iter().flatten() {
        variants.insert(result.0, result.1);
    }

    Ok(ImageManifest { variants })
}

pub fn generate_srcset(original_path: &str, manifest: &ImageManifest) -> String {
    let escaped_path = crate::xml::escape(original_path);
    let Some(image_variants) = manifest.variants.get(original_path) else {
        return format!("<img src=\"/{}\">", escaped_path);
    };

    if image_variants.is_empty() {
        return format!("<img src=\"/{}\">", escaped_path);
    }

    let mut formats_seen: Vec<String> = Vec::new();
    for variant in image_variants {
        let normalized = if variant.format == "jpeg" {
            "jpg".to_string()
        } else {
            variant.format.clone()
        };
        if !formats_seen.contains(&normalized) {
            formats_seen.push(normalized);
        }
    }

    let mut parts = Vec::new();
    parts.push("<picture>".to_string());

    for format in &formats_seen {
        let matching: Vec<&ImageVariant> = image_variants
            .iter()
            .filter(|variant| {
                let normalized = if variant.format == "jpeg" {
                    "jpg"
                } else {
                    &variant.format
                };
                normalized == format
            })
            .collect();

        if !matching.is_empty() {
            let mime_type = format_to_mime(format);
            let srcset_entries: Vec<String> = matching
                .iter()
                .map(|variant| format!("/{} {}w", crate::xml::escape(&variant.path), variant.width))
                .collect();
            parts.push(format!(
                "<source type=\"{}\" srcset=\"{}\">",
                mime_type,
                srcset_entries.join(", ")
            ));
        }
    }

    parts.push(format!("<img src=\"/{}\">", escaped_path));
    parts.push("</picture>".to_string());

    parts.join("")
}

fn format_to_mime(format: &str) -> &'static str {
    match format {
        "webp" => "image/webp",
        "jpg" | "jpeg" => "image/jpeg",
        "png" => "image/png",
        "gif" => "image/gif",
        "avif" => "image/avif",
        _ => "application/octet-stream",
    }
}

pub fn apply_srcset_to_html(output_dir: &Path, manifest: &ImageManifest) -> Result<()> {
    if manifest.variants.is_empty() {
        return Ok(());
    }

    for entry in WalkDir::new(output_dir)
        .into_iter()
        .filter_map(|entry| entry.ok())
    {
        let path = entry.path();
        if !path.is_file()
            || path.extension().and_then(|extension| extension.to_str()) != Some("html")
        {
            continue;
        }

        let content = fs::read_to_string(path)?;
        let updated = replace_img_tags_with_srcset(&content, manifest);

        if updated != content {
            fs::write(path, updated)?;
        }
    }

    Ok(())
}

fn find_img_tag_start(html: &str) -> Option<usize> {
    let bytes = html.as_bytes();
    let length = bytes.len();
    if length < 4 {
        return None;
    }
    let mut position = 0;
    while position + 3 < length {
        if bytes[position] == b'<'
            && bytes[position + 1].eq_ignore_ascii_case(&b'i')
            && bytes[position + 2].eq_ignore_ascii_case(&b'm')
            && bytes[position + 3].eq_ignore_ascii_case(&b'g')
        {
            let after_tag = position + 4;
            if after_tag >= length {
                return Some(position);
            }
            let next_char = bytes[after_tag];
            if next_char == b' '
                || next_char == b'\t'
                || next_char == b'\n'
                || next_char == b'\r'
                || next_char == b'/'
                || next_char == b'>'
            {
                return Some(position);
            }
        }
        position += 1;
    }
    None
}

fn find_tag_end(html: &str) -> Option<usize> {
    let mut position = 0;
    let bytes = html.as_bytes();
    let length = bytes.len();

    while position < length {
        match bytes[position] {
            b'"' => {
                position += 1;
                while position < length && bytes[position] != b'"' {
                    position += 1;
                }
                if position < length {
                    position += 1;
                }
            }
            b'\'' => {
                position += 1;
                while position < length && bytes[position] != b'\'' {
                    position += 1;
                }
                if position < length {
                    position += 1;
                }
            }
            b'>' => return Some(position),
            _ => position += 1,
        }
    }

    None
}

fn replace_img_tags_with_srcset(html: &str, manifest: &ImageManifest) -> String {
    let mut output = String::with_capacity(html.len());
    let mut remaining = html;

    while let Some(img_start) = find_img_tag_start(remaining) {
        output.push_str(&remaining[..img_start]);
        remaining = &remaining[img_start..];

        if let Some(tag_end) = find_tag_end(remaining) {
            let tag_length = tag_end + 1;
            let img_tag = &remaining[..tag_length];

            if let Some(src) = extract_src_attribute(img_tag) {
                let normalized = src.trim_start_matches('/');
                if manifest.variants.contains_key(normalized) {
                    let image_variants = &manifest.variants[normalized];
                    let mut formats_seen: Vec<String> = Vec::new();
                    for variant in image_variants {
                        let normalized_format = if variant.format == "jpeg" {
                            "jpg".to_string()
                        } else {
                            variant.format.clone()
                        };
                        if !formats_seen.contains(&normalized_format) {
                            formats_seen.push(normalized_format);
                        }
                    }

                    if !formats_seen.is_empty() {
                        output.push_str("<picture>");
                        for format in &formats_seen {
                            let matching: Vec<&ImageVariant> = image_variants
                                .iter()
                                .filter(|variant| {
                                    let nf = if variant.format == "jpeg" {
                                        "jpg"
                                    } else {
                                        &variant.format
                                    };
                                    nf == format
                                })
                                .collect();
                            let mime_type = format_to_mime(format);
                            let srcset: Vec<String> = matching
                                .iter()
                                .map(|variant| {
                                    format!(
                                        "/{} {}w",
                                        crate::xml::escape(&variant.path),
                                        variant.width
                                    )
                                })
                                .collect();
                            output.push_str(&format!(
                                "<source type=\"{}\" srcset=\"{}\">",
                                mime_type,
                                srcset.join(", ")
                            ));
                        }
                        output.push_str(img_tag);
                        output.push_str("</picture>");
                        remaining = &remaining[tag_length..];
                        continue;
                    }
                }
            }

            output.push_str(img_tag);
            remaining = &remaining[tag_length..];
        } else {
            output.push_str(remaining);
            break;
        }
    }

    output.push_str(remaining);
    output
}

fn find_standalone_src(tag: &str, pattern: &str) -> Option<usize> {
    let mut search_from = 0;
    while let Some(position) = tag[search_from..].find(pattern) {
        let absolute = search_from + position;
        if absolute == 0
            || !tag.as_bytes()[absolute - 1].is_ascii_alphanumeric()
                && tag.as_bytes()[absolute - 1] != b'-'
                && tag.as_bytes()[absolute - 1] != b'_'
        {
            return Some(absolute);
        }
        search_from = absolute + 1;
    }
    None
}

fn extract_src_attribute(tag: &str) -> Option<String> {
    let lower_tag = tag.to_ascii_lowercase();
    if let Some(src_position) = find_standalone_src(&lower_tag, "src=\"") {
        let value_start = src_position + 5;
        let rest = &tag[value_start..];
        let value_end = rest.find('"')?;
        return Some(crate::xml::unescape(&rest[..value_end]));
    }
    if let Some(src_position) = find_standalone_src(&lower_tag, "src='") {
        let value_start = src_position + 5;
        let rest = &tag[value_start..];
        let value_end = rest.find('\'')?;
        return Some(crate::xml::unescape(&rest[..value_end]));
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_image_file() {
        assert!(is_image_file(Path::new("photo.jpg")));
        assert!(is_image_file(Path::new("photo.jpeg")));
        assert!(is_image_file(Path::new("photo.png")));
        assert!(is_image_file(Path::new("photo.gif")));
        assert!(is_image_file(Path::new("photo.webp")));
        assert!(!is_image_file(Path::new("style.css")));
        assert!(!is_image_file(Path::new("readme.md")));
    }

    #[test]
    fn test_is_generated_variant() {
        let widths = vec![320, 640, 1024];
        assert!(is_generated_variant(Path::new("photo-320w.webp"), &widths));
        assert!(is_generated_variant(Path::new("photo-640w.jpg"), &widths));
        assert!(!is_generated_variant(Path::new("photo.jpg"), &widths));
        assert!(!is_generated_variant(Path::new("photo-500w.jpg"), &widths));
    }

    #[test]
    fn test_generate_srcset_no_variants() {
        let manifest = ImageManifest {
            variants: HashMap::new(),
        };
        let result = generate_srcset("images/photo.jpg", &manifest);
        assert_eq!(result, "<img src=\"/images/photo.jpg\">");
    }

    #[test]
    fn test_generate_srcset_with_variants() {
        let mut variants = HashMap::new();
        variants.insert(
            "images/photo.jpg".to_string(),
            vec![
                ImageVariant {
                    path: "images/photo-320w.webp".to_string(),
                    width: 320,
                    format: "webp".to_string(),
                },
                ImageVariant {
                    path: "images/photo-320w.jpg".to_string(),
                    width: 320,
                    format: "jpg".to_string(),
                },
            ],
        );
        let manifest = ImageManifest { variants };
        let result = generate_srcset("images/photo.jpg", &manifest);
        assert!(result.contains("<picture>"));
        assert!(result.contains("</picture>"));
        assert!(result.contains("<source"));
        assert!(result.contains("image/webp"));
        assert!(result.contains("320w"));
    }

    #[test]
    fn test_replace_img_tags_with_srcset() {
        let mut variants = HashMap::new();
        variants.insert(
            "images/photo.jpg".to_string(),
            vec![ImageVariant {
                path: "images/photo-320w.webp".to_string(),
                width: 320,
                format: "webp".to_string(),
            }],
        );
        let manifest = ImageManifest { variants };
        let html = r#"<p><img src="/images/photo.jpg"></p>"#;
        let result = replace_img_tags_with_srcset(html, &manifest);
        assert!(result.contains("<picture>"));
        assert!(result.contains("</picture>"));
    }

    #[test]
    fn test_extract_src_attribute_double_quotes() {
        assert_eq!(
            extract_src_attribute(r#"<img src="test.jpg">"#),
            Some("test.jpg".to_string())
        );
    }

    #[test]
    fn test_extract_src_attribute_single_quotes() {
        assert_eq!(
            extract_src_attribute("<img src='test.jpg'>"),
            Some("test.jpg".to_string())
        );
    }

    #[test]
    fn test_extract_src_attribute_no_src() {
        assert_eq!(extract_src_attribute("<img alt=\"test\">"), None);
    }

    #[test]
    fn test_extract_src_does_not_match_data_src() {
        assert_eq!(extract_src_attribute(r#"<img data-src="lazy.jpg">"#), None);
    }
}
