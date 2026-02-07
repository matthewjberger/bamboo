use crate::error::{BambooError, Result};
use crate::images::ImageConfig;
use crate::parsing::{
    extract_excerpt, extract_frontmatter, parse_date_from_filename, parse_markdown, reading_time,
    word_count,
};
use crate::search::strip_html_tags;
use crate::shortcodes::ShortcodeProcessor;
use crate::types::{
    Asset, Collection, CollectionItem, Page, Post, Site, SiteConfig, default_posts_per_page,
};
use chrono::{NaiveDate, NaiveTime, TimeZone, Utc};
use serde::Deserialize;
use serde_json::Value;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

const MAX_DATA_DEPTH: usize = 10;

pub struct SiteBuilder {
    input_dir: PathBuf,
    include_drafts: bool,
    base_url_override: Option<String>,
    shortcode_processor: Option<ShortcodeProcessor>,
}

#[derive(Debug, Deserialize)]
struct RawSiteConfig {
    title: String,
    base_url: String,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    author: Option<String>,
    #[serde(default)]
    language: Option<String>,
    #[serde(default = "default_posts_per_page")]
    posts_per_page: usize,
    #[serde(default)]
    minify: bool,
    #[serde(default)]
    fingerprint: bool,
    #[serde(default)]
    images: Option<ImageConfig>,
    #[serde(default)]
    extra: HashMap<String, Value>,
}

impl SiteBuilder {
    pub fn new(input_dir: impl AsRef<Path>) -> Self {
        Self {
            input_dir: input_dir.as_ref().to_path_buf(),
            include_drafts: false,
            base_url_override: None,
            shortcode_processor: None,
        }
    }

    pub fn include_drafts(mut self, include: bool) -> Self {
        self.include_drafts = include;
        self
    }

    pub fn base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url_override = Some(url.into());
        self
    }

    pub fn shortcode_dirs(mut self, dirs: &[PathBuf]) -> Result<Self> {
        self.shortcode_processor = Some(ShortcodeProcessor::new(dirs)?);
        Ok(self)
    }

    pub fn build(&mut self) -> Result<Site> {
        let mut config = self.load_config()?;

        if let Some(ref url) = self.base_url_override {
            config.base_url = url.trim_end_matches('/').to_string();
        }

        if self.shortcode_processor.is_none() {
            let mut dirs = Vec::new();
            let site_shortcodes = self.input_dir.join("templates").join("shortcodes");
            if site_shortcodes.is_dir() {
                dirs.push(site_shortcodes);
            }
            self.shortcode_processor = Some(ShortcodeProcessor::new(&dirs)?);
        }

        let (home, mut pages) = self.load_pages()?;
        let posts = self.load_posts()?;
        let mut collections = self.load_collections()?;
        let data = self.load_data()?;
        let assets = self.collect_assets()?;

        pages.sort_by(|a, b| a.weight.cmp(&b.weight).then_with(|| a.slug.cmp(&b.slug)));

        for collection in collections.values_mut() {
            collection
                .items
                .sort_by(|a, b| a.weight.cmp(&b.weight).then_with(|| a.slug.cmp(&b.slug)));
        }

        Ok(Site {
            config,
            home,
            pages,
            posts,
            collections,
            data,
            assets,
        })
    }

    fn load_config(&self) -> Result<SiteConfig> {
        let config_path = self.input_dir.join("bamboo.toml");

        if !config_path.exists() {
            return Err(BambooError::ConfigNotFound { path: config_path });
        }

        let content = fs::read_to_string(&config_path)?;
        let raw: RawSiteConfig =
            toml::from_str(&content).map_err(|error| BambooError::TomlParse {
                path: config_path.clone(),
                message: error.to_string(),
            })?;

        Ok(SiteConfig {
            title: raw.title,
            base_url: raw.base_url.trim_end_matches('/').to_string(),
            description: raw.description,
            author: raw.author,
            language: raw.language,
            posts_per_page: raw.posts_per_page,
            minify: raw.minify,
            fingerprint: raw.fingerprint,
            images: raw.images,
            extra: raw.extra,
        })
    }

    fn load_pages(&self) -> Result<(Option<Page>, Vec<Page>)> {
        let content_dir = self.input_dir.join("content");
        let mut pages = Vec::new();
        let mut home = None;
        let mut seen_slugs: HashMap<String, PathBuf> = HashMap::new();

        if !content_dir.exists() {
            return Ok((home, pages));
        }

        let skip_dirs = self.find_reserved_dirs(&content_dir)?;

        for entry in WalkDir::new(&content_dir)
            .min_depth(1)
            .into_iter()
            .filter_entry(|entry| {
                let path = entry.path();
                if path.is_dir() {
                    !skip_dirs.contains(&path.to_path_buf())
                } else {
                    true
                }
            })
        {
            let entry = entry.map_err(|error| BambooError::WalkDir {
                path: content_dir.clone(),
                message: error.to_string(),
            })?;

            let path = entry.path();

            if !path.is_file() {
                continue;
            }

            if path
                .extension()
                .map(|extension| extension != "md")
                .unwrap_or(true)
            {
                continue;
            }

            let filename = path.file_name().unwrap().to_string_lossy();

            if filename.starts_with('_') && filename != "_index.md" {
                continue;
            }

            let relative =
                path.strip_prefix(&content_dir)
                    .map_err(|_| BambooError::InvalidPath {
                        path: path.to_path_buf(),
                    })?;

            let page = self.parse_page(path, relative)?;

            if page.draft && !self.include_drafts {
                continue;
            }

            if page.slug == "index"
                && relative
                    .parent()
                    .map(|parent| parent == Path::new(""))
                    .unwrap_or(true)
            {
                home = Some(page);
            } else {
                if let Some(existing_path) = seen_slugs.get(&page.slug) {
                    return Err(BambooError::DuplicatePage {
                        slug: page.slug,
                        path: path.to_path_buf(),
                        existing_path: existing_path.clone(),
                    });
                }
                seen_slugs.insert(page.slug.clone(), path.to_path_buf());
                pages.push(page);
            }
        }

        Ok((home, pages))
    }

    fn find_reserved_dirs(&self, content_dir: &Path) -> Result<Vec<PathBuf>> {
        let mut reserved = vec![content_dir.join("posts")];

        for entry in WalkDir::new(content_dir)
            .min_depth(1)
            .max_depth(1)
            .into_iter()
        {
            let entry = entry.map_err(|error| BambooError::WalkDir {
                path: content_dir.to_path_buf(),
                message: error.to_string(),
            })?;
            let path = entry.path();
            if path.is_dir() && path.join("_collection.toml").exists() {
                reserved.push(path.to_path_buf());
            }
        }

        Ok(reserved)
    }

    fn process_shortcodes(&self, content: &str) -> Result<String> {
        if let Some(ref processor) = self.shortcode_processor {
            processor.process(content)
        } else {
            Ok(content.to_string())
        }
    }

    fn parse_page(&self, path: &Path, relative: &Path) -> Result<Page> {
        let content = fs::read_to_string(path)?;
        let (frontmatter, raw_content) = extract_frontmatter(&content, path)?;
        let processed_content = self.process_shortcodes(&raw_content)?;
        let rendered = parse_markdown(&processed_content);

        let filename = path.file_name().unwrap().to_string_lossy();

        let relative_dir = relative.parent().unwrap_or(Path::new(""));

        let file_slug = if filename == "_index.md" {
            "index".to_string()
        } else {
            filename
                .strip_suffix(".md")
                .unwrap_or(&filename)
                .to_string()
        };

        let slug = if relative_dir == Path::new("") {
            file_slug.clone()
        } else {
            let dir_part = relative_dir.to_string_lossy().replace('\\', "/");
            if file_slug == "index" {
                dir_part.to_string()
            } else {
                format!("{}/{}", dir_part, file_slug)
            }
        };

        let title = frontmatter
            .get_string("title")
            .unwrap_or_else(|| file_slug.clone());

        let template = frontmatter.get_string("template");
        let weight = frontmatter.get_i64("weight").unwrap_or(0) as i32;
        let draft = frontmatter.get_bool("draft").unwrap_or(false);
        let redirect_from = frontmatter.get_array("redirect_from").unwrap_or_default();

        let output_path = if slug == "index" {
            PathBuf::from("index.html")
        } else {
            PathBuf::from(&slug).join("index.html")
        };

        let url = if slug == "index" {
            "/".to_string()
        } else {
            format!("/{}/", slug)
        };

        let plain_text = strip_html_tags(&rendered.html);
        let words = word_count(&plain_text);

        Ok(Page {
            slug,
            title,
            content: rendered.html,
            raw_content,
            frontmatter,
            path: output_path,
            template,
            weight,
            word_count: words,
            reading_time: reading_time(words),
            toc: rendered.toc,
            url,
            draft,
            redirect_from,
        })
    }

    fn load_posts(&self) -> Result<Vec<Post>> {
        let posts_dir = self.input_dir.join("content").join("posts");
        let mut posts = Vec::new();

        if !posts_dir.exists() {
            return Ok(posts);
        }

        for entry in WalkDir::new(&posts_dir)
            .min_depth(1)
            .max_depth(1)
            .into_iter()
        {
            let entry = entry.map_err(|error| BambooError::WalkDir {
                path: posts_dir.clone(),
                message: error.to_string(),
            })?;

            let path = entry.path();

            if !path.is_file() {
                continue;
            }

            if path
                .extension()
                .map(|extension| extension != "md")
                .unwrap_or(true)
            {
                continue;
            }

            let filename = path.file_name().unwrap().to_string_lossy();

            if filename.starts_with('_') {
                continue;
            }

            let post = self.parse_post(path)?;

            if post.draft && !self.include_drafts {
                continue;
            }

            posts.push(post);
        }

        posts.sort_by(|a, b| b.date.cmp(&a.date));

        Ok(posts)
    }

    fn parse_post(&self, path: &Path) -> Result<Post> {
        let content = fs::read_to_string(path)?;
        let (frontmatter, raw_content) = extract_frontmatter(&content, path)?;
        let processed_content = self.process_shortcodes(&raw_content)?;
        let rendered = parse_markdown(&processed_content);

        let filename = path.file_name().unwrap().to_string_lossy();

        let (date_str, slug) = if let Some((date, slug)) = parse_date_from_filename(&filename) {
            (Some(date), slug)
        } else {
            let slug = filename
                .strip_suffix(".md")
                .unwrap_or(&filename)
                .to_string();
            (frontmatter.get_string("date"), slug)
        };

        let date = if let Some(date_str) = date_str {
            let naive = NaiveDate::parse_from_str(&date_str, "%Y-%m-%d").map_err(|_| {
                BambooError::InvalidDate {
                    path: path.to_path_buf(),
                }
            })?;
            Utc.from_utc_datetime(&naive.and_time(NaiveTime::MIN))
        } else {
            return Err(BambooError::MissingField {
                field: "date".to_string(),
                path: path.to_path_buf(),
            });
        };

        let title = frontmatter
            .get_string("title")
            .unwrap_or_else(|| slug.clone());
        let draft = frontmatter.get_bool("draft").unwrap_or(false);
        let tags = frontmatter.get_array("tags").unwrap_or_default();
        let categories = frontmatter.get_array("categories").unwrap_or_default();
        let template = frontmatter.get_string("template");
        let redirect_from = frontmatter.get_array("redirect_from").unwrap_or_default();

        let excerpt = frontmatter
            .get_string("excerpt")
            .or_else(|| extract_excerpt(&raw_content, 200));

        let output_path = PathBuf::from("posts").join(&slug).join("index.html");
        let url = format!("/posts/{}/", slug);
        let plain_text = strip_html_tags(&rendered.html);
        let words = word_count(&plain_text);

        Ok(Post {
            slug,
            title,
            date,
            content: rendered.html,
            raw_content,
            excerpt,
            frontmatter,
            path: output_path,
            draft,
            tags,
            categories,
            template,
            word_count: words,
            reading_time: reading_time(words),
            toc: rendered.toc,
            url,
            redirect_from,
        })
    }

    fn load_collections(&self) -> Result<HashMap<String, Collection>> {
        let content_dir = self.input_dir.join("content");
        let mut collections = HashMap::new();

        if !content_dir.exists() {
            return Ok(collections);
        }

        for entry in WalkDir::new(&content_dir)
            .min_depth(1)
            .max_depth(1)
            .into_iter()
        {
            let entry = entry.map_err(|error| BambooError::WalkDir {
                path: content_dir.clone(),
                message: error.to_string(),
            })?;

            let path = entry.path();

            if !path.is_dir() {
                continue;
            }

            let dir_name = path.file_name().unwrap().to_string_lossy();

            if dir_name == "posts" {
                continue;
            }

            let collection_config = path.join("_collection.toml");
            if !collection_config.exists() {
                continue;
            }

            let collection = self.load_collection(path, &dir_name)?;
            collections.insert(dir_name.to_string(), collection);
        }

        Ok(collections)
    }

    fn load_collection(&self, dir: &Path, name: &str) -> Result<Collection> {
        let mut items = Vec::new();

        for entry in WalkDir::new(dir).min_depth(1).max_depth(1).into_iter() {
            let entry = entry.map_err(|error| BambooError::WalkDir {
                path: dir.to_path_buf(),
                message: error.to_string(),
            })?;

            let path = entry.path();

            if !path.is_file() {
                continue;
            }

            if path
                .extension()
                .map(|extension| extension != "md")
                .unwrap_or(true)
            {
                continue;
            }

            let filename = path.file_name().unwrap().to_string_lossy();

            if filename.starts_with('_') {
                continue;
            }

            let item = self.parse_collection_item(path, name)?;
            items.push(item);
        }

        Ok(Collection {
            name: name.to_string(),
            items,
        })
    }

    fn parse_collection_item(&self, path: &Path, collection_name: &str) -> Result<CollectionItem> {
        let content = fs::read_to_string(path)?;
        let (frontmatter, raw_content) = extract_frontmatter(&content, path)?;
        let processed_content = self.process_shortcodes(&raw_content)?;
        let rendered = parse_markdown(&processed_content);

        let filename = path.file_name().unwrap().to_string_lossy();
        let slug = filename
            .strip_suffix(".md")
            .unwrap_or(&filename)
            .to_string();

        let title = frontmatter
            .get_string("title")
            .unwrap_or_else(|| slug.clone());
        let template = frontmatter.get_string("template");
        let weight = frontmatter.get_i64("weight").unwrap_or(0) as i32;

        let output_path = PathBuf::from(collection_name)
            .join(&slug)
            .join("index.html");

        let url = format!("/{}/{}/", collection_name, slug);
        let plain_text = strip_html_tags(&rendered.html);
        let words = word_count(&plain_text);

        Ok(CollectionItem {
            slug,
            title,
            content: rendered.html,
            raw_content,
            frontmatter,
            path: output_path,
            template,
            weight,
            word_count: words,
            reading_time: reading_time(words),
            toc: rendered.toc,
            url,
        })
    }

    fn load_data(&self) -> Result<HashMap<String, Value>> {
        let data_dir = self.input_dir.join("data");
        let mut data = HashMap::new();

        if !data_dir.exists() {
            return Ok(data);
        }

        for entry in WalkDir::new(&data_dir)
            .min_depth(1)
            .max_depth(MAX_DATA_DEPTH)
            .into_iter()
        {
            let entry = entry.map_err(|error| BambooError::WalkDir {
                path: data_dir.clone(),
                message: error.to_string(),
            })?;

            let path = entry.path();

            if !path.is_file() {
                continue;
            }

            let extension = path.extension().and_then(|ext| ext.to_str()).unwrap_or("");

            if !["toml", "yaml", "yml", "json"].contains(&extension) {
                continue;
            }

            let relative = path
                .strip_prefix(&data_dir)
                .map_err(|_| BambooError::InvalidPath {
                    path: path.to_path_buf(),
                })?;

            let content = fs::read_to_string(path)?;

            let value: Value = match extension {
                "toml" => toml::from_str(&content).map_err(|error| BambooError::TomlParse {
                    path: path.to_path_buf(),
                    message: error.to_string(),
                })?,
                "yaml" | "yml" => {
                    serde_yaml::from_str(&content).map_err(|error| BambooError::YamlParse {
                        path: path.to_path_buf(),
                        message: error.to_string(),
                    })?
                }
                "json" => {
                    serde_json::from_str(&content).map_err(|error| BambooError::JsonParse {
                        path: path.to_path_buf(),
                        message: error.to_string(),
                    })?
                }
                _ => continue,
            };

            let key = build_data_key(relative);
            insert_nested_value(&mut data, &key, value);
        }

        Ok(data)
    }

    fn collect_assets(&self) -> Result<Vec<Asset>> {
        let static_dir = self.input_dir.join("static");
        let mut assets = Vec::new();

        if !static_dir.exists() {
            return Ok(assets);
        }

        for entry in WalkDir::new(&static_dir).min_depth(1).into_iter() {
            let entry = entry.map_err(|error| BambooError::WalkDir {
                path: static_dir.clone(),
                message: error.to_string(),
            })?;

            let path = entry.path();

            if !path.is_file() {
                continue;
            }

            let relative = path.strip_prefix(&static_dir).unwrap();

            assets.push(Asset {
                source: path.to_path_buf(),
                dest: relative.to_path_buf(),
            });
        }

        Ok(assets)
    }
}

fn build_data_key(path: &Path) -> Vec<String> {
    let mut parts: Vec<String> = path
        .parent()
        .map(|parent| {
            parent
                .iter()
                .map(|segment| segment.to_string_lossy().to_string())
                .collect()
        })
        .unwrap_or_default();

    if let Some(stem) = path.file_stem() {
        parts.push(stem.to_string_lossy().to_string());
    }

    parts
}

fn insert_nested_value(data: &mut HashMap<String, Value>, keys: &[String], value: Value) {
    if keys.is_empty() {
        return;
    }

    if keys.len() == 1 {
        if let Some(existing) = data.get(&keys[0])
            && existing.is_object()
        {
            if let Value::Object(new_map) = &value
                && let Some(Value::Object(existing_map)) = data.get_mut(&keys[0])
            {
                for (key, val) in new_map {
                    existing_map.insert(key.clone(), val.clone());
                }
            }
            return;
        }
        data.insert(keys[0].clone(), value);
        return;
    }

    let first = &keys[0];
    let rest = &keys[1..];

    let nested = data
        .entry(first.clone())
        .or_insert_with(|| Value::Object(serde_json::Map::new()));

    if !nested.is_object() {
        return;
    }

    if let Value::Object(map) = nested {
        insert_nested_value_map(map, rest, value);
    }
}

fn insert_nested_value_map(
    map: &mut serde_json::Map<String, Value>,
    keys: &[String],
    value: Value,
) {
    if keys.is_empty() {
        return;
    }

    if keys.len() == 1 {
        if let Some(existing) = map.get(&keys[0])
            && existing.is_object()
        {
            if let Value::Object(new_map) = &value
                && let Some(Value::Object(existing_map)) = map.get_mut(&keys[0])
            {
                for (key, val) in new_map {
                    existing_map.insert(key.clone(), val.clone());
                }
            }
            return;
        }
        map.insert(keys[0].clone(), value);
        return;
    }

    let first = &keys[0];
    let rest = &keys[1..];

    let nested = map
        .entry(first.clone())
        .or_insert_with(|| Value::Object(serde_json::Map::new()));

    if !nested.is_object() {
        return;
    }

    if let Value::Object(inner_map) = nested {
        insert_nested_value_map(inner_map, rest, value);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn create_test_site() -> TempDir {
        let dir = TempDir::new().unwrap();

        fs::write(
            dir.path().join("bamboo.toml"),
            r#"
title = "Test Site"
base_url = "https://example.com"
description = "A test site"
"#,
        )
        .unwrap();

        fs::create_dir_all(dir.path().join("content/posts")).unwrap();

        fs::write(
            dir.path().join("content/_index.md"),
            r#"+++
title = "Home"
+++

Welcome!"#,
        )
        .unwrap();

        fs::write(
            dir.path().join("content/about.md"),
            r#"+++
title = "About"
weight = 10
+++

About page"#,
        )
        .unwrap();

        fs::write(
            dir.path().join("content/contact.md"),
            r#"+++
title = "Contact"
weight = 5
+++

Contact page"#,
        )
        .unwrap();

        fs::write(
            dir.path().join("content/posts/2024-01-15-hello.md"),
            r#"+++
title = "Hello World"
tags = ["test"]
+++

First paragraph for excerpt.

Second paragraph."#,
        )
        .unwrap();

        dir
    }

    #[test]
    fn test_build_site() {
        let dir = create_test_site();
        let mut builder = SiteBuilder::new(dir.path());
        let site = builder.build().unwrap();

        assert_eq!(site.config.title, "Test Site");
        assert!(site.home.is_some());
        assert_eq!(site.pages.len(), 2);
        assert_eq!(site.posts.len(), 1);
    }

    #[test]
    fn test_page_sorting_by_weight() {
        let dir = create_test_site();
        let mut builder = SiteBuilder::new(dir.path());
        let site = builder.build().unwrap();

        assert_eq!(site.pages[0].slug, "contact");
        assert_eq!(site.pages[1].slug, "about");
    }

    #[test]
    fn test_post_excerpt() {
        let dir = create_test_site();
        let mut builder = SiteBuilder::new(dir.path());
        let site = builder.build().unwrap();

        let post = &site.posts[0];
        assert!(post.excerpt.is_some());
        assert!(post.excerpt.as_ref().unwrap().contains("First paragraph"));
    }

    #[test]
    fn test_base_url_override() {
        let dir = create_test_site();
        let mut builder = SiteBuilder::new(dir.path()).base_url("https://custom.com");
        let site = builder.build().unwrap();

        assert_eq!(site.config.base_url, "https://custom.com");
    }

    #[test]
    fn test_nested_data() {
        let dir = create_test_site();

        fs::create_dir_all(dir.path().join("data/nav")).unwrap();
        fs::write(
            dir.path().join("data/nav/main.toml"),
            r#"
[[items]]
name = "Home"
url = "/"
"#,
        )
        .unwrap();

        let mut builder = SiteBuilder::new(dir.path());
        let site = builder.build().unwrap();

        assert!(site.data.contains_key("nav"));
        let nav = site.data.get("nav").unwrap();
        assert!(nav.get("main").is_some());
    }
}
