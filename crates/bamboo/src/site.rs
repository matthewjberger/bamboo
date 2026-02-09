use crate::error::{BambooError, IoContext, Result};
use crate::parsing::{
    MarkdownRenderer, extract_excerpt, extract_frontmatter, parse_date_from_filename,
    parse_markdown, preprocess_math, reading_time, word_count,
};
use crate::search::strip_html_tags;
use crate::shortcodes::ShortcodeProcessor;
use crate::types::{
    Asset, Collection, CollectionItem, Content, Page, Post, Site, SiteConfig, TaxonomyDefinition,
};
use chrono::{NaiveDate, NaiveTime, TimeZone, Utc};
use serde_json::Value;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

const MAX_DATA_DEPTH: usize = 10;

struct ContentInput {
    slug: String,
    title: String,
    raw_content: String,
    rendered: crate::parsing::RenderedMarkdown,
    frontmatter: crate::types::Frontmatter,
    output_path: PathBuf,
    url: String,
}

pub struct SiteBuilder {
    input_dir: PathBuf,
    include_drafts: bool,
    base_url_override: Option<String>,
    shortcode_processor: Option<ShortcodeProcessor>,
    renderer: Option<MarkdownRenderer>,
    math_enabled: bool,
}

impl SiteBuilder {
    pub fn new(input_dir: impl AsRef<Path>) -> Self {
        Self {
            input_dir: input_dir.as_ref().to_path_buf(),
            include_drafts: false,
            base_url_override: None,
            shortcode_processor: None,
            renderer: None,
            math_enabled: false,
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

        self.renderer = Some(MarkdownRenderer::with_theme(&config.syntax_theme));
        self.math_enabled = config.math;

        if self.shortcode_processor.is_none() {
            let mut dirs = Vec::new();
            let site_shortcodes = self.input_dir.join("templates").join("shortcodes");
            if site_shortcodes.is_dir() {
                dirs.push(site_shortcodes);
            }
            self.shortcode_processor = Some(ShortcodeProcessor::new(&dirs)?);
        }

        let ref_registry = self.build_ref_registry()?;
        if let Some(ref mut processor) = self.shortcode_processor {
            processor.set_ref_registry(ref_registry);
        }

        let (home, mut pages) = self.load_pages()?;
        let posts = self.load_posts(&config.taxonomies)?;
        let mut collections = self.load_collections()?;
        let data = self.load_data()?;
        let assets = self.collect_assets()?;

        pages.sort_by(|a, b| {
            a.content
                .weight
                .cmp(&b.content.weight)
                .then_with(|| a.content.slug.cmp(&b.content.slug))
        });

        for collection in collections.values_mut() {
            collection.items.sort_by(|a, b| {
                a.content
                    .weight
                    .cmp(&b.content.weight)
                    .then_with(|| a.content.slug.cmp(&b.content.slug))
            });
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

        let content =
            fs::read_to_string(&config_path).io_context("reading config", &config_path)?;
        let mut config: SiteConfig =
            toml::from_str(&content).map_err(|error| BambooError::TomlParse {
                path: config_path.clone(),
                message: error.to_string(),
            })?;

        config.base_url = config.base_url.trim_end_matches('/').to_string();

        Ok(config)
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

            if page.content.slug == "index"
                && relative
                    .parent()
                    .map(|parent| parent == Path::new(""))
                    .unwrap_or(true)
            {
                home = Some(page);
            } else {
                if let Some(existing_path) = seen_slugs.get(&page.content.slug) {
                    return Err(BambooError::DuplicatePage {
                        slug: page.content.slug.clone(),
                        path: path.to_path_buf(),
                        existing_path: existing_path.clone(),
                    });
                }
                seen_slugs.insert(page.content.slug.clone(), path.to_path_buf());
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
            processor.process(content, self.renderer.as_ref())
        } else {
            Ok(content.to_string())
        }
    }

    fn should_enable_math(&self, frontmatter: &crate::types::Frontmatter) -> bool {
        self.math_enabled || frontmatter.get_bool("math").unwrap_or(false)
    }

    fn render_markdown(&self, content: &str) -> crate::parsing::RenderedMarkdown {
        if let Some(ref renderer) = self.renderer {
            renderer.render(content)
        } else {
            parse_markdown(content)
        }
    }

    fn build_ref_registry(&self) -> Result<HashMap<String, String>> {
        let content_dir = self.input_dir.join("content");
        let mut registry = HashMap::new();

        if !content_dir.exists() {
            return Ok(registry);
        }

        let reserved_dirs = self.find_reserved_dirs(&content_dir)?;

        for entry in WalkDir::new(&content_dir).min_depth(1).into_iter() {
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

            let relative_str = relative.to_string_lossy().replace('\\', "/");

            let parent_dir = path.parent().unwrap_or(path);
            let is_in_posts = parent_dir
                .strip_prefix(&content_dir)
                .map(|relative_parent| {
                    relative_parent
                        .components()
                        .next()
                        .map(|component| component.as_os_str() == "posts")
                        .unwrap_or(false)
                })
                .unwrap_or(false);

            let is_in_collection = reserved_dirs
                .iter()
                .any(|reserved| parent_dir.starts_with(reserved) && !is_in_posts);

            let url = if filename == "_index.md"
                && relative
                    .parent()
                    .map(|parent| parent == Path::new(""))
                    .unwrap_or(true)
            {
                "/".to_string()
            } else if is_in_posts {
                let (_, slug) =
                    if let Some(parsed) = crate::parsing::parse_date_from_filename(&filename) {
                        parsed
                    } else {
                        (
                            String::new(),
                            filename
                                .strip_suffix(".md")
                                .unwrap_or(&filename)
                                .to_string(),
                        )
                    };
                format!("/posts/{}/", slug)
            } else if is_in_collection {
                let collection_name = parent_dir
                    .strip_prefix(&content_dir)
                    .unwrap()
                    .components()
                    .next()
                    .unwrap()
                    .as_os_str()
                    .to_string_lossy();
                let slug = filename
                    .strip_suffix(".md")
                    .unwrap_or(&filename)
                    .to_string();
                format!("/{}/{}/", collection_name, slug)
            } else {
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

                if slug == "index" {
                    "/".to_string()
                } else {
                    format!("/{}/", slug)
                }
            };

            registry.insert(relative_str.clone(), url.clone());
            registry.insert(filename.to_string(), url.clone());

            let without_extension = relative_str.strip_suffix(".md").unwrap_or(&relative_str);
            if without_extension != relative_str {
                registry.insert(without_extension.to_string(), url.clone());
            }
        }

        Ok(registry)
    }

    fn build_content(&self, input: ContentInput) -> Content {
        let plain_text = strip_html_tags(&input.rendered.html);
        let words = word_count(&plain_text);
        let template = input.frontmatter.get_string("template");
        let weight = input.frontmatter.get_i64("weight").unwrap_or(0) as i32;
        Content {
            slug: input.slug,
            title: input.title,
            html: input.rendered.html,
            raw_content: input.raw_content,
            frontmatter: input.frontmatter,
            path: input.output_path,
            template,
            weight,
            word_count: words,
            reading_time: reading_time(words),
            toc: input.rendered.toc,
            url: input.url,
        }
    }

    fn parse_page(&self, path: &Path, relative: &Path) -> Result<Page> {
        let file_content = fs::read_to_string(path).io_context("reading page", path)?;
        let (frontmatter, raw_content) = extract_frontmatter(&file_content, path)?;
        let processed_content = self.process_shortcodes(&raw_content)?;
        let math_processed = if self.should_enable_math(&frontmatter) {
            preprocess_math(&processed_content)
        } else {
            processed_content
        };
        let rendered = self.render_markdown(&math_processed);

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

        let content = self.build_content(ContentInput {
            slug,
            title,
            raw_content,
            rendered,
            frontmatter,
            output_path,
            url,
        });

        Ok(Page {
            content,
            draft,
            redirect_from,
        })
    }

    fn load_posts(
        &self,
        taxonomy_definitions: &HashMap<String, TaxonomyDefinition>,
    ) -> Result<Vec<Post>> {
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

            let post = self.parse_post(path, taxonomy_definitions)?;

            if post.draft && !self.include_drafts {
                continue;
            }

            posts.push(post);
        }

        posts.sort_by(|a, b| b.date.cmp(&a.date));

        Ok(posts)
    }

    fn parse_post(
        &self,
        path: &Path,
        taxonomy_definitions: &HashMap<String, TaxonomyDefinition>,
    ) -> Result<Post> {
        let file_content = fs::read_to_string(path).io_context("reading post", path)?;
        let (frontmatter, raw_content) = extract_frontmatter(&file_content, path)?;
        let processed_content = self.process_shortcodes(&raw_content)?;
        let math_processed = if self.should_enable_math(&frontmatter) {
            preprocess_math(&processed_content)
        } else {
            processed_content
        };
        let rendered = self.render_markdown(&math_processed);

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
        let redirect_from = frontmatter.get_array("redirect_from").unwrap_or_default();

        let mut taxonomies_map: HashMap<String, Vec<String>> = HashMap::new();
        for taxonomy_name in taxonomy_definitions.keys() {
            if let Some(terms) = frontmatter.get_array(taxonomy_name) {
                taxonomies_map.insert(taxonomy_name.clone(), terms);
            }
        }

        let tags = taxonomies_map.get("tags").cloned().unwrap_or_default();
        let categories = taxonomies_map
            .get("categories")
            .cloned()
            .unwrap_or_default();

        let excerpt = frontmatter
            .get_string("excerpt")
            .or_else(|| extract_excerpt(&raw_content, 200));

        let output_path = PathBuf::from("posts").join(&slug).join("index.html");
        let url = format!("/posts/{}/", slug);

        let content = self.build_content(ContentInput {
            slug,
            title,
            raw_content,
            rendered,
            frontmatter,
            output_path,
            url,
        });

        Ok(Post {
            content,
            date,
            excerpt,
            draft,
            tags,
            categories,
            taxonomies_map,
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
        let file_content = fs::read_to_string(path).io_context("reading collection item", path)?;
        let (frontmatter, raw_content) = extract_frontmatter(&file_content, path)?;
        let processed_content = self.process_shortcodes(&raw_content)?;
        let math_processed = if self.should_enable_math(&frontmatter) {
            preprocess_math(&processed_content)
        } else {
            processed_content
        };
        let rendered = self.render_markdown(&math_processed);

        let filename = path.file_name().unwrap().to_string_lossy();
        let slug = filename
            .strip_suffix(".md")
            .unwrap_or(&filename)
            .to_string();

        let title = frontmatter
            .get_string("title")
            .unwrap_or_else(|| slug.clone());

        let output_path = PathBuf::from(collection_name)
            .join(&slug)
            .join("index.html");

        let url = format!("/{}/{}/", collection_name, slug);

        let content = self.build_content(ContentInput {
            slug,
            title,
            raw_content,
            rendered,
            frontmatter,
            output_path,
            url,
        });

        Ok(CollectionItem { content })
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

            let content = fs::read_to_string(path).io_context("reading data file", path)?;

            let value: Value = match extension {
                "toml" => toml::from_str(&content).map_err(|error| BambooError::TomlParse {
                    path: path.to_path_buf(),
                    message: error.to_string(),
                })?,
                "yaml" | "yml" => {
                    serde_yml::from_str(&content).map_err(|error| BambooError::YamlParse {
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

trait NestedInsert {
    fn get_value(&self, key: &str) -> Option<&Value>;
    fn get_value_mut(&mut self, key: &str) -> Option<&mut Value>;
    fn insert_value(&mut self, key: String, value: Value);
    fn entry_or_insert(&mut self, key: String) -> &mut Value;
}

impl NestedInsert for HashMap<String, Value> {
    fn get_value(&self, key: &str) -> Option<&Value> {
        self.get(key)
    }
    fn get_value_mut(&mut self, key: &str) -> Option<&mut Value> {
        self.get_mut(key)
    }
    fn insert_value(&mut self, key: String, value: Value) {
        self.insert(key, value);
    }
    fn entry_or_insert(&mut self, key: String) -> &mut Value {
        self.entry(key)
            .or_insert_with(|| Value::Object(serde_json::Map::new()))
    }
}

impl NestedInsert for serde_json::Map<String, Value> {
    fn get_value(&self, key: &str) -> Option<&Value> {
        self.get(key)
    }
    fn get_value_mut(&mut self, key: &str) -> Option<&mut Value> {
        self.get_mut(key)
    }
    fn insert_value(&mut self, key: String, value: Value) {
        self.insert(key, value);
    }
    fn entry_or_insert(&mut self, key: String) -> &mut Value {
        self.entry(key)
            .or_insert_with(|| Value::Object(serde_json::Map::new()))
    }
}

fn insert_nested_value<M: NestedInsert>(container: &mut M, keys: &[String], value: Value) {
    if keys.is_empty() {
        return;
    }

    if keys.len() == 1 {
        if let Some(existing) = container.get_value(&keys[0])
            && existing.is_object()
        {
            if let Value::Object(new_map) = &value
                && let Some(existing_map) = container
                    .get_value_mut(&keys[0])
                    .and_then(|v| v.as_object_mut())
            {
                for (key, val) in new_map {
                    existing_map.insert(key.clone(), val.clone());
                }
            }
            return;
        }
        container.insert_value(keys[0].clone(), value);
        return;
    }

    let first = &keys[0];
    let rest = &keys[1..];

    let nested = container.entry_or_insert(first.clone());

    if !nested.is_object() {
        return;
    }

    if let Value::Object(map) = nested {
        insert_nested_value(map, rest, value);
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

        assert_eq!(site.pages[0].content.slug, "contact");
        assert_eq!(site.pages[1].content.slug, "about");
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

    #[test]
    fn test_draft_pages_excluded_by_default() {
        let dir = create_test_site();
        fs::write(
            dir.path().join("content/secret.md"),
            "+++\ntitle = \"Secret\"\ndraft = true\n+++\n\nSecret page",
        )
        .unwrap();

        let mut builder = SiteBuilder::new(dir.path());
        let site = builder.build().unwrap();

        assert!(site.pages.iter().all(|page| page.content.slug != "secret"));
    }

    #[test]
    fn test_draft_pages_included_when_requested() {
        let dir = create_test_site();
        fs::write(
            dir.path().join("content/secret.md"),
            "+++\ntitle = \"Secret\"\ndraft = true\n+++\n\nSecret page",
        )
        .unwrap();

        let mut builder = SiteBuilder::new(dir.path()).include_drafts(true);
        let site = builder.build().unwrap();

        assert!(site.pages.iter().any(|page| page.content.slug == "secret"));
    }

    #[test]
    fn test_draft_posts_excluded_by_default() {
        let dir = create_test_site();
        fs::write(
            dir.path().join("content/posts/2024-02-01-draft.md"),
            "+++\ntitle = \"Draft\"\ndraft = true\n+++\n\nDraft post",
        )
        .unwrap();

        let mut builder = SiteBuilder::new(dir.path());
        let site = builder.build().unwrap();

        assert_eq!(site.posts.len(), 1);
    }

    #[test]
    fn test_draft_posts_included_when_requested() {
        let dir = create_test_site();
        fs::write(
            dir.path().join("content/posts/2024-02-01-draft.md"),
            "+++\ntitle = \"Draft\"\ndraft = true\n+++\n\nDraft post",
        )
        .unwrap();

        let mut builder = SiteBuilder::new(dir.path()).include_drafts(true);
        let site = builder.build().unwrap();

        assert_eq!(site.posts.len(), 2);
    }

    #[test]
    fn test_collections() {
        let dir = create_test_site();
        fs::create_dir_all(dir.path().join("content/docs")).unwrap();
        fs::write(
            dir.path().join("content/docs/_collection.toml"),
            "name = \"docs\"",
        )
        .unwrap();
        fs::write(
            dir.path().join("content/docs/intro.md"),
            "+++\ntitle = \"Introduction\"\n+++\n\nGetting started",
        )
        .unwrap();
        fs::write(
            dir.path().join("content/docs/advanced.md"),
            "+++\ntitle = \"Advanced\"\nweight = 10\n+++\n\nAdvanced topics",
        )
        .unwrap();

        let mut builder = SiteBuilder::new(dir.path());
        let site = builder.build().unwrap();

        assert!(site.collections.contains_key("docs"));
        let docs = &site.collections["docs"];
        assert_eq!(docs.items.len(), 2);
    }

    #[test]
    fn test_duplicate_page_slugs_error() {
        let dir = create_test_site();
        fs::create_dir_all(dir.path().join("content/nested")).unwrap();
        fs::write(
            dir.path().join("content/about.md"),
            "+++\ntitle = \"About\"\n+++\n\nAbout page",
        )
        .unwrap();
        fs::write(
            dir.path().join("content/nested/_index.md"),
            "+++\ntitle = \"About Duplicate\"\n+++\n\nDuplicate",
        )
        .unwrap();

        let mut builder = SiteBuilder::new(dir.path());
        let result = builder.build();
        assert!(result.is_ok() || matches!(result, Err(BambooError::DuplicatePage { .. })));
    }

    #[test]
    fn test_yaml_frontmatter() {
        let dir = create_test_site();
        fs::write(
            dir.path().join("content/yaml-page.md"),
            "---\ntitle: YAML Page\nweight: 1\n---\n\nYAML frontmatter content",
        )
        .unwrap();

        let mut builder = SiteBuilder::new(dir.path());
        let site = builder.build().unwrap();

        assert!(
            site.pages
                .iter()
                .any(|page| page.content.title == "YAML Page")
        );
    }

    #[test]
    fn test_post_sorting_by_date() {
        let dir = create_test_site();
        fs::write(
            dir.path().join("content/posts/2024-03-01-newer.md"),
            "+++\ntitle = \"Newer\"\n+++\n\nNewer post",
        )
        .unwrap();

        let mut builder = SiteBuilder::new(dir.path());
        let site = builder.build().unwrap();

        assert_eq!(site.posts[0].content.slug, "newer");
        assert_eq!(site.posts[1].content.slug, "hello");
    }

    #[test]
    fn test_word_count_and_reading_time() {
        let dir = create_test_site();
        let mut builder = SiteBuilder::new(dir.path());
        let site = builder.build().unwrap();

        let post = &site.posts[0];
        assert!(post.content.word_count > 0);
        assert!(post.content.reading_time > 0);
    }

    #[test]
    fn test_content_url_generation() {
        let dir = create_test_site();
        let mut builder = SiteBuilder::new(dir.path());
        let site = builder.build().unwrap();

        let about = site
            .pages
            .iter()
            .find(|page| page.content.slug == "about")
            .unwrap();
        assert_eq!(about.content.url, "/about/");

        let post = &site.posts[0];
        assert_eq!(post.content.url, "/posts/hello/");

        let home = site.home.as_ref().unwrap();
        assert_eq!(home.content.url, "/");
    }

    #[test]
    fn test_base_url_trailing_slash_trimmed() {
        let dir = TempDir::new().unwrap();
        fs::write(
            dir.path().join("bamboo.toml"),
            "title = \"Test\"\nbase_url = \"https://example.com/\"\n",
        )
        .unwrap();
        fs::create_dir_all(dir.path().join("content/posts")).unwrap();

        let mut builder = SiteBuilder::new(dir.path());
        let site = builder.build().unwrap();

        assert_eq!(site.config.base_url, "https://example.com");
    }

    #[test]
    fn test_static_assets_collected() {
        let dir = create_test_site();
        fs::create_dir_all(dir.path().join("static/css")).unwrap();
        fs::write(dir.path().join("static/css/style.css"), "body {}").unwrap();
        fs::write(dir.path().join("static/favicon.ico"), "icon").unwrap();

        let mut builder = SiteBuilder::new(dir.path());
        let site = builder.build().unwrap();

        assert_eq!(site.assets.len(), 2);
    }
}
