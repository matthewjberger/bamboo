//! Data types that describe a loaded site: [`Site`], [`SiteConfig`], [`Page`],
//! [`Post`], [`Collection`], [`Content`], [`Frontmatter`], and supporting
//! metadata. These types appear in the Tera template context, so every field
//! is effectively part of the authoring-side API.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::path::PathBuf;

use crate::images::ImageConfig;

/// Describes a taxonomy (e.g. tags, categories) declared under
/// `[taxonomies.<name>]` in `bamboo.toml`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaxonomyDefinition {
    /// Singular form used on per-term pages (e.g. `"tag"` for `tags`).
    #[serde(default)]
    pub singular: Option<String>,
    /// Template override for the taxonomy index page (defaults to
    /// `<name>.html`, e.g. `tags.html`).
    #[serde(default)]
    pub index_template: Option<String>,
    /// Template override for individual term pages (defaults to the singular
    /// name, e.g. `tag.html`).
    #[serde(default)]
    pub term_template: Option<String>,
}

/// Default taxonomies (`tags` and `categories`) applied when none are
/// declared in `bamboo.toml`.
pub fn default_taxonomies() -> HashMap<String, TaxonomyDefinition> {
    let mut taxonomies = HashMap::new();
    taxonomies.insert(
        "tags".to_string(),
        TaxonomyDefinition {
            singular: Some("tag".to_string()),
            index_template: None,
            term_template: None,
        },
    );
    taxonomies.insert(
        "categories".to_string(),
        TaxonomyDefinition {
            singular: Some("category".to_string()),
            index_template: None,
            term_template: None,
        },
    );
    taxonomies
}

/// The fully-loaded site, produced by [`SiteBuilder::build`](crate::SiteBuilder::build)
/// and consumed by [`ThemeEngine::render_site`](crate::ThemeEngine::render_site).
///
/// Also serialized into every Tera template under the `site` name, so every
/// field listed here is addressable as `{{ site.<field> }}` in templates.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Site {
    /// Parsed `bamboo.toml` contents.
    pub config: SiteConfig,
    /// The home page (`content/_index.md`), if present.
    pub home: Option<Page>,
    /// All non-home pages, including nested pages under subdirectories.
    pub pages: Vec<Page>,
    /// All blog posts (`content/posts/*.md`), sorted newest-first.
    pub posts: Vec<Post>,
    /// User-defined collections keyed by name (directory containing
    /// `_collection.toml` → the [`Collection`] it produced).
    pub collections: HashMap<String, Collection>,
    /// Data from `data/*.{toml,yaml,json}`, keyed by file stem (or nested
    /// path for files in subdirectories).
    pub data: HashMap<String, Value>,
    /// Static assets (from `static/`) that will be copied to the output dir.
    pub assets: Vec<Asset>,
}

/// Parsed `bamboo.toml` contents. Also available in templates as
/// `{{ site.config }}`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SiteConfig {
    /// Human-readable site title.
    pub title: String,
    /// Absolute base URL the site will be served from (e.g.
    /// `https://example.com` or `https://user.github.io/repo`). Used to
    /// resolve links, feeds, sitemap entries, and asset paths.
    pub base_url: String,
    /// Optional site description, emitted into `<meta name="description">`
    /// and feed metadata.
    #[serde(default)]
    pub description: Option<String>,
    /// Optional author name for feed metadata.
    #[serde(default)]
    pub author: Option<String>,
    /// IETF language tag for the site (e.g. `"en"`). Defaults to `"en"` in
    /// templates that need a fallback.
    #[serde(default)]
    pub language: Option<String>,
    /// Posts per paginated index page. `0` disables pagination (all posts
    /// on one page). Defaults to 10.
    #[serde(default = "default_posts_per_page")]
    pub posts_per_page: usize,
    /// If `true`, HTML/CSS/JS output is minified in place after rendering.
    #[serde(default)]
    pub minify: bool,
    /// If `true`, CSS and JS files receive a content-hash suffix and all
    /// references to them are rewritten. Useful for aggressive cache headers.
    #[serde(default)]
    pub fingerprint: bool,
    /// Name of the syntect theme used to highlight fenced code blocks.
    /// Defaults to `base16-ocean.dark`.
    #[serde(default = "default_syntax_theme")]
    pub syntax_theme: String,
    /// Optional responsive-image pipeline configuration.
    #[serde(default)]
    pub images: Option<ImageConfig>,
    /// Taxonomy definitions. Defaults to `tags` + `categories`; override
    /// under `[taxonomies.<name>]` to add custom ones.
    #[serde(default = "default_taxonomies")]
    pub taxonomies: HashMap<String, TaxonomyDefinition>,
    /// Enable LaTeX math rendering (KaTeX) site-wide.
    #[serde(default)]
    pub math: bool,
    /// Prefixes (matched against the normalized local path after the base
    /// URL is stripped) that the post-build link validator should skip.
    /// Useful when the site shares a domain with other deployments, so
    /// `https://example.com/other-project/` doesn't get flagged as a
    /// broken internal link.
    #[serde(default)]
    pub link_check_ignore: Vec<String>,
    /// Arbitrary user fields from `[extra]`, accessible in templates as
    /// `site.config.extra.<name>`.
    #[serde(default)]
    pub extra: HashMap<String, Value>,
}

/// Default value for [`SiteConfig::posts_per_page`] (10).
pub fn default_posts_per_page() -> usize {
    10
}

/// Default value for [`SiteConfig::syntax_theme`] (`base16-ocean.dark`).
pub fn default_syntax_theme() -> String {
    "base16-ocean.dark".to_string()
}

/// One entry in a page's auto-generated table of contents.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TocEntry {
    /// Heading level (1–6), matching the source markdown `#` depth.
    pub level: u32,
    /// URL-safe heading id, suitable for use as an `#anchor`.
    pub id: String,
    /// Visible heading text with inline formatting stripped.
    pub title: String,
}

/// Content common to all renderable items: pages, posts, and collection items.
///
/// Typically accessed through the containing [`Page`], [`Post`], or
/// [`CollectionItem`], which flatten its fields into themselves via
/// `#[serde(flatten)]`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Content {
    /// URL slug (filename stem, or `slug` frontmatter override).
    pub slug: String,
    /// Display title (frontmatter `title`, filename fallback).
    pub title: String,
    /// Rendered HTML body. Serialized to templates as the `content` field so
    /// authors can write `{{ post.content | safe }}`.
    #[serde(rename = "content")]
    pub html: String,
    /// Raw markdown source before rendering. Not serialized.
    #[serde(skip_serializing)]
    pub raw_content: String,
    /// Parsed frontmatter, preserving every field for template access.
    pub frontmatter: Frontmatter,
    /// Absolute path of the source file on disk.
    pub path: PathBuf,
    /// Explicit `template = "..."` frontmatter override, if set.
    #[serde(default)]
    pub template: Option<String>,
    /// Sort order hint (lower values render first); from `weight` frontmatter.
    #[serde(default)]
    pub weight: i32,
    /// Word count of the rendered body.
    #[serde(default)]
    pub word_count: usize,
    /// Estimated reading time in minutes (at roughly 200 WPM).
    #[serde(default)]
    pub reading_time: usize,
    /// Heading-based table of contents, in source order.
    #[serde(default)]
    pub toc: Vec<TocEntry>,
    /// Resolved URL path of this content within the site (e.g.
    /// `/posts/hello/`).
    #[serde(default)]
    pub url: String,
}

/// A non-post page: either the home page (`_index.md`) or any top-level /
/// nested page under `content/`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Page {
    /// Shared content fields.
    #[serde(flatten)]
    pub content: Content,
    /// If `true`, excluded from build output unless `--drafts` is passed.
    #[serde(default)]
    pub draft: bool,
    /// Old URLs that should redirect to this page (from `redirect_from`
    /// frontmatter).
    #[serde(default)]
    pub redirect_from: Vec<String>,
}

/// A dated blog post, loaded from `content/posts/*.md`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Post {
    /// Shared content fields.
    #[serde(flatten)]
    pub content: Content,
    /// Publication date, parsed from frontmatter or the filename prefix
    /// (e.g. `2024-01-15-hello.md`).
    pub date: DateTime<Utc>,
    /// Custom excerpt. Auto-derived from the first paragraph when the
    /// `excerpt` frontmatter field is absent.
    #[serde(default)]
    pub excerpt: Option<String>,
    /// If `true`, excluded from build output unless `--drafts` is passed.
    #[serde(default)]
    pub draft: bool,
    /// Tag names from `tags` frontmatter.
    #[serde(default)]
    pub tags: Vec<String>,
    /// Category names from `categories` frontmatter.
    #[serde(default)]
    pub categories: Vec<String>,
    /// Values for any custom taxonomies configured under `[taxonomies.*]`,
    /// keyed by taxonomy name.
    #[serde(default)]
    pub taxonomies_map: HashMap<String, Vec<String>>,
    /// Old URLs that should redirect to this post (from `redirect_from`
    /// frontmatter).
    #[serde(default)]
    pub redirect_from: Vec<String>,
}

/// A named collection of content items, declared by placing a
/// `_collection.toml` file in a `content/` subdirectory.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Collection {
    /// Collection name (directory name containing the `_collection.toml`).
    pub name: String,
    /// Items belonging to this collection, in weight/filename order.
    pub items: Vec<CollectionItem>,
}

/// A single entry in a [`Collection`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollectionItem {
    /// Shared content fields.
    #[serde(flatten)]
    pub content: Content,
}

/// A static asset discovered under `static/` that will be copied verbatim
/// into the output directory.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Asset {
    /// Path to the source file on disk.
    pub source: PathBuf,
    /// Path inside the output directory where the asset will be written.
    pub dest: PathBuf,
}

/// Arbitrary key/value pairs parsed from a content file's TOML or YAML
/// frontmatter block. Unknown fields are preserved so they remain available
/// to templates as `{{ post.frontmatter.<key> }}`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Frontmatter {
    /// Raw frontmatter map. Flattened into the parent struct during
    /// serialization so template authors don't need to reach through
    /// `frontmatter.*`.
    #[serde(flatten)]
    pub raw: HashMap<String, Value>,
}

impl Frontmatter {
    /// Fetches a frontmatter field and deserializes it into `T`. Returns
    /// `None` if the key is missing or the value cannot be decoded.
    pub fn get<T: serde::de::DeserializeOwned>(&self, key: &str) -> Option<T> {
        self.raw
            .get(key)
            .and_then(|v| serde_json::from_value(v.clone()).ok())
    }

    /// Fetches a string frontmatter field, returning `None` for missing keys
    /// or non-string values.
    pub fn get_string(&self, key: &str) -> Option<String> {
        self.raw
            .get(key)
            .and_then(|value| value.as_str().map(String::from))
    }

    /// Fetches a boolean frontmatter field.
    pub fn get_bool(&self, key: &str) -> Option<bool> {
        self.raw.get(key).and_then(|value| value.as_bool())
    }

    /// Fetches a signed-integer frontmatter field.
    pub fn get_i64(&self, key: &str) -> Option<i64> {
        self.raw.get(key).and_then(|value| value.as_i64())
    }

    /// Fetches a frontmatter field as a `Vec<String>`, keeping only entries
    /// that deserialize as strings.
    pub fn get_array(&self, key: &str) -> Option<Vec<String>> {
        self.raw.get(key).and_then(|value| {
            value.as_array().map(|array| {
                array
                    .iter()
                    .filter_map(|item| item.as_str().map(String::from))
                    .collect()
            })
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frontmatter_with(key: &str, value: Value) -> Frontmatter {
        let mut raw = HashMap::new();
        raw.insert(key.to_string(), value);
        Frontmatter { raw }
    }

    #[test]
    fn test_get_string_valid() {
        let frontmatter = frontmatter_with("title", Value::String("Hello".to_string()));
        assert_eq!(frontmatter.get_string("title"), Some("Hello".to_string()));
    }

    #[test]
    fn test_get_string_missing() {
        let frontmatter = Frontmatter::default();
        assert_eq!(frontmatter.get_string("title"), None);
    }

    #[test]
    fn test_get_string_wrong_type() {
        let frontmatter = frontmatter_with("title", Value::Bool(true));
        assert_eq!(frontmatter.get_string("title"), None);
    }

    #[test]
    fn test_get_bool_valid() {
        let frontmatter = frontmatter_with("draft", Value::Bool(true));
        assert_eq!(frontmatter.get_bool("draft"), Some(true));
    }

    #[test]
    fn test_get_bool_missing() {
        let frontmatter = Frontmatter::default();
        assert_eq!(frontmatter.get_bool("draft"), None);
    }

    #[test]
    fn test_get_bool_wrong_type() {
        let frontmatter = frontmatter_with("draft", Value::String("true".to_string()));
        assert_eq!(frontmatter.get_bool("draft"), None);
    }

    #[test]
    fn test_get_i64_valid() {
        let frontmatter = frontmatter_with("weight", serde_json::json!(42));
        assert_eq!(frontmatter.get_i64("weight"), Some(42));
    }

    #[test]
    fn test_get_i64_wrong_type() {
        let frontmatter = frontmatter_with("weight", Value::String("42".to_string()));
        assert_eq!(frontmatter.get_i64("weight"), None);
    }

    #[test]
    fn test_get_array_valid() {
        let frontmatter = frontmatter_with("tags", serde_json::json!(["rust", "web"]));
        assert_eq!(
            frontmatter.get_array("tags"),
            Some(vec!["rust".to_string(), "web".to_string()])
        );
    }

    #[test]
    fn test_get_array_wrong_type() {
        let frontmatter = frontmatter_with("tags", Value::String("rust".to_string()));
        assert_eq!(frontmatter.get_array("tags"), None);
    }

    #[test]
    fn test_get_generic() {
        let frontmatter = frontmatter_with("count", serde_json::json!(5));
        assert_eq!(frontmatter.get::<i64>("count"), Some(5));
    }

    #[test]
    fn test_default_posts_per_page() {
        assert_eq!(default_posts_per_page(), 10);
    }
}
