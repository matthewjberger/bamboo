use crate::error::{BambooError, Result};
use crate::types::{Asset, Site};
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
const DEFAULT_HEADER_PARTIAL: &str =
    include_str!("../themes/default/templates/partials/header.html");
const DEFAULT_FOOTER_PARTIAL: &str =
    include_str!("../themes/default/templates/partials/footer.html");
const DEFAULT_NAV_PARTIAL: &str = include_str!("../themes/default/templates/partials/nav.html");

pub struct ThemeEngine {
    tera: Tera,
    theme_static_dir: Option<PathBuf>,
}

impl ThemeEngine {
    pub fn new(theme: &str) -> Result<Self> {
        let theme_path = Path::new(theme);

        if theme_path.exists() && theme_path.is_dir() {
            Self::from_directory(theme_path)
        } else if theme == "default" {
            Self::builtin_default()
        } else {
            Err(BambooError::ThemeNotFound {
                name: theme.to_string(),
            })
        }
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
        tera.add_raw_template("partials/header.html", DEFAULT_HEADER_PARTIAL)?;
        tera.add_raw_template("partials/footer.html", DEFAULT_FOOTER_PARTIAL)?;
        tera.add_raw_template("partials/nav.html", DEFAULT_NAV_PARTIAL)?;

        Ok(Self {
            tera,
            theme_static_dir: None,
        })
    }

    fn from_directory(theme_dir: &Path) -> Result<Self> {
        let templates_dir = theme_dir.join("templates");
        let static_dir = theme_dir.join("static");

        let pattern = templates_dir.join("**").join("*.html");
        let pattern_str = pattern.to_string_lossy();

        let tera = Tera::new(&pattern_str)?;

        let theme_static_dir = if static_dir.exists() {
            Some(static_dir)
        } else {
            None
        };

        Ok(Self {
            tera,
            theme_static_dir,
        })
    }

    pub fn render_site(&self, site: &Site, output_dir: &Path) -> Result<()> {
        fs::create_dir_all(output_dir)?;

        self.render_index(site, output_dir)?;

        for page in &site.pages {
            self.render_page(site, page, output_dir)?;
        }

        for post in &site.posts {
            self.render_post(site, post, output_dir)?;
        }

        for (name, collection) in &site.collections {
            self.render_collection(site, name, collection, output_dir)?;
        }

        self.copy_assets(&site.assets, output_dir)?;
        self.copy_theme_static(output_dir)?;

        self.generate_rss(site, output_dir)?;
        self.generate_sitemap(site, output_dir)?;

        Ok(())
    }

    fn render_index(&self, site: &Site, output_dir: &Path) -> Result<()> {
        let mut context = Context::new();
        context.insert("site", site);

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
        context.insert("site", site);
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

    fn render_post(&self, site: &Site, post: &crate::types::Post, output_dir: &Path) -> Result<()> {
        let mut context = Context::new();
        context.insert("site", site);
        context.insert("post", post);

        let template_name = post.template.as_deref().unwrap_or("post.html");
        let rendered = self.tera.render(template_name, &context)?;

        let output_path = output_dir.join(&post.path);
        if let Some(parent) = output_path.parent() {
            fs::create_dir_all(parent)?;
        }

        fs::write(output_path, rendered)?;

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
        context.insert("site", site);
        context.insert("collection", collection);
        context.insert("collection_name", name);

        let index_rendered = self.tera.render("collection.html", &context)?;
        let index_path = output_dir.join(name).join("index.html");
        if let Some(parent) = index_path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(index_path, index_rendered)?;

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
        context.insert("site", site);
        context.insert("item", item);
        context.insert("collection", collection);
        context.insert("collection_name", collection_name);

        let template_name = item.template.as_deref().unwrap_or("collection_item.html");

        let template_name = if self.tera.get_template_names().any(|n| n == template_name) {
            template_name
        } else {
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
        if let Some(static_dir) = &self.theme_static_dir {
            for entry in WalkDir::new(static_dir)
                .min_depth(1)
                .into_iter()
                .filter_map(|e| e.ok())
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

    fn generate_rss(&self, site: &Site, output_dir: &Path) -> Result<()> {
        let base_url = site.config.base_url.trim_end_matches('/');
        let language = site.config.language.as_deref().unwrap_or("en");

        let mut items = String::new();
        for post in &site.posts {
            let post_url = format!("{}/posts/{}/", base_url, post.slug);
            let pub_date = post.date.format("%a, %d %b %Y %H:%M:%S +0000").to_string();
            let description = post
                .excerpt
                .as_deref()
                .unwrap_or("")
                .replace('&', "&amp;")
                .replace('<', "&lt;")
                .replace('>', "&gt;");

            items.push_str(&format!(
                r#"    <item>
      <title>{}</title>
      <link>{}</link>
      <guid>{}</guid>
      <pubDate>{}</pubDate>
      <description>{}</description>
    </item>
"#,
                escape_xml(&post.title),
                post_url,
                post_url,
                pub_date,
                description
            ));
        }

        let rss = format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<rss version="2.0" xmlns:atom="http://www.w3.org/2005/Atom">
  <channel>
    <title>{}</title>
    <link>{}</link>
    <description>{}</description>
    <language>{}</language>
    <atom:link href="{}/rss.xml" rel="self" type="application/rss+xml"/>
{}  </channel>
</rss>
"#,
            escape_xml(&site.config.title),
            base_url,
            escape_xml(site.config.description.as_deref().unwrap_or("")),
            language,
            base_url,
            items
        );

        fs::write(output_dir.join("rss.xml"), rss)?;

        Ok(())
    }

    fn generate_sitemap(&self, site: &Site, output_dir: &Path) -> Result<()> {
        let base_url = site.config.base_url.trim_end_matches('/');

        let mut urls = String::new();

        urls.push_str(&format!(
            "  <url>\n    <loc>{}/</loc>\n  </url>\n",
            base_url
        ));

        for page in &site.pages {
            urls.push_str(&format!(
                "  <url>\n    <loc>{}/{}/</loc>\n  </url>\n",
                base_url, page.slug
            ));
        }

        for post in &site.posts {
            let lastmod = post.date.format("%Y-%m-%d").to_string();
            urls.push_str(&format!(
                "  <url>\n    <loc>{}/posts/{}/</loc>\n    <lastmod>{}</lastmod>\n  </url>\n",
                base_url, post.slug, lastmod
            ));
        }

        for (name, collection) in &site.collections {
            urls.push_str(&format!(
                "  <url>\n    <loc>{}/{}/</loc>\n  </url>\n",
                base_url, name
            ));

            for item in &collection.items {
                urls.push_str(&format!(
                    "  <url>\n    <loc>{}/{}/{}/</loc>\n  </url>\n",
                    base_url, name, item.slug
                ));
            }
        }

        let sitemap = format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">
{}
</urlset>
"#,
            urls
        );

        fs::write(output_dir.join("sitemap.xml"), sitemap)?;

        Ok(())
    }
}

fn escape_xml(input: &str) -> String {
    input
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

pub fn clean_output_dir(output_dir: &Path) -> Result<()> {
    if output_dir.exists() {
        fs::remove_dir_all(output_dir)?;
    }
    Ok(())
}
