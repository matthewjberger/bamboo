//! Bamboo is a fast static site generator written in Rust.
//!
//! This crate is the library that powers the `bamboo` CLI. Most users will
//! prefer the CLI (`cargo install bamboo-cli`), but the library is useful when
//! you want to embed site generation into your own tool, generate output in
//! multiple passes, or compose custom post-processing around the render step.
//!
//! See the [project README](https://github.com/matthewjberger/bamboo) for the
//! authoring-side documentation — content directory layout, frontmatter fields,
//! shortcodes, and the Tera template context.
//!
//! # Entry points
//!
//! - [`SiteBuilder`] reads a site directory (`bamboo.toml`, `content/`, `data/`,
//!   `static/`, `templates/`) and produces an in-memory [`Site`] tree.
//! - [`ThemeEngine`] renders a [`Site`] to an output directory using Tera
//!   templates from the built-in default theme or a custom theme.
//!
//! # Example
//!
//! ```no_run
//! use bamboo_ssg::{SiteBuilder, ThemeEngine};
//!
//! let site = SiteBuilder::new("./my-site")
//!     .base_url("https://example.com")
//!     .include_drafts(false)
//!     .build()?;
//!
//! let theme = ThemeEngine::new("default")?;
//! theme.render_site(&site, std::path::Path::new("./dist"))?;
//! # Ok::<_, bamboo_ssg::BambooError>(())
//! ```

#![warn(missing_docs)]

pub mod assets;
pub mod cache;
pub mod error;
pub mod feeds;
pub mod images;
pub mod links;
pub mod parsing;
pub mod redirects;
pub mod search;
pub mod shortcodes;
pub mod site;
pub mod sitemap;
pub(crate) mod taxonomy;
pub mod theme;
pub mod types;
pub mod xml;

pub use cache::{
    BuildState, ChangeClassification, RenderTarget, classify_changes, compute_content_hashes,
    expand_targets, load_cache, save_cache, should_render,
};
pub use error::{BambooError, IoContext, Result};
pub use links::{LinkWarning, validate_internal_links};
pub use parsing::{
    MarkdownRenderer, RenderedMarkdown, extract_excerpt, extract_frontmatter,
    parse_date_from_filename, reading_time, slugify, word_count,
};
pub use site::SiteBuilder;
pub use theme::{ThemeEngine, clean_output_dir};
pub use types::{
    Asset, Collection, CollectionItem, Content, Frontmatter, Page, Post, Site, SiteConfig,
    TaxonomyDefinition, TocEntry,
};
