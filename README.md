<h1 align="center">bamboo 🎋</h1>

<p align="center">
  <a href="https://github.com/matthewjberger/bamboo"><img alt="github" src="https://img.shields.io/badge/github-matthewjberger/bamboo-8da0cb?style=for-the-badge&labelColor=555555&logo=github" height="20"></a>
  <a href="https://crates.io/crates/bamboo-ssg"><img alt="crates.io" src="https://img.shields.io/crates/v/bamboo-ssg.svg?style=for-the-badge&color=fc8d62&logo=rust" height="20"></a>
  <a href="https://github.com/matthewjberger/bamboo/blob/main/LICENSE-MIT"><img alt="license" src="https://img.shields.io/badge/license-MIT%2FApache--2.0-blue?style=for-the-badge&labelColor=555555" height="20"></a>
</p>

<p align="center"><strong>A fast static site generator written in Rust.</strong></p>

<p align="center">
  <a href="https://matthewjberger.github.io/bamboo/">Live Demo</a> · <code>cargo install bamboo-cli</code>
</p>

Bamboo transforms markdown content with frontmatter into static HTML sites. It features syntax highlighting, Tera templating, live reload development, shortcodes, asset optimization, responsive images, and generates RSS/Atom feeds, sitemaps, and search indexes automatically.

## Features

| Feature | Description |
|---------|-------------|
| **Markdown** | Full CommonMark support with tables, footnotes, strikethrough, task lists |
| **Frontmatter** | TOML (`+++`) or YAML (`---`) metadata in content files |
| **Syntax Highlighting** | Built-in highlighting via syntect with configurable themes |
| **Templating** | Tera templates with inheritance, includes, filters, and macros |
| **Shortcodes** | Inline (`{{</* name */>}}`) and block (`{{%/* name */%}}`) shortcodes with Tera templates |
| **Collections** | Organize content beyond posts — projects, recipes, portfolios (supports nesting) |
| **Data Files** | Load TOML/YAML/JSON from `data/` directory into templates |
| **Pagination** | Automatic pagination for post listings, tag pages, and category pages |
| **Tags & Categories** | Auto-generated tag and category index/listing pages |
| **Table of Contents** | Auto-generated heading-based TOC available in templates |
| **Reading Time** | Word count and estimated reading time for all content |
| **Search** | Client-side search with auto-generated JSON index and Fuse.js |
| **Feeds** | Automatic RSS and Atom feed generation |
| **Sitemap** | Automatic sitemap.xml generation |
| **Redirects** | `redirect_from` frontmatter for old URL redirects |
| **Asset Pipeline** | CSS/JS/HTML minification and content-hash fingerprinting |
| **Responsive Images** | Automatic resizing and `<picture>` srcset generation |
| **Live Reload** | Development server with automatic rebuild on file changes |
| **Sass/SCSS** | Automatic Sass/SCSS compilation to CSS |
| **Math** | LaTeX math rendering with KaTeX support (`$...$` inline, `$$...$$` display) |
| **Custom Taxonomies** | Define custom taxonomies beyond tags and categories |
| **Custom Permalinks** | Override output URL via `permalink` frontmatter field |
| **Cross-References** | Link between content with `{{</* ref "page.md" */>}}` shortcodes |
| **Incremental Builds** | Only rebuild changed content during development |
| **Themes** | Built-in default theme with light/dark mode, or use custom themes with overrides |

## Quick Start

```bash
cargo install bamboo-cli
bamboo new my-site
cd my-site
bamboo serve
```

Open http://localhost:3000 to see your site. Edit files and watch them rebuild instantly.

### Template

Use the [bamboo-template](https://github.com/matthewjberger/bamboo-template) for a ready-to-deploy starter with GitHub Pages CI included.

## CLI Commands

```bash
bamboo new <name>              # Create a new site in a new directory
bamboo init                    # Initialize a site in the current directory
bamboo build                   # Build the site to dist/
bamboo build --drafts          # Include draft content
bamboo build --theme ./mytheme # Use a custom theme
bamboo build --output ./public # Custom output directory
bamboo build --base-url <url>  # Override base URL
bamboo serve                   # Dev server with live reload at localhost:3000
bamboo serve --port 8080       # Custom port
bamboo serve --open            # Open browser automatically
bamboo serve --drafts          # Include drafts in dev server
```

## Project Structure

```
my-site/
├── bamboo.toml              # Site configuration
├── content/
│   ├── _index.md            # Home page
│   ├── about.md             # Static page → /about/
│   ├── docs/                # Nested pages
│   │   └── _index.md        # → /docs/
│   ├── posts/               # Blog posts → /posts/<slug>/
│   │   └── 2024-01-15-hello.md
│   └── projects/            # Collection (needs _collection.toml)
│       ├── _collection.toml
│       ├── my-project.md
│       └── archived/        # Nested subdirectories supported
│           └── old-project.md
├── data/                    # Data files accessible in templates
│   └── nav.toml             # → {{ site.data.nav }}
├── static/                  # Copied as-is to output
│   └── images/
└── templates/               # Site-level template overrides
    └── shortcodes/          # Custom shortcode templates
```

## Configuration

`bamboo.toml`:

```toml
title = "My Site"
base_url = "https://example.com"
description = "A site built with Bamboo"
author = "Your Name"
language = "en"
posts_per_page = 10    # Posts per page (0 = all on one page)
syntax_theme = "base16-ocean.dark"  # Syntax highlighting theme
math = false           # Enable LaTeX math rendering
minify = false         # Minify CSS, JS, and HTML output
fingerprint = false    # Content-hash asset filenames for cache busting

[taxonomies.tags]      # Built-in (auto-configured)
singular = "tag"

[taxonomies.categories]  # Built-in (auto-configured)
singular = "category"

[images]               # Responsive image generation (optional)
widths = [320, 640, 1024, 1920]
quality = 80
formats = ["webp", "jpg"]

[extra]
github = "https://github.com/username"
```

All `[extra]` fields are available in templates as `{{ site.config.extra.github }}`.

## Content

### Frontmatter

TOML style:

```markdown
+++
title = "My Post"
date = "2024-01-15"
tags = ["rust", "web"]
categories = ["tutorials"]
draft = false
weight = 10
template = "custom.html"
excerpt = "Custom excerpt text"
redirect_from = ["/old-url/"]
+++

Your content here...
```

YAML style:

```markdown
---
title: My Post
date: 2024-01-15
tags:
  - rust
  - web
---

Your content here...
```

### Frontmatter Fields

| Field | Type | Applies to | Description |
|-------|------|------------|-------------|
| `title` | string | all | Page/post title (defaults to filename) |
| `date` | date | posts | Publication date (or parsed from filename) |
| `draft` | bool | all | Exclude from build unless `--drafts` flag |
| `tags` | array | posts | Post tags for tag pages |
| `categories` | array | posts | Post categories for category pages |
| `weight` | number | pages, items | Sort order (lower = first) |
| `template` | string | all | Override default template |
| `excerpt` | string | posts | Custom excerpt (auto-generated from first paragraph if omitted) |
| `permalink` | string | all | Override the output URL (e.g. `/custom-path/`) |
| `redirect_from` | array | posts, pages | Old URLs that redirect to this content |
| `math` | bool | all | Enable LaTeX math for this page (when not globally enabled) |

### Date from Filename

Posts can embed dates in filenames: `2024-01-15-hello-world.md` extracts date `2024-01-15` and slug `hello-world`.

### Nested Pages

Pages can be organized in subdirectories. Use `_index.md` for directory index pages:

- `content/docs/_index.md` → `/docs/`
- `content/docs/getting-started.md` → `/docs/getting-started/`

## Shortcodes

Shortcodes embed reusable components in markdown content.

### Inline Shortcodes

```markdown
{{</* youtube id="dQw4w9WgXcQ" */>}}
{{</* figure src="/images/photo.jpg" caption="A photo" */>}}
{{</* gist user="username" id="abc123" */>}}
```

### Block Shortcodes

Block shortcodes wrap markdown content:

```markdown
{{%/* note type="warning" title="Heads up" */%}}
This content is rendered as **markdown** inside the shortcode.
{{%/* /note */%}}

{{%/* details summary="Click to expand" */%}}
Hidden content here.
{{%/* /details */%}}
```

### Built-in Shortcodes

| Shortcode | Type | Parameters |
|-----------|------|------------|
| `youtube` | inline | `id` (required), `title` |
| `figure` | inline | `src` (required), `alt`, `caption`, `width`, `height`, `class` |
| `gist` | inline | `user` (required), `id` (required), `file` |
| `note` | block | `type` (info/warning/error), `title`, body content |
| `details` | block | `summary`, `open`, body content |

### Custom Shortcodes

Place Tera HTML templates in `templates/shortcodes/`. Parameters become template variables:

```html
<!-- templates/shortcodes/alert.html -->
<div class="alert alert-{{ level }}">{{ body }}</div>
```

```markdown
{{%/* alert level="danger" */%}}
Something went wrong!
{{%/* /alert */%}}
```

## Templating

Bamboo uses [Tera](https://keats.github.io/tera/) for templating. Templates live in your theme's `templates/` directory. Site-level templates in `templates/` override theme templates.

### Template Context

**All templates receive:**

| Variable | Description |
|----------|-------------|
| `site.config` | bamboo.toml contents |
| `site.config.title` | Site title |
| `site.config.base_url` | Base URL |
| `site.config.extra.*` | Custom fields from `[extra]` |
| `site.pages` | All pages (for navigation) |
| `site.data` | Data from `data/` directory |
| `site.collections` | Map of collection name to collection |

**Index template (`index.html`):**

| Variable | Description |
|----------|-------------|
| `posts` | Posts for current page (first `posts_per_page`) |
| `home` / `page` | Home page content (from `_index.md`) |
| `current_page` | Current page number |
| `total_pages` | Total number of pages |
| `next_page_url` | URL to next page (if exists) |

**Post template (`post.html`):**

| Variable | Description |
|----------|-------------|
| `post.title` | Post title |
| `post.content` | Rendered HTML (use `{{ post.content \| safe }}`) |
| `post.date` | Publication date |
| `post.tags` | Tag list |
| `post.categories` | Category list |
| `post.excerpt` | Auto-generated or custom excerpt |
| `post.slug` | URL slug |
| `post.url` | Full URL path |
| `post.word_count` | Word count |
| `post.reading_time` | Estimated minutes to read |
| `post.toc` | Table of contents entries |
| `prev_post` | Previous (older) post |
| `next_post` | Next (newer) post |

**Page template (`page.html`):**

| Variable | Description |
|----------|-------------|
| `page.title` | Page title |
| `page.content` | Rendered HTML |
| `page.slug` | URL slug |
| `page.url` | Full URL path |
| `page.word_count` | Word count |
| `page.reading_time` | Estimated minutes to read |
| `page.toc` | Table of contents entries |

**Tag/Category page templates (`tag.html`, `category.html`):**

| Variable | Description |
|----------|-------------|
| `tag_name` / `category_name` | Display name |
| `tag_slug` / `category_slug` | URL slug |
| `posts` | Posts with this tag/category (paginated) |
| `current_page`, `total_pages` | Pagination info |
| `prev_page_url`, `next_page_url` | Pagination links |

**Collection templates (`collection.html`, `collection_item.html`):**

| Variable | Description |
|----------|-------------|
| `collection` | Collection with `name` and `items` |
| `collection_name` | Collection name |
| `item` | Current item (in item template) |

### Custom Filters

| Filter | Description |
|--------|-------------|
| `slugify` | Convert text to URL slug |
| `reading_time` | Estimated minutes to read content |
| `word_count` | Count words in content |
| `toc` | Render table of contents as HTML (use with `\| safe`) |

### Template Example

```html
{% extends "base.html" %}

{% block content %}
<article>
  <h1>{{ post.title }}</h1>
  <time>{{ post.date | date(format="%B %d, %Y") }}</time>
  <span>{{ post.reading_time }} min read</span>

  <div class="tags">
    {% for tag in post.tags %}
    <a href="{{ site.config.base_url }}/tags/{{ tag | slugify }}/">{{ tag }}</a>
    {% endfor %}
  </div>

  {{ post.content | safe }}
</article>
{% endblock %}
```

## Data Files

Place TOML, YAML, or JSON files in `data/`. They're accessible as `site.data.<filename>`.

```toml
# data/social.toml
[[links]]
name = "GitHub"
url = "https://github.com/username"

[[links]]
name = "Twitter"
url = "https://twitter.com/username"
```

```html
<!-- In template -->
{% for link in site.data.social.links %}
<a href="{{ link.url }}">{{ link.name }}</a>
{% endfor %}
```

Nested directories work: `data/nav/main.toml` → `site.data.nav.main`

## Themes

Bamboo includes a built-in default theme with light/dark mode toggle. Create custom themes by specifying a theme directory:

```bash
bamboo build --theme ./my-theme
```

### Theme Structure

```
my-theme/
├── templates/
│   ├── base.html
│   ├── index.html
│   ├── page.html
│   ├── post.html
│   ├── collection.html
│   ├── collection_item.html
│   ├── tags.html
│   ├── tag.html
│   ├── categories.html
│   ├── category.html
│   ├── pagination.html
│   ├── search.html
│   ├── 404.html
│   ├── shortcodes/
│   │   └── *.html
│   └── partials/
│       ├── header.html
│       ├── footer.html
│       └── nav.html
└── static/
    └── css/
```

### Theme Overrides

Override specific templates without creating a full theme by placing templates in your site's `templates/` directory. These take priority over theme templates.

## Examples

The repository includes example sites:

| Example | Description | Command |
|---------|-------------|---------|
| `blog` | Standard blog with posts, pages, collections | `just example blog` |
| `book` | Book with chapters, sidebar navigation, prev/next | `just example book` |
| `changelog` | Software release notes and changelog | `just example changelog` |
| `docs` | Documentation site with sidebar navigation | `just example docs` |
| `landing` | Product landing page with pricing | `just example landing` |
| `portfolio` | Creative portfolio with project showcase | `just example portfolio` |
| `slideshow` | Presentation/slideshow site | `just example slideshow` |

## Output

Building generates:

```
dist/
├── index.html               # Home page
├── style.css                 # Theme stylesheet (built-in theme)
├── 404.html                  # Not found page
├── about/index.html          # Static pages
├── posts/
│   └── hello/index.html      # Blog posts
├── tags/
│   ├── index.html            # Tags listing
│   └── rust/index.html       # Per-tag post listing
├── categories/
│   ├── index.html            # Categories listing
│   └── tutorials/index.html  # Per-category post listing
├── page/
│   └── 2/index.html          # Pagination pages
├── search/
│   └── index.html            # Search page
├── projects/
│   ├── index.html            # Collection index
│   ├── my-project/index.html # Collection items
│   └── archived/
│       └── old-project/index.html  # Nested collection items
├── rss.xml                   # RSS feed
├── atom.xml                  # Atom feed
├── sitemap.xml               # Sitemap
└── search-index.json         # Client-side search index
```

## As a Library

Use `bamboo-ssg` as a library in your own tools:

```toml
[dependencies]
bamboo-ssg = "0.2.1"
```

```rust
use bamboo_ssg::{SiteBuilder, ThemeEngine};

let mut builder = SiteBuilder::new("./my-site")
    .base_url("https://example.com")
    .include_drafts(false);

let site = builder.build()?;

let theme = ThemeEngine::new("default")?;
theme.render_site(&site, "./dist")?;
```

## License

Dual-licensed under MIT ([LICENSE-MIT](LICENSE-MIT)) or Apache 2.0 ([LICENSE-APACHE](LICENSE-APACHE)).
