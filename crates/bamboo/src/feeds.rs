use crate::error::Result;
use crate::types::Site;
use crate::xml::escape;
use std::fs;
use std::path::Path;

pub fn generate_rss(site: &Site, output_dir: &Path) -> Result<()> {
    let base_url = site.config.base_url.trim_end_matches('/');
    let language = site.config.language.as_deref().unwrap_or("en");

    let mut items = String::new();
    for post in &site.posts {
        let post_url = format!("{}/posts/{}/", base_url, post.slug);
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
            escape(&post.title),
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

pub fn generate_atom(site: &Site, output_dir: &Path) -> Result<()> {
    let base_url = site.config.base_url.trim_end_matches('/');

    let updated = site
        .posts
        .first()
        .map(|post| post.date.to_rfc3339())
        .unwrap_or_else(|| chrono::Utc::now().to_rfc3339());

    let mut entries = String::new();
    for post in &site.posts {
        let post_url = format!("{}/posts/{}/", base_url, post.slug);
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
            title = escape(&post.title),
            url = escape(&post_url),
            updated = post.date.to_rfc3339(),
            summary = escape(summary),
            content = escape(&post.content),
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
