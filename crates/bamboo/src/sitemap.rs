use crate::error::Result;
use crate::parsing::slugify;
use crate::types::Site;
use crate::xml::escape;
use std::fs;
use std::path::Path;

pub fn generate_sitemap(site: &Site, output_dir: &Path) -> Result<()> {
    let base_url = site.config.base_url.trim_end_matches('/');
    let escaped_base_url = escape(base_url);

    let mut urls = String::new();

    urls.push_str(&format!(
        "  <url>\n    <loc>{}/</loc>\n  </url>\n",
        escaped_base_url
    ));

    for page in &site.pages {
        if page.slug == "404" {
            continue;
        }
        urls.push_str(&format!(
            "  <url>\n    <loc>{}/{}/</loc>\n  </url>\n",
            escaped_base_url,
            escape(&page.slug)
        ));
    }

    for post in &site.posts {
        let lastmod = post.date.format("%Y-%m-%d").to_string();
        urls.push_str(&format!(
            "  <url>\n    <loc>{}/posts/{}/</loc>\n    <lastmod>{}</lastmod>\n  </url>\n",
            escaped_base_url,
            escape(&post.slug),
            lastmod
        ));
    }

    let posts_per_page = site.config.posts_per_page;
    if posts_per_page > 0 && !site.posts.is_empty() {
        let total_pages = site.posts.len().div_ceil(posts_per_page);
        for page_number in 2..=total_pages {
            urls.push_str(&format!(
                "  <url>\n    <loc>{}/page/{}/</loc>\n  </url>\n",
                escaped_base_url, page_number
            ));
        }
    }

    let mut sorted_collections: Vec<(&String, &crate::types::Collection)> =
        site.collections.iter().collect();
    sorted_collections.sort_by_key(|(name, _)| name.as_str());
    for (name, collection) in sorted_collections {
        urls.push_str(&format!(
            "  <url>\n    <loc>{}/{}/</loc>\n  </url>\n",
            escaped_base_url,
            escape(name)
        ));

        for item in &collection.items {
            urls.push_str(&format!(
                "  <url>\n    <loc>{}/{}/{}/</loc>\n  </url>\n",
                escaped_base_url,
                escape(name),
                escape(&item.slug)
            ));
        }
    }

    use std::collections::HashMap;
    let mut tag_counts: HashMap<String, usize> = HashMap::new();
    for post in &site.posts {
        for tag in &post.tags {
            let slug = slugify(tag);
            *tag_counts.entry(slug).or_default() += 1;
        }
    }
    if !tag_counts.is_empty() {
        urls.push_str(&format!(
            "  <url>\n    <loc>{}/tags/</loc>\n  </url>\n",
            escaped_base_url
        ));
        let mut sorted_tags: Vec<(&String, &usize)> = tag_counts.iter().collect();
        sorted_tags.sort_by_key(|(slug, _)| slug.as_str());
        for (slug, count) in sorted_tags {
            urls.push_str(&format!(
                "  <url>\n    <loc>{}/tags/{}/</loc>\n  </url>\n",
                escaped_base_url,
                escape(slug)
            ));
            if posts_per_page > 0 {
                let total_pages = count.div_ceil(posts_per_page);
                for page_number in 2..=total_pages {
                    urls.push_str(&format!(
                        "  <url>\n    <loc>{}/tags/{}/page/{}/</loc>\n  </url>\n",
                        escaped_base_url,
                        escape(slug),
                        page_number
                    ));
                }
            }
        }
    }

    let mut category_counts: HashMap<String, usize> = HashMap::new();
    for post in &site.posts {
        for category in &post.categories {
            let slug = slugify(category);
            *category_counts.entry(slug).or_default() += 1;
        }
    }
    if !category_counts.is_empty() {
        urls.push_str(&format!(
            "  <url>\n    <loc>{}/categories/</loc>\n  </url>\n",
            escaped_base_url
        ));
        let mut sorted_categories: Vec<(&String, &usize)> = category_counts.iter().collect();
        sorted_categories.sort_by_key(|(slug, _)| slug.as_str());
        for (slug, count) in sorted_categories {
            urls.push_str(&format!(
                "  <url>\n    <loc>{}/categories/{}/</loc>\n  </url>\n",
                escaped_base_url,
                escape(slug)
            ));
            if posts_per_page > 0 {
                let total_pages = count.div_ceil(posts_per_page);
                for page_number in 2..=total_pages {
                    urls.push_str(&format!(
                        "  <url>\n    <loc>{}/categories/{}/page/{}/</loc>\n  </url>\n",
                        escaped_base_url,
                        escape(slug),
                        page_number
                    ));
                }
            }
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
