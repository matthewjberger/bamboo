use crate::error::Result;
use crate::parsing::slugify;
use crate::theme::SiteMetadata;
use crate::types::Site;
use rayon::prelude::*;
use serde::Serialize;
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use tera::{Context, Tera};

#[derive(Debug, Clone, Serialize)]
struct TaxonomyInfo {
    name: String,
    slug: String,
    count: usize,
}

struct TaxonomyConfig<'a> {
    taxonomy_name: &'a str,
    index_template: &'a str,
    item_template: &'a str,
    name_context_key: &'a str,
    slug_context_key: &'a str,
}

impl<'a> TaxonomyConfig<'a> {
    fn index_template_or_fallback(&self, tera: &Tera) -> &'a str {
        if tera
            .get_template_names()
            .any(|name| name == self.index_template)
        {
            self.index_template
        } else {
            "taxonomy.html"
        }
    }

    fn item_template_or_fallback(&self, tera: &Tera) -> &'a str {
        if tera
            .get_template_names()
            .any(|name| name == self.item_template)
        {
            self.item_template
        } else {
            "taxonomy_term.html"
        }
    }
}

pub(crate) fn render_all_taxonomies(
    tera: &Tera,
    site: &Site,
    metadata: &SiteMetadata,
    output_dir: &Path,
) -> Result<()> {
    for (taxonomy_name, taxonomy_definition) in &site.config.taxonomies {
        let singular = taxonomy_definition
            .singular
            .clone()
            .unwrap_or_else(|| taxonomy_name.trim_end_matches('s').to_string());

        let (index_template, item_template, name_context_key, slug_context_key) =
            match taxonomy_name.as_str() {
                "tags" => {
                    let index_tpl = taxonomy_definition
                        .index_template
                        .as_deref()
                        .unwrap_or("tags.html");
                    let item_tpl = taxonomy_definition
                        .term_template
                        .as_deref()
                        .unwrap_or("tag.html");
                    (
                        index_tpl.to_string(),
                        item_tpl.to_string(),
                        "tag_name".to_string(),
                        "tag_slug".to_string(),
                    )
                }
                "categories" => {
                    let index_tpl = taxonomy_definition
                        .index_template
                        .as_deref()
                        .unwrap_or("categories.html");
                    let item_tpl = taxonomy_definition
                        .term_template
                        .as_deref()
                        .unwrap_or("category.html");
                    (
                        index_tpl.to_string(),
                        item_tpl.to_string(),
                        "category_name".to_string(),
                        "category_slug".to_string(),
                    )
                }
                _ => {
                    let index_tpl = taxonomy_definition
                        .index_template
                        .as_deref()
                        .unwrap_or("taxonomy.html")
                        .to_string();
                    let item_tpl = taxonomy_definition
                        .term_template
                        .as_deref()
                        .unwrap_or("taxonomy_term.html")
                        .to_string();
                    (
                        index_tpl,
                        item_tpl,
                        format!("{}_name", singular),
                        format!("{}_slug", singular),
                    )
                }
            };

        let config = TaxonomyConfig {
            taxonomy_name,
            index_template: &index_template,
            item_template: &item_template,
            name_context_key: &name_context_key,
            slug_context_key: &slug_context_key,
        };

        let taxonomy_name_owned = taxonomy_name.clone();
        render_taxonomy_pages(tera, site, metadata, output_dir, config, |post| {
            post.taxonomies_map
                .get(&taxonomy_name_owned)
                .into_iter()
                .flat_map(|terms| terms.iter())
        })?;
    }
    Ok(())
}

fn render_taxonomy_pages<'a, F, I>(
    tera: &Tera,
    site: &'a Site,
    metadata: &SiteMetadata,
    output_dir: &Path,
    taxonomy_config: TaxonomyConfig,
    extract_terms: F,
) -> Result<()>
where
    F: Fn(&'a crate::types::Post) -> I,
    I: Iterator<Item = &'a String>,
{
    let mut slug_posts: HashMap<String, Vec<&crate::types::Post>> = HashMap::new();
    let mut slug_display_name: HashMap<String, String> = HashMap::new();

    for post in &site.posts {
        for term in extract_terms(post) {
            let slug = slugify(term);
            slug_posts.entry(slug.clone()).or_default().push(post);
            slug_display_name
                .entry(slug)
                .or_insert_with(|| term.clone());
        }
    }

    if slug_posts.is_empty() {
        return Ok(());
    }

    let mut taxonomy_items: Vec<TaxonomyInfo> = slug_posts
        .iter()
        .map(|(slug, posts)| TaxonomyInfo {
            name: slug_display_name
                .get(slug)
                .cloned()
                .unwrap_or_else(|| slug.clone()),
            slug: slug.clone(),
            count: posts.len(),
        })
        .collect();
    taxonomy_items.sort_by(|a, b| a.name.cmp(&b.name));

    let mut context = Context::new();
    context.insert("site", metadata);
    context.insert(taxonomy_config.taxonomy_name, &taxonomy_items);
    context.insert("taxonomy_items", &taxonomy_items);
    context.insert("taxonomy_name", taxonomy_config.taxonomy_name);

    let taxonomy_dir = output_dir.join(taxonomy_config.taxonomy_name);
    let taxonomy_index = taxonomy_dir.join("index.html");
    let index_template = taxonomy_config.index_template_or_fallback(tera);
    let rendered = tera.render(index_template, &context)?;
    fs::create_dir_all(&taxonomy_dir)?;
    fs::write(taxonomy_index, rendered)?;

    let posts_per_page = site.config.posts_per_page;

    let item_template = taxonomy_config.item_template_or_fallback(tera);

    let slug_entries: Vec<_> = slug_posts.iter().collect();
    slug_entries
        .par_iter()
        .try_for_each(|(slug, posts)| -> Result<()> {
            let display_name = slug_display_name.get(slug.as_str()).unwrap_or(slug);
            let term_dir = taxonomy_dir.join(slug);
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
                context.insert("site", metadata);
                context.insert(taxonomy_config.name_context_key, display_name);
                context.insert(taxonomy_config.slug_context_key, &slug);
                context.insert("term_name", display_name);
                context.insert("term_slug", &slug);
                context.insert("taxonomy_name", taxonomy_config.taxonomy_name);
                context.insert("posts", page_posts);
                context.insert("current_page", &page_number);
                context.insert("total_pages", &total_pages);

                if page_number > 1 {
                    let prev_url = if page_number == 2 {
                        format!("{}/{}/{}/", base_url, taxonomy_config.taxonomy_name, slug)
                    } else {
                        format!(
                            "{}/{}/{}/page/{}/",
                            base_url,
                            taxonomy_config.taxonomy_name,
                            slug,
                            page_number - 1
                        )
                    };
                    context.insert("prev_page_url", &prev_url);
                }

                if page_number < total_pages {
                    let next_url = format!(
                        "{}/{}/{}/page/{}/",
                        base_url,
                        taxonomy_config.taxonomy_name,
                        slug,
                        page_number + 1
                    );
                    context.insert("next_page_url", &next_url);
                }

                if page_number == 1 {
                    let rendered = tera.render(item_template, &context)?;
                    fs::create_dir_all(&term_dir)?;
                    fs::write(term_dir.join("index.html"), rendered)?;
                } else {
                    let rendered = tera.render(item_template, &context)?;
                    let page_dir = term_dir.join("page").join(page_number.to_string());
                    fs::create_dir_all(&page_dir)?;
                    fs::write(page_dir.join("index.html"), rendered)?;
                }
            }

            Ok(())
        })?;

    Ok(())
}
