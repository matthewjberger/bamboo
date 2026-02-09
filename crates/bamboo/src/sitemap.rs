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
        if page.content.slug == "404" {
            continue;
        }
        urls.push_str(&format!(
            "  <url>\n    <loc>{}/{}/</loc>\n  </url>\n",
            escaped_base_url,
            escape(&page.content.slug)
        ));
    }

    for post in &site.posts {
        let lastmod = post.date.format("%Y-%m-%d").to_string();
        urls.push_str(&format!(
            "  <url>\n    <loc>{}/posts/{}/</loc>\n    <lastmod>{}</lastmod>\n  </url>\n",
            escaped_base_url,
            escape(&post.content.slug),
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
                escape(&item.content.slug)
            ));
        }
    }

    let mut sorted_taxonomy_names: Vec<&String> = site.config.taxonomies.keys().collect();
    sorted_taxonomy_names.sort();

    for taxonomy_name in sorted_taxonomy_names {
        use std::collections::HashMap as TaxonomyCountMap;
        let mut term_counts: TaxonomyCountMap<String, usize> = TaxonomyCountMap::new();
        for post in &site.posts {
            if let Some(terms) = post.taxonomies_map.get(taxonomy_name) {
                for term in terms {
                    let slug = slugify(term);
                    *term_counts.entry(slug).or_default() += 1;
                }
            }
        }
        if !term_counts.is_empty() {
            urls.push_str(&format!(
                "  <url>\n    <loc>{}/{}/</loc>\n  </url>\n",
                escaped_base_url,
                escape(taxonomy_name)
            ));
            let mut sorted_terms: Vec<(&String, &usize)> = term_counts.iter().collect();
            sorted_terms.sort_by_key(|(slug, _)| slug.as_str());
            for (slug, count) in sorted_terms {
                urls.push_str(&format!(
                    "  <url>\n    <loc>{}/{}/{}/</loc>\n  </url>\n",
                    escaped_base_url,
                    escape(taxonomy_name),
                    escape(slug)
                ));
                if posts_per_page > 0 {
                    let total_pages = count.div_ceil(posts_per_page);
                    for page_number in 2..=total_pages {
                        urls.push_str(&format!(
                            "  <url>\n    <loc>{}/{}/{}/page/{}/</loc>\n  </url>\n",
                            escaped_base_url,
                            escape(taxonomy_name),
                            escape(slug),
                            page_number
                        ));
                    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::*;
    use chrono::{NaiveDate, NaiveTime, TimeZone, Utc};
    use std::collections::HashMap;
    use std::path::PathBuf;

    fn minimal_site() -> Site {
        Site {
            config: SiteConfig {
                title: "Test".to_string(),
                base_url: "https://example.com".to_string(),
                description: None,
                author: None,
                language: None,
                posts_per_page: 10,
                minify: false,
                fingerprint: false,
                images: None,
                syntax_theme: crate::types::default_syntax_theme(),
                taxonomies: crate::types::default_taxonomies(),
                math: false,
                extra: HashMap::new(),
            },
            home: None,
            pages: vec![],
            posts: vec![],
            collections: HashMap::new(),
            data: HashMap::new(),
            assets: vec![],
        }
    }

    fn make_post(slug: &str, tags: Vec<&str>, categories: Vec<&str>) -> Post {
        let date = Utc.from_utc_datetime(
            &NaiveDate::from_ymd_opt(2024, 1, 1)
                .unwrap()
                .and_time(NaiveTime::MIN),
        );
        Post {
            content: Content {
                slug: slug.to_string(),
                title: slug.to_string(),
                html: String::new(),
                raw_content: String::new(),
                frontmatter: Frontmatter::default(),
                path: PathBuf::from(format!("posts/{}/index.html", slug)),
                template: None,
                weight: 0,
                word_count: 0,
                reading_time: 0,
                toc: vec![],
                url: format!("/posts/{}/", slug),
            },
            date,
            excerpt: None,
            draft: false,
            tags: tags.iter().map(|tag| String::from(*tag)).collect(),
            categories: categories
                .iter()
                .map(|category| String::from(*category))
                .collect(),
            taxonomies_map: {
                let mut map = std::collections::HashMap::new();
                let tag_vec: Vec<String> = tags.iter().map(|tag| String::from(*tag)).collect();
                let cat_vec: Vec<String> = categories
                    .iter()
                    .map(|category| String::from(*category))
                    .collect();
                if !tag_vec.is_empty() {
                    map.insert("tags".to_string(), tag_vec);
                }
                if !cat_vec.is_empty() {
                    map.insert("categories".to_string(), cat_vec);
                }
                map
            },
            redirect_from: vec![],
        }
    }

    #[test]
    fn test_sitemap_basic_urls() {
        let mut site = minimal_site();
        site.pages.push(Page {
            content: Content {
                slug: "about".to_string(),
                title: "About".to_string(),
                html: String::new(),
                raw_content: String::new(),
                frontmatter: Frontmatter::default(),
                path: PathBuf::from("about/index.html"),
                template: None,
                weight: 0,
                word_count: 0,
                reading_time: 0,
                toc: vec![],
                url: "/about/".to_string(),
            },
            draft: false,
            redirect_from: vec![],
        });

        let output_dir = tempfile::TempDir::new().unwrap();
        generate_sitemap(&site, output_dir.path()).unwrap();

        let content = std::fs::read_to_string(output_dir.path().join("sitemap.xml")).unwrap();
        assert!(content.contains("https://example.com/"));
        assert!(content.contains("https://example.com/about/"));
    }

    #[test]
    fn test_sitemap_excludes_404() {
        let mut site = minimal_site();
        site.pages.push(Page {
            content: Content {
                slug: "404".to_string(),
                title: "Not Found".to_string(),
                html: String::new(),
                raw_content: String::new(),
                frontmatter: Frontmatter::default(),
                path: PathBuf::from("404.html"),
                template: None,
                weight: 0,
                word_count: 0,
                reading_time: 0,
                toc: vec![],
                url: "/404/".to_string(),
            },
            draft: false,
            redirect_from: vec![],
        });

        let output_dir = tempfile::TempDir::new().unwrap();
        generate_sitemap(&site, output_dir.path()).unwrap();

        let content = std::fs::read_to_string(output_dir.path().join("sitemap.xml")).unwrap();
        assert!(!content.contains("/404/"));
    }

    #[test]
    fn test_sitemap_tags_and_categories() {
        let mut site = minimal_site();
        site.posts
            .push(make_post("hello", vec!["rust"], vec!["tech"]));

        let output_dir = tempfile::TempDir::new().unwrap();
        generate_sitemap(&site, output_dir.path()).unwrap();

        let content = std::fs::read_to_string(output_dir.path().join("sitemap.xml")).unwrap();
        assert!(content.contains("/tags/"));
        assert!(content.contains("/tags/rust/"));
        assert!(content.contains("/categories/"));
        assert!(content.contains("/categories/tech/"));
    }

    #[test]
    fn test_sitemap_pagination() {
        let mut site = minimal_site();
        site.config.posts_per_page = 1;
        site.posts.push(make_post("a", vec![], vec![]));
        site.posts.push(make_post("b", vec![], vec![]));

        let output_dir = tempfile::TempDir::new().unwrap();
        generate_sitemap(&site, output_dir.path()).unwrap();

        let content = std::fs::read_to_string(output_dir.path().join("sitemap.xml")).unwrap();
        assert!(content.contains("/page/2/"));
    }

    #[test]
    fn test_sitemap_collections() {
        let mut site = minimal_site();
        site.collections.insert(
            "docs".to_string(),
            Collection {
                name: "docs".to_string(),
                items: vec![CollectionItem {
                    content: Content {
                        slug: "intro".to_string(),
                        title: "Intro".to_string(),
                        html: String::new(),
                        raw_content: String::new(),
                        frontmatter: Frontmatter::default(),
                        path: PathBuf::from("docs/intro/index.html"),
                        template: None,
                        weight: 0,
                        word_count: 0,
                        reading_time: 0,
                        toc: vec![],
                        url: "/docs/intro/".to_string(),
                    },
                }],
            },
        );

        let output_dir = tempfile::TempDir::new().unwrap();
        generate_sitemap(&site, output_dir.path()).unwrap();

        let content = std::fs::read_to_string(output_dir.path().join("sitemap.xml")).unwrap();
        assert!(content.contains("/docs/"));
        assert!(content.contains("/docs/intro/"));
    }
}
