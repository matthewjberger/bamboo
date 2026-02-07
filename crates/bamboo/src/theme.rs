use crate::assets::AssetConfig;
use crate::error::Result;
use crate::feeds;
use crate::images;
use crate::parsing::slugify;
use crate::redirects;
use crate::search;
use crate::sitemap;
use crate::types::{Asset, Site};
use serde::Serialize;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use tera::{Context, Tera};
use walkdir::WalkDir;

const DEFAULT_BASE_TEMPLATE: &str = include_str!("../themes/default/templates/base.html");
const DEFAULT_INDEX_TEMPLATE: &str = include_str!("../themes/default/templates/index.html");
const DEFAULT_PAGE_TEMPLATE: &str = include_str!("../themes/default/templates/page.html");
const DEFAULT_POST_TEMPLATE: &str = include_str!("../themes/default/templates/post.html");
const DEFAULT_COLLECTION_TEMPLATE: &str =
    include_str!("../themes/default/templates/collection.html");
const DEFAULT_COLLECTION_ITEM_TEMPLATE: &str =
    include_str!("../themes/default/templates/collection_item.html");
const DEFAULT_SLIDESHOW_TEMPLATE: &str = include_str!("../themes/default/templates/slideshow.html");
const DEFAULT_DOCS_TEMPLATE: &str = include_str!("../themes/default/templates/docs.html");
const DEFAULT_PORTFOLIO_TEMPLATE: &str = include_str!("../themes/default/templates/portfolio.html");
const DEFAULT_LANDING_TEMPLATE: &str = include_str!("../themes/default/templates/landing.html");
const DEFAULT_CHANGELOG_TEMPLATE: &str = include_str!("../themes/default/templates/changelog.html");
const DEFAULT_TAGS_TEMPLATE: &str = include_str!("../themes/default/templates/tags.html");
const DEFAULT_TAG_TEMPLATE: &str = include_str!("../themes/default/templates/tag.html");
const DEFAULT_CATEGORIES_TEMPLATE: &str =
    include_str!("../themes/default/templates/categories.html");
const DEFAULT_CATEGORY_TEMPLATE: &str = include_str!("../themes/default/templates/category.html");
const DEFAULT_PAGINATION_TEMPLATE: &str =
    include_str!("../themes/default/templates/pagination.html");
const DEFAULT_404_TEMPLATE: &str = include_str!("../themes/default/templates/404.html");
const DEFAULT_HEADER_PARTIAL: &str =
    include_str!("../themes/default/templates/partials/header.html");
const DEFAULT_FOOTER_PARTIAL: &str =
    include_str!("../themes/default/templates/partials/footer.html");
const DEFAULT_NAV_PARTIAL: &str = include_str!("../themes/default/templates/partials/nav.html");
const DEFAULT_SEARCH_TEMPLATE: &str = include_str!("../themes/default/templates/search.html");
const DEFAULT_STYLESHEET: &str = include_str!("../themes/default/static/style.css");

#[derive(Debug, Clone, Serialize)]
struct TagInfo {
    name: String,
    slug: String,
    count: usize,
}

#[derive(Debug, Clone, Serialize)]
struct CategoryInfo {
    name: String,
    slug: String,
    count: usize,
}

#[derive(Debug, Clone, Serialize)]
struct SiteMetadata<'a> {
    config: &'a crate::types::SiteConfig,
    pages: &'a [crate::types::Page],
    data: &'a HashMap<String, serde_json::Value>,
    collections: &'a HashMap<String, crate::types::Collection>,
}

pub struct ThemeEngine {
    tera: Tera,
    theme_static_dir: Option<PathBuf>,
    override_static_dir: Option<PathBuf>,
    is_builtin_default: bool,
}

impl ThemeEngine {
    pub fn new(theme: &str) -> Result<Self> {
        let theme_path = Path::new(theme);

        if theme_path.exists() && theme_path.is_dir() {
            Self::from_directory(theme_path)
        } else if theme == "default" {
            Self::builtin_default()
        } else {
            Err(crate::error::BambooError::ThemeNotFound {
                name: theme.to_string(),
            })
        }
    }

    pub fn new_with_overrides(theme: &str, override_dir: &Path) -> Result<Self> {
        let mut engine = Self::new(theme)?;
        engine.apply_overrides(override_dir)?;
        Ok(engine)
    }

    fn apply_overrides(&mut self, override_dir: &Path) -> Result<()> {
        let templates_dir = override_dir.join("templates");
        if !templates_dir.exists() {
            return Ok(());
        }

        for entry in WalkDir::new(&templates_dir)
            .min_depth(1)
            .into_iter()
            .filter_map(|entry| entry.ok())
        {
            let path = entry.path();
            if !path.is_file() {
                continue;
            }
            if path
                .extension()
                .map(|extension| extension != "html")
                .unwrap_or(true)
            {
                continue;
            }
            let relative = path.strip_prefix(&templates_dir).map_err(|_| {
                crate::error::BambooError::InvalidPath {
                    path: path.to_path_buf(),
                }
            })?;
            let template_name = relative.to_string_lossy().replace('\\', "/");
            let content = fs::read_to_string(path)?;
            self.tera.add_raw_template(&template_name, &content)?;
        }

        let static_dir = override_dir.join("static");
        if static_dir.exists() {
            self.override_static_dir = Some(static_dir);
        }

        Ok(())
    }

    fn builtin_default() -> Result<Self> {
        let mut tera = Tera::default();

        tera.add_raw_template("base.html", DEFAULT_BASE_TEMPLATE)?;
        tera.add_raw_template("index.html", DEFAULT_INDEX_TEMPLATE)?;
        tera.add_raw_template("page.html", DEFAULT_PAGE_TEMPLATE)?;
        tera.add_raw_template("post.html", DEFAULT_POST_TEMPLATE)?;
        tera.add_raw_template("collection.html", DEFAULT_COLLECTION_TEMPLATE)?;
        tera.add_raw_template("collection_item.html", DEFAULT_COLLECTION_ITEM_TEMPLATE)?;
        tera.add_raw_template("slideshow.html", DEFAULT_SLIDESHOW_TEMPLATE)?;
        tera.add_raw_template("docs.html", DEFAULT_DOCS_TEMPLATE)?;
        tera.add_raw_template("portfolio.html", DEFAULT_PORTFOLIO_TEMPLATE)?;
        tera.add_raw_template("landing.html", DEFAULT_LANDING_TEMPLATE)?;
        tera.add_raw_template("changelog.html", DEFAULT_CHANGELOG_TEMPLATE)?;
        tera.add_raw_template("tags.html", DEFAULT_TAGS_TEMPLATE)?;
        tera.add_raw_template("tag.html", DEFAULT_TAG_TEMPLATE)?;
        tera.add_raw_template("categories.html", DEFAULT_CATEGORIES_TEMPLATE)?;
        tera.add_raw_template("category.html", DEFAULT_CATEGORY_TEMPLATE)?;
        tera.add_raw_template("pagination.html", DEFAULT_PAGINATION_TEMPLATE)?;
        tera.add_raw_template("404.html", DEFAULT_404_TEMPLATE)?;
        tera.add_raw_template("partials/header.html", DEFAULT_HEADER_PARTIAL)?;
        tera.add_raw_template("partials/footer.html", DEFAULT_FOOTER_PARTIAL)?;
        tera.add_raw_template("partials/nav.html", DEFAULT_NAV_PARTIAL)?;
        tera.add_raw_template("search.html", DEFAULT_SEARCH_TEMPLATE)?;

        register_custom_filters(&mut tera);

        Ok(Self {
            tera,
            theme_static_dir: None,
            override_static_dir: None,
            is_builtin_default: true,
        })
    }

    fn from_directory(theme_dir: &Path) -> Result<Self> {
        let templates_dir = theme_dir.join("templates");
        let static_dir = theme_dir.join("static");

        let escaped_templates =
            escape_glob_path(&templates_dir.to_string_lossy().replace('\\', "/"));
        let pattern_str = format!("{escaped_templates}/**/*.html");

        let mut tera = Tera::new(&pattern_str)?;
        register_custom_filters(&mut tera);

        let theme_static_dir = if static_dir.exists() {
            Some(static_dir)
        } else {
            None
        };

        Ok(Self {
            tera,
            theme_static_dir,
            override_static_dir: None,
            is_builtin_default: false,
        })
    }

    fn site_metadata<'a>(&self, site: &'a Site) -> SiteMetadata<'a> {
        SiteMetadata {
            config: &site.config,
            pages: &site.pages,
            data: &site.data,
            collections: &site.collections,
        }
    }

    pub fn render_site(&self, site: &Site, output_dir: &Path) -> Result<()> {
        fs::create_dir_all(output_dir)?;

        if self.is_builtin_default {
            fs::write(output_dir.join("style.css"), DEFAULT_STYLESHEET)?;
        }

        self.render_index(site, output_dir)?;

        for page in &site.pages {
            if page.slug == "404" {
                continue;
            }
            self.render_page(site, page, output_dir)?;
        }

        for (index, post) in site.posts.iter().enumerate() {
            let prev_post = if index + 1 < site.posts.len() {
                Some(&site.posts[index + 1])
            } else {
                None
            };
            let next_post = if index > 0 {
                Some(&site.posts[index - 1])
            } else {
                None
            };
            self.render_post(site, post, prev_post, next_post, output_dir)?;
        }

        for (name, collection) in &site.collections {
            self.render_collection(site, name, collection, output_dir)?;
        }

        self.render_pagination(site, output_dir)?;
        self.render_tag_pages(site, output_dir)?;
        self.render_category_pages(site, output_dir)?;
        self.render_404(site, output_dir)?;
        self.render_search(site, output_dir)?;

        self.copy_theme_static(output_dir)?;
        self.copy_assets(&site.assets, output_dir)?;

        feeds::generate_rss(site, output_dir)?;
        feeds::generate_atom(site, output_dir)?;
        sitemap::generate_sitemap(site, output_dir)?;
        redirects::generate_redirects(site, output_dir)?;
        search::generate_search_index(site, output_dir)?;

        if let Some(ref image_config) = site.config.images {
            let manifest = images::process_images(output_dir, image_config)?;
            images::apply_srcset_to_html(output_dir, &manifest)?;
        }

        let asset_config = AssetConfig {
            minify: site.config.minify,
            fingerprint: site.config.fingerprint,
            base_url: site.config.base_url.clone(),
        };
        if asset_config.minify || asset_config.fingerprint {
            crate::assets::process_assets(output_dir, &asset_config)?;
        }

        Ok(())
    }

    fn render_index(&self, site: &Site, output_dir: &Path) -> Result<()> {
        let posts_per_page = site.config.posts_per_page;
        let index_posts: Vec<&crate::types::Post> =
            site.posts.iter().take(posts_per_page).collect();
        let total_pages = if posts_per_page > 0 && !site.posts.is_empty() {
            site.posts.len().div_ceil(posts_per_page)
        } else {
            1
        };
        let base_url = site.config.base_url.trim_end_matches('/');

        let mut context = Context::new();
        let metadata = self.site_metadata(site);
        context.insert("site", &metadata);
        context.insert("posts", &index_posts);
        context.insert("current_page", &1usize);
        context.insert("total_pages", &total_pages);

        if total_pages > 1 {
            let next_url = format!("{}/page/2/", base_url);
            context.insert("next_page_url", &next_url);
        }

        let template_name = if let Some(home) = &site.home {
            context.insert("home", home);
            context.insert("page", home);
            home.template.as_deref().unwrap_or("index.html")
        } else {
            "index.html"
        };

        let rendered = self.tera.render(template_name, &context)?;
        let output_path = output_dir.join("index.html");

        fs::write(output_path, rendered)?;

        Ok(())
    }

    fn render_page(&self, site: &Site, page: &crate::types::Page, output_dir: &Path) -> Result<()> {
        let mut context = Context::new();
        let metadata = self.site_metadata(site);
        context.insert("site", &metadata);
        context.insert("page", page);

        let template_name = page.template.as_deref().unwrap_or("page.html");
        let rendered = self.tera.render(template_name, &context)?;

        let output_path = output_dir.join(&page.path);
        if let Some(parent) = output_path.parent() {
            fs::create_dir_all(parent)?;
        }

        fs::write(output_path, rendered)?;

        Ok(())
    }

    fn render_post(
        &self,
        site: &Site,
        post: &crate::types::Post,
        prev_post: Option<&crate::types::Post>,
        next_post: Option<&crate::types::Post>,
        output_dir: &Path,
    ) -> Result<()> {
        let mut context = Context::new();
        let metadata = self.site_metadata(site);
        context.insert("site", &metadata);
        context.insert("post", post);

        if let Some(prev) = prev_post {
            context.insert("prev_post", prev);
        }
        if let Some(next) = next_post {
            context.insert("next_post", next);
        }

        let template_name = post.template.as_deref().unwrap_or("post.html");
        let rendered = self.tera.render(template_name, &context)?;

        let output_path = output_dir.join(&post.path);
        if let Some(parent) = output_path.parent() {
            fs::create_dir_all(parent)?;
        }

        fs::write(output_path, rendered)?;

        Ok(())
    }

    fn render_pagination(&self, site: &Site, output_dir: &Path) -> Result<()> {
        let posts_per_page = site.config.posts_per_page;
        if posts_per_page == 0 || site.posts.is_empty() {
            return Ok(());
        }

        let total_pages = site.posts.len().div_ceil(posts_per_page);
        let base_url = site.config.base_url.trim_end_matches('/');
        let metadata = self.site_metadata(site);

        for page_number in 2..=total_pages {
            let start = (page_number - 1) * posts_per_page;
            let end = (start + posts_per_page).min(site.posts.len());
            let page_posts = &site.posts[start..end];

            let mut context = Context::new();
            context.insert("site", &metadata);
            context.insert("posts", page_posts);
            context.insert("current_page", &page_number);
            context.insert("total_pages", &total_pages);

            let prev_url = if page_number == 2 {
                format!("{}/", base_url)
            } else {
                format!("{}/page/{}/", base_url, page_number - 1)
            };
            context.insert("prev_page_url", &prev_url);

            if page_number < total_pages {
                let next_url = format!("{}/page/{}/", base_url, page_number + 1);
                context.insert("next_page_url", &next_url);
            }

            let rendered = self.tera.render("pagination.html", &context)?;
            let page_dir = output_dir.join("page").join(page_number.to_string());
            fs::create_dir_all(&page_dir)?;
            fs::write(page_dir.join("index.html"), rendered)?;
        }

        Ok(())
    }

    fn render_tag_pages(&self, site: &Site, output_dir: &Path) -> Result<()> {
        let mut slug_posts: HashMap<String, Vec<&crate::types::Post>> = HashMap::new();
        let mut slug_display_name: HashMap<String, String> = HashMap::new();

        for post in &site.posts {
            for tag in &post.tags {
                let slug = slugify(tag);
                slug_posts.entry(slug.clone()).or_default().push(post);
                slug_display_name.entry(slug).or_insert_with(|| tag.clone());
            }
        }

        if slug_posts.is_empty() {
            return Ok(());
        }

        let mut tags: Vec<TagInfo> = slug_posts
            .iter()
            .map(|(slug, posts)| TagInfo {
                name: slug_display_name
                    .get(slug)
                    .cloned()
                    .unwrap_or_else(|| slug.clone()),
                slug: slug.clone(),
                count: posts.len(),
            })
            .collect();
        tags.sort_by(|a, b| a.name.cmp(&b.name));

        let metadata = self.site_metadata(site);

        let mut context = Context::new();
        context.insert("site", &metadata);
        context.insert("tags", &tags);

        let tags_dir = output_dir.join("tags");
        let tags_index = tags_dir.join("index.html");
        if !tags_index.exists() {
            let rendered = self.tera.render("tags.html", &context)?;
            fs::create_dir_all(&tags_dir)?;
            fs::write(tags_index, rendered)?;
        }

        let posts_per_page = site.config.posts_per_page;

        for (slug, posts) in &slug_posts {
            let tag_name = slug_display_name.get(slug.as_str()).unwrap_or(slug);
            let tag_dir = tags_dir.join(slug);
            let effective_per_page = if posts_per_page == 0 {
                posts.len().max(1)
            } else {
                posts_per_page
            };
            let total_pages = posts.len().div_ceil(effective_per_page);
            let base_url = site.config.base_url.trim_end_matches('/');

            for page_number in 1..=total_pages {
                let start = (page_number - 1) * effective_per_page;
                let end = (start + effective_per_page).min(posts.len());
                let page_posts = &posts[start..end];

                let mut context = Context::new();
                context.insert("site", &metadata);
                context.insert("tag_name", tag_name);
                context.insert("tag_slug", &slug);
                context.insert("posts", page_posts);
                context.insert("current_page", &page_number);
                context.insert("total_pages", &total_pages);

                if page_number > 1 {
                    let prev_url = if page_number == 2 {
                        format!("{}/tags/{}/", base_url, slug)
                    } else {
                        format!("{}/tags/{}/page/{}/", base_url, slug, page_number - 1)
                    };
                    context.insert("prev_page_url", &prev_url);
                }

                if page_number < total_pages {
                    let next_url = format!("{}/tags/{}/page/{}/", base_url, slug, page_number + 1);
                    context.insert("next_page_url", &next_url);
                }

                if page_number == 1 {
                    let rendered = self.tera.render("tag.html", &context)?;
                    fs::create_dir_all(&tag_dir)?;
                    fs::write(tag_dir.join("index.html"), rendered)?;
                } else {
                    let rendered = self.tera.render("tag.html", &context)?;
                    let page_dir = tag_dir.join("page").join(page_number.to_string());
                    fs::create_dir_all(&page_dir)?;
                    fs::write(page_dir.join("index.html"), rendered)?;
                }
            }
        }

        Ok(())
    }

    fn render_category_pages(&self, site: &Site, output_dir: &Path) -> Result<()> {
        let mut slug_posts: HashMap<String, Vec<&crate::types::Post>> = HashMap::new();
        let mut slug_display_name: HashMap<String, String> = HashMap::new();

        for post in &site.posts {
            for category in &post.categories {
                let slug = slugify(category);
                slug_posts.entry(slug.clone()).or_default().push(post);
                slug_display_name
                    .entry(slug)
                    .or_insert_with(|| category.clone());
            }
        }

        if slug_posts.is_empty() {
            return Ok(());
        }

        let mut categories: Vec<CategoryInfo> = slug_posts
            .iter()
            .map(|(slug, posts)| CategoryInfo {
                name: slug_display_name
                    .get(slug)
                    .cloned()
                    .unwrap_or_else(|| slug.clone()),
                slug: slug.clone(),
                count: posts.len(),
            })
            .collect();
        categories.sort_by(|a, b| a.name.cmp(&b.name));

        let metadata = self.site_metadata(site);

        let mut context = Context::new();
        context.insert("site", &metadata);
        context.insert("categories", &categories);

        let categories_dir = output_dir.join("categories");
        let categories_index = categories_dir.join("index.html");
        if !categories_index.exists() {
            let rendered = self.tera.render("categories.html", &context)?;
            fs::create_dir_all(&categories_dir)?;
            fs::write(categories_index, rendered)?;
        }

        let posts_per_page = site.config.posts_per_page;

        for (slug, posts) in &slug_posts {
            let category_name = slug_display_name.get(slug.as_str()).unwrap_or(slug);
            let category_dir = categories_dir.join(slug.as_str());
            let effective_per_page = if posts_per_page == 0 {
                posts.len().max(1)
            } else {
                posts_per_page
            };
            let total_pages = posts.len().div_ceil(effective_per_page);
            let base_url = site.config.base_url.trim_end_matches('/');

            for page_number in 1..=total_pages {
                let start = (page_number - 1) * effective_per_page;
                let end = (start + effective_per_page).min(posts.len());
                let page_posts = &posts[start..end];

                let mut context = Context::new();
                context.insert("site", &metadata);
                context.insert("category_name", category_name);
                context.insert("category_slug", &slug);
                context.insert("posts", page_posts);
                context.insert("current_page", &page_number);
                context.insert("total_pages", &total_pages);

                if page_number > 1 {
                    let prev_url = if page_number == 2 {
                        format!("{}/categories/{}/", base_url, slug)
                    } else {
                        format!("{}/categories/{}/page/{}/", base_url, slug, page_number - 1)
                    };
                    context.insert("prev_page_url", &prev_url);
                }

                if page_number < total_pages {
                    let next_url =
                        format!("{}/categories/{}/page/{}/", base_url, slug, page_number + 1);
                    context.insert("next_page_url", &next_url);
                }

                if page_number == 1 {
                    let rendered = self.tera.render("category.html", &context)?;
                    fs::create_dir_all(&category_dir)?;
                    fs::write(category_dir.join("index.html"), rendered)?;
                } else {
                    let rendered = self.tera.render("category.html", &context)?;
                    let page_dir = category_dir.join("page").join(page_number.to_string());
                    fs::create_dir_all(&page_dir)?;
                    fs::write(page_dir.join("index.html"), rendered)?;
                }
            }
        }

        Ok(())
    }

    fn render_404(&self, site: &Site, output_dir: &Path) -> Result<()> {
        let mut context = Context::new();
        let metadata = self.site_metadata(site);
        context.insert("site", &metadata);

        let four_oh_four_page = site.pages.iter().find(|page| page.slug == "404");
        if let Some(page) = four_oh_four_page {
            context.insert("page", page);
        }

        let rendered = self.tera.render("404.html", &context)?;
        fs::write(output_dir.join("404.html"), rendered)?;

        Ok(())
    }

    fn render_search(&self, site: &Site, output_dir: &Path) -> Result<()> {
        let search_dir = output_dir.join("search");
        let search_index = search_dir.join("index.html");
        if search_index.exists() {
            return Ok(());
        }

        let mut context = Context::new();
        let metadata = self.site_metadata(site);
        context.insert("site", &metadata);

        let rendered = self.tera.render("search.html", &context)?;
        fs::create_dir_all(&search_dir)?;
        fs::write(search_index, rendered)?;

        Ok(())
    }

    fn render_collection(
        &self,
        site: &Site,
        name: &str,
        collection: &crate::types::Collection,
        output_dir: &Path,
    ) -> Result<()> {
        let mut context = Context::new();
        let metadata = self.site_metadata(site);
        context.insert("site", &metadata);
        context.insert("collection", collection);
        context.insert("collection_name", name);

        let index_path = output_dir.join(name).join("index.html");
        if let Some(parent) = index_path.parent() {
            fs::create_dir_all(parent)?;
        }
        if !index_path.exists() {
            let index_rendered = self.tera.render("collection.html", &context)?;
            fs::write(index_path, index_rendered)?;
        }

        for item in &collection.items {
            self.render_collection_item(site, name, collection, item, output_dir)?;
        }

        Ok(())
    }

    fn render_collection_item(
        &self,
        site: &Site,
        collection_name: &str,
        collection: &crate::types::Collection,
        item: &crate::types::CollectionItem,
        output_dir: &Path,
    ) -> Result<()> {
        let mut context = Context::new();
        let metadata = self.site_metadata(site);
        context.insert("site", &metadata);
        context.insert("item", item);
        context.insert("collection", collection);
        context.insert("collection_name", collection_name);

        let template_name = item.template.as_deref().unwrap_or("collection_item.html");

        let template_name = if self
            .tera
            .get_template_names()
            .any(|name| name == template_name)
        {
            template_name
        } else {
            context.insert("page", item);
            "page.html"
        };

        let rendered = self.tera.render(template_name, &context)?;
        let output_path = output_dir.join(&item.path);
        if let Some(parent) = output_path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(output_path, rendered)?;

        Ok(())
    }

    fn copy_assets(&self, assets: &[Asset], output_dir: &Path) -> Result<()> {
        for asset in assets {
            let dest = output_dir.join(&asset.dest);
            if let Some(parent) = dest.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::copy(&asset.source, &dest)?;
        }

        Ok(())
    }

    fn copy_theme_static(&self, output_dir: &Path) -> Result<()> {
        self.copy_static_dir(&self.theme_static_dir, output_dir)?;
        self.copy_static_dir(&self.override_static_dir, output_dir)?;
        Ok(())
    }

    fn copy_static_dir(&self, static_dir: &Option<PathBuf>, output_dir: &Path) -> Result<()> {
        if let Some(static_dir) = static_dir {
            for entry in WalkDir::new(static_dir)
                .min_depth(1)
                .into_iter()
                .filter_map(|entry| entry.ok())
            {
                let path = entry.path();

                if !path.is_file() {
                    continue;
                }

                let relative = path.strip_prefix(static_dir).unwrap();
                let dest = output_dir.join(relative);

                if let Some(parent) = dest.parent() {
                    fs::create_dir_all(parent)?;
                }

                fs::copy(path, &dest)?;
            }
        }

        Ok(())
    }
}

fn register_custom_filters(tera: &mut Tera) {
    tera.register_filter(
        "reading_time",
        |value: &tera::Value, _args: &HashMap<String, tera::Value>| {
            let raw_text = value.as_str().unwrap_or("");
            let plain_text = crate::search::strip_html_tags(raw_text);
            let words = plain_text.split_whitespace().count();
            let minutes = words / 200;
            let result = if minutes == 0 && words > 0 {
                1
            } else {
                minutes
            };
            Ok(tera::Value::Number(serde_json::Number::from(result)))
        },
    );

    tera.register_filter(
        "word_count",
        |value: &tera::Value, _args: &HashMap<String, tera::Value>| {
            let raw_text = value.as_str().unwrap_or("");
            let plain_text = crate::search::strip_html_tags(raw_text);
            let count = plain_text.split_whitespace().count();
            Ok(tera::Value::Number(serde_json::Number::from(count)))
        },
    );

    tera.register_filter(
        "toc",
        |value: &tera::Value, _args: &HashMap<String, tera::Value>| {
            let empty = Vec::new();
            let entries = value.as_array().unwrap_or(&empty);
            let mut html = String::from("<ul class=\"toc\">\n");
            for entry in entries {
                let level = entry.get("level").and_then(|v| v.as_u64()).unwrap_or(1);
                let id = entry
                    .get("id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let title = entry
                    .get("title")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let indent = "  ".repeat(level as usize);
                let escaped_title = crate::xml::escape(title);
                let escaped_id = crate::xml::escape(id);
                html.push_str(&format!(
                    "{indent}<li class=\"toc-level-{level}\"><a href=\"#{escaped_id}\">{escaped_title}</a></li>\n"
                ));
            }
            html.push_str("</ul>");
            Ok(tera::Value::String(html))
        },
    );

    tera.register_filter(
        "slugify",
        |value: &tera::Value, _args: &HashMap<String, tera::Value>| {
            let text = value.as_str().unwrap_or("");
            Ok(tera::Value::String(slugify(text)))
        },
    );
}

fn escape_glob_path(path: &str) -> String {
    let mut escaped = String::with_capacity(path.len());
    for character in path.chars() {
        match character {
            '[' | ']' | '{' | '}' | '*' | '?' => {
                escaped.push('[');
                escaped.push(character);
                escaped.push(']');
            }
            _ => escaped.push(character),
        }
    }
    escaped
}

fn is_filesystem_root(path: &Path) -> bool {
    if path.parent().is_none() {
        return true;
    }
    if path.parent() == Some(Path::new("")) {
        return true;
    }
    let path_str = path.to_string_lossy();
    let stripped = path_str.trim_start_matches("\\\\?\\");
    if stripped.len() <= 3 && stripped.ends_with('\\') && stripped.chars().nth(1) == Some(':') {
        return true;
    }
    if stripped.starts_with("\\\\") || stripped.starts_with("UNC\\") {
        let without_prefix = if let Some(rest) = stripped.strip_prefix("UNC\\") {
            rest
        } else if let Some(rest) = stripped.strip_prefix("\\\\") {
            rest
        } else {
            stripped
        };
        let slash_count = without_prefix.matches('\\').count();
        if slash_count <= 1 {
            return true;
        }
    }
    false
}

fn is_direct_child_of_root(path: &Path) -> bool {
    if let Some(parent) = path.parent() {
        is_filesystem_root(parent)
    } else {
        true
    }
}

pub fn clean_output_dir(output_dir: &Path) -> Result<()> {
    if output_dir.exists() {
        let canonical =
            output_dir
                .canonicalize()
                .map_err(|_| crate::error::BambooError::InvalidPath {
                    path: output_dir.to_path_buf(),
                })?;
        if is_filesystem_root(&canonical) {
            return Err(crate::error::BambooError::InvalidPath {
                path: output_dir.to_path_buf(),
            });
        }
        if is_direct_child_of_root(&canonical) {
            return Err(crate::error::BambooError::InvalidPath {
                path: output_dir.to_path_buf(),
            });
        }
        if let Some(home) = dirs_home()
            && canonical == home.canonicalize().unwrap_or(home)
        {
            return Err(crate::error::BambooError::InvalidPath {
                path: output_dir.to_path_buf(),
            });
        }
        if canonical.join("bamboo.toml").exists() {
            return Err(crate::error::BambooError::InvalidPath {
                path: output_dir.to_path_buf(),
            });
        }
        if let Ok(current_dir) = std::env::current_dir()
            && let Ok(canonical_current) = current_dir.canonicalize()
            && canonical == canonical_current
        {
            return Err(crate::error::BambooError::InvalidPath {
                path: output_dir.to_path_buf(),
            });
        }
        fs::remove_dir_all(output_dir)?;
    }
    Ok(())
}

fn dirs_home() -> Option<PathBuf> {
    std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .ok()
        .map(PathBuf::from)
}
