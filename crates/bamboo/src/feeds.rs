//! RSS 2.0 and Atom feed generation for a built [`Site`].

use crate::error::Result;
use crate::parsing::extract_excerpt;
use crate::types::{Collection, Site};
use crate::xml::escape;
use std::fs;
use std::path::Path;

/// Writes an RSS 2.0 feed (`rss.xml`) covering every post to `output_dir`.
pub fn generate_rss(site: &Site, output_dir: &Path) -> Result<()> {
    let base_url = site.config.base_url.trim_end_matches('/');
    let language = site.config.language.as_deref().unwrap_or("en");

    let mut items = String::new();
    for post in &site.posts {
        let post_url = format!("{}/posts/{}/", base_url, post.content.slug);
        let pub_date = post.date.format("%a, %d %b %Y %H:%M:%S +0000").to_string();
        let description = escape(post.excerpt.as_deref().unwrap_or(""));

        items.push_str(&format!(
            r#"    <item>
      <title>{}</title>
      <link>{}</link>
      <guid>{}</guid>
      <pubDate>{}</pubDate>
      <description>{}</description>
    </item>
"#,
            escape(&post.content.title),
            escape(&post_url),
            escape(&post_url),
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
        escape(&site.config.title),
        escape(base_url),
        escape(site.config.description.as_deref().unwrap_or("")),
        escape(language),
        escape(base_url),
        items
    );

    fs::write(output_dir.join("rss.xml"), rss)?;

    Ok(())
}

/// Writes an Atom feed (`atom.xml`) covering every post to `output_dir`.
pub fn generate_atom(site: &Site, output_dir: &Path) -> Result<()> {
    let base_url = site.config.base_url.trim_end_matches('/');

    let updated = site
        .posts
        .first()
        .map(|post| post.date.to_rfc3339())
        .unwrap_or_else(|| chrono::Utc::now().to_rfc3339());

    let mut entries = String::new();
    for post in &site.posts {
        let post_url = format!("{}/posts/{}/", base_url, post.content.slug);
        let summary = post.excerpt.as_deref().unwrap_or("");

        entries.push_str(&format!(
            r#"  <entry>
    <title>{title}</title>
    <link href="{url}" rel="alternate"/>
    <id>{url}</id>
    <updated>{updated}</updated>
    <summary type="text">{summary}</summary>
    <content type="html">{content}</content>
  </entry>
"#,
            title = escape(&post.content.title),
            url = escape(&post_url),
            updated = post.date.to_rfc3339(),
            summary = escape(summary),
            content = escape(&post.content.html),
        ));
    }

    let author_name = site.config.author.as_deref().unwrap_or(&site.config.title);

    let atom = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<feed xmlns="http://www.w3.org/2005/Atom">
  <title>{title}</title>
  <link href="{base_url}/" rel="alternate"/>
  <link href="{base_url}/atom.xml" rel="self"/>
  <id>{base_url}/</id>
  <updated>{updated}</updated>
  <author>
    <name>{author}</name>
  </author>
  <subtitle>{description}</subtitle>
{entries}</feed>
"#,
        title = escape(&site.config.title),
        base_url = escape(base_url),
        updated = updated,
        author = escape(author_name),
        description = escape(site.config.description.as_deref().unwrap_or("")),
        entries = entries,
    );

    fs::write(output_dir.join("atom.xml"), atom)?;

    Ok(())
}

/// Writes a per-collection RSS feed to `output_dir/<collection>/rss.xml`.
pub fn generate_collection_rss(
    site: &Site,
    collection_name: &str,
    collection: &Collection,
    output_dir: &Path,
) -> Result<()> {
    let base_url = site.config.base_url.trim_end_matches('/');
    let language = site.config.language.as_deref().unwrap_or("en");

    let mut items = String::new();
    for item in &collection.items {
        let item_url = format!("{}{}", base_url, item.content.url);

        let description = extract_excerpt(&item.content.raw_content, 200).unwrap_or_default();

        let mut item_xml = format!(
            "    <item>\n      <title>{}</title>\n      <link>{}</link>\n      <guid>{}</guid>\n      <description>{}</description>\n",
            escape(&item.content.title),
            escape(&item_url),
            escape(&item_url),
            escape(&description),
        );

        if let Some(date_str) = item.content.frontmatter.get_string("date")
            && let Ok(naive) = chrono::NaiveDate::parse_from_str(&date_str, "%Y-%m-%d")
        {
            let datetime = chrono::TimeZone::from_utc_datetime(
                &chrono::Utc,
                &naive.and_time(chrono::NaiveTime::MIN),
            );
            let pub_date = datetime.format("%a, %d %b %Y %H:%M:%S +0000").to_string();
            item_xml.push_str(&format!("      <pubDate>{}</pubDate>\n", pub_date));
        }

        item_xml.push_str("    </item>\n");
        items.push_str(&item_xml);
    }

    let rss = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<rss version="2.0" xmlns:atom="http://www.w3.org/2005/Atom">
  <channel>
    <title>{} - {}</title>
    <link>{}/{}/</link>
    <description>{}</description>
    <language>{}</language>
    <atom:link href="{}/{}/rss.xml" rel="self" type="application/rss+xml"/>
{}  </channel>
</rss>
"#,
        escape(&site.config.title),
        escape(collection_name),
        escape(base_url),
        escape(collection_name),
        escape(site.config.description.as_deref().unwrap_or("")),
        escape(language),
        escape(base_url),
        escape(collection_name),
        items
    );

    let collection_dir = output_dir.join(collection_name);
    fs::create_dir_all(&collection_dir)?;
    fs::write(collection_dir.join("rss.xml"), rss)?;

    Ok(())
}

/// Writes a per-collection Atom feed to `output_dir/<collection>/atom.xml`.
pub fn generate_collection_atom(
    site: &Site,
    collection_name: &str,
    collection: &Collection,
    output_dir: &Path,
) -> Result<()> {
    let base_url = site.config.base_url.trim_end_matches('/');

    let updated = collection
        .items
        .iter()
        .filter_map(|item| item.content.frontmatter.get_string("date"))
        .filter_map(|date_str| chrono::NaiveDate::parse_from_str(&date_str, "%Y-%m-%d").ok())
        .max()
        .map(|date| {
            chrono::TimeZone::from_utc_datetime(
                &chrono::Utc,
                &date.and_time(chrono::NaiveTime::MIN),
            )
            .to_rfc3339()
        })
        .unwrap_or_else(|| chrono::Utc::now().to_rfc3339());

    let mut entries = String::new();
    for item in &collection.items {
        let item_url = format!("{}{}", base_url, item.content.url);

        let entry_updated = item
            .content
            .frontmatter
            .get_string("date")
            .and_then(|date_str| chrono::NaiveDate::parse_from_str(&date_str, "%Y-%m-%d").ok())
            .map(|date| {
                chrono::TimeZone::from_utc_datetime(
                    &chrono::Utc,
                    &date.and_time(chrono::NaiveTime::MIN),
                )
                .to_rfc3339()
            })
            .unwrap_or_else(|| updated.clone());

        entries.push_str(&format!(
            r#"  <entry>
    <title>{title}</title>
    <link href="{url}" rel="alternate"/>
    <id>{url}</id>
    <updated>{updated}</updated>
    <content type="html">{content}</content>
  </entry>
"#,
            title = escape(&item.content.title),
            url = escape(&item_url),
            updated = entry_updated,
            content = escape(&item.content.html),
        ));
    }

    let author_name = site.config.author.as_deref().unwrap_or(&site.config.title);

    let atom = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<feed xmlns="http://www.w3.org/2005/Atom">
  <title>{title} - {collection}</title>
  <link href="{base_url}/{collection}/" rel="alternate"/>
  <link href="{base_url}/{collection}/atom.xml" rel="self"/>
  <id>{base_url}/{collection}/</id>
  <updated>{updated}</updated>
  <author>
    <name>{author}</name>
  </author>
{entries}</feed>
"#,
        title = escape(&site.config.title),
        collection = escape(collection_name),
        base_url = escape(base_url),
        updated = updated,
        author = escape(author_name),
        entries = entries,
    );

    let collection_dir = output_dir.join(collection_name);
    fs::create_dir_all(&collection_dir)?;
    fs::write(collection_dir.join("atom.xml"), atom)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::*;
    use chrono::{NaiveDate, NaiveTime, TimeZone, Utc};
    use std::collections::HashMap;
    use std::path::PathBuf;

    fn test_site_with_post() -> Site {
        let date = Utc.from_utc_datetime(
            &NaiveDate::from_ymd_opt(2024, 6, 15)
                .unwrap()
                .and_time(NaiveTime::MIN),
        );
        Site {
            config: SiteConfig {
                title: "Test Blog".to_string(),
                base_url: "https://example.com".to_string(),
                description: Some("A test blog".to_string()),
                author: Some("Author".to_string()),
                language: Some("en".to_string()),
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
            posts: vec![Post {
                content: Content {
                    slug: "hello-world".to_string(),
                    title: "Hello World".to_string(),
                    html: "<p>Hello</p>".to_string(),
                    raw_content: "Hello".to_string(),
                    frontmatter: Frontmatter::default(),
                    path: PathBuf::from("posts/hello-world/index.html"),
                    template: None,
                    weight: 0,
                    word_count: 1,
                    reading_time: 1,
                    toc: vec![],
                    url: "/posts/hello-world/".to_string(),
                },
                date,
                excerpt: Some("Hello excerpt".to_string()),
                draft: false,
                tags: vec!["test".to_string()],
                categories: vec![],
                taxonomies_map: HashMap::from([("tags".to_string(), vec!["test".to_string()])]),
                redirect_from: vec![],
            }],
            collections: HashMap::new(),
            data: HashMap::new(),
            assets: vec![],
        }
    }

    #[test]
    fn test_rss_basic_structure() {
        let site = test_site_with_post();
        let output_dir = tempfile::TempDir::new().unwrap();
        generate_rss(&site, output_dir.path()).unwrap();

        let rss_content = std::fs::read_to_string(output_dir.path().join("rss.xml")).unwrap();
        assert!(rss_content.contains("<?xml version=\"1.0\""));
        assert!(rss_content.contains("<rss version=\"2.0\""));
        assert!(rss_content.contains("<title>Test Blog</title>"));
        assert!(rss_content.contains("<title>Hello World</title>"));
        assert!(rss_content.contains("Hello excerpt"));
    }

    #[test]
    fn test_atom_basic_structure() {
        let site = test_site_with_post();
        let output_dir = tempfile::TempDir::new().unwrap();
        generate_atom(&site, output_dir.path()).unwrap();

        let atom_content = std::fs::read_to_string(output_dir.path().join("atom.xml")).unwrap();
        assert!(atom_content.contains("<feed xmlns=\"http://www.w3.org/2005/Atom\""));
        assert!(atom_content.contains("<title>Test Blog</title>"));
        assert!(atom_content.contains("<title>Hello World</title>"));
        assert!(atom_content.contains("<name>Author</name>"));
    }

    #[test]
    fn test_rss_xml_escaping() {
        let mut site = test_site_with_post();
        site.config.title = "Blog & <Friends>".to_string();
        let output_dir = tempfile::TempDir::new().unwrap();
        generate_rss(&site, output_dir.path()).unwrap();

        let rss_content = std::fs::read_to_string(output_dir.path().join("rss.xml")).unwrap();
        assert!(rss_content.contains("Blog &amp; &lt;Friends&gt;"));
    }

    #[test]
    fn test_atom_updated_uses_latest_post_date() {
        let site = test_site_with_post();
        let output_dir = tempfile::TempDir::new().unwrap();
        generate_atom(&site, output_dir.path()).unwrap();

        let atom_content = std::fs::read_to_string(output_dir.path().join("atom.xml")).unwrap();
        assert!(atom_content.contains("2024-06-15"));
    }

    fn test_collection() -> Collection {
        let mut frontmatter = Frontmatter::default();
        frontmatter
            .raw
            .insert("date".to_string(), serde_json::json!("2024-06-15"));

        Collection {
            name: "docs".to_string(),
            items: vec![CollectionItem {
                content: Content {
                    slug: "intro".to_string(),
                    title: "Introduction".to_string(),
                    html: "<p>Welcome</p>".to_string(),
                    raw_content: "Welcome".to_string(),
                    frontmatter,
                    path: PathBuf::from("docs/intro/index.html"),
                    template: None,
                    weight: 0,
                    word_count: 1,
                    reading_time: 1,
                    toc: vec![],
                    url: "/docs/intro/".to_string(),
                },
            }],
        }
    }

    #[test]
    fn test_collection_rss_structure() {
        let site = test_site_with_post();
        let collection = test_collection();
        let output_dir = tempfile::TempDir::new().unwrap();
        generate_collection_rss(&site, "docs", &collection, output_dir.path()).unwrap();

        let content = std::fs::read_to_string(output_dir.path().join("docs/rss.xml")).unwrap();
        assert!(content.contains("<title>Test Blog - docs</title>"));
        assert!(content.contains("<title>Introduction</title>"));
        assert!(content.contains("/docs/intro/"));
        assert!(content.contains("<pubDate>"));
        assert!(content.contains("<description>Welcome</description>"));
    }

    #[test]
    fn test_collection_atom_structure() {
        let site = test_site_with_post();
        let collection = test_collection();
        let output_dir = tempfile::TempDir::new().unwrap();
        generate_collection_atom(&site, "docs", &collection, output_dir.path()).unwrap();

        let content = std::fs::read_to_string(output_dir.path().join("docs/atom.xml")).unwrap();
        assert!(content.contains("<title>Test Blog - docs</title>"));
        assert!(content.contains("<title>Introduction</title>"));
        assert!(content.contains("/docs/intro/"));
        assert!(content.contains("2024-06-15"));
    }

    #[test]
    fn test_collection_rss_without_dates() {
        let site = test_site_with_post();
        let collection = Collection {
            name: "notes".to_string(),
            items: vec![CollectionItem {
                content: Content {
                    slug: "note-1".to_string(),
                    title: "Note One".to_string(),
                    html: "<p>A note</p>".to_string(),
                    raw_content: "A note".to_string(),
                    frontmatter: Frontmatter::default(),
                    path: PathBuf::from("notes/note-1/index.html"),
                    template: None,
                    weight: 0,
                    word_count: 2,
                    reading_time: 1,
                    toc: vec![],
                    url: "/notes/note-1/".to_string(),
                },
            }],
        };
        let output_dir = tempfile::TempDir::new().unwrap();
        generate_collection_rss(&site, "notes", &collection, output_dir.path()).unwrap();

        let content = std::fs::read_to_string(output_dir.path().join("notes/rss.xml")).unwrap();
        assert!(content.contains("<title>Note One</title>"));
        assert!(!content.contains("<pubDate>"));
        assert!(content.contains("<description>A note</description>"));
    }
}
