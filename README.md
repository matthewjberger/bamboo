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

Bamboo transforms markdown content with frontmatter into static HTML sites. It features syntax highlighting, Tera templating, live reload development, and generates RSS feeds and sitemaps automatically.

## Features

| Feature | Description |
|---------|-------------|
| **Markdown** | Full CommonMark support with tables, footnotes, strikethrough, task lists |
| **Frontmatter** | TOML (`+++`) or YAML (`---`) metadata in content files |
| **Syntax Highlighting** | Built-in highlighting via syntect with base16-ocean.dark theme |
| **Templating** | Tera templates with inheritance, includes, filters, and macros |
| **Collections** | Organize content beyond posts — projects, recipes, portfolios |
| **Data Files** | Load TOML/YAML/JSON from `data/` directory into templates |
| **Live Reload** | Development server with automatic rebuild on file changes |
| **SEO** | Automatic RSS feed and sitemap.xml generation |

## Quick Start

```bash
bamboo new my-site
cd my-site
bamboo serve
```

Open http://localhost:3000 to see your site. Edit files and watch them rebuild instantly.

## Project Structure

```
my-site/
├── bamboo.toml              # Site configuration
├── content/
│   ├── _index.md            # Home page
│   ├── about.md             # Static page → /about/
│   ├── posts/               # Blog posts → /posts/<slug>/
│   │   └── 2024-01-15-hello.md
│   └── projects/            # Collection → /projects/<slug>/
│       └── my-project.md
├── data/                    # Data files accessible in templates
│   └── nav.toml             # → {{ site.data.nav }}
└── static/                  # Copied as-is to output
    └── images/
```

## Configuration

`bamboo.toml`:

```toml
title = "My Site"
base_url = "https://example.com"
description = "A site built with Bamboo"
author = "Your Name"
language = "en"

[extra]
github = "https://github.com/username"
twitter = "@username"
```

All `[extra]` fields are available in templates as `{{ site.config.extra.github }}`.

## Content

### Frontmatter

TOML style:

```markdown
+++
title = "My Post"
date = 2024-01-15
tags = ["rust", "web"]
draft = false
weight = 10
template = "custom.html"
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

| Field | Type | Description |
|-------|------|-------------|
| `title` | string | Page/post title (required) |
| `date` | date | Publication date (posts only, or parsed from filename) |
| `draft` | bool | Exclude from build unless `--drafts` flag |
| `tags` | array | Post tags |
| `categories` | array | Post categories |
| `weight` | number | Sort order (lower = first) |
| `template` | string | Override default template |

### Date from Filename

Posts can embed dates in filenames: `2024-01-15-hello-world.md` extracts date `2024-01-15` and slug `hello-world`.

## Templating

Bamboo uses [Tera](https://keats.github.io/tera/) for templating. Templates live in your theme's `templates/` directory.

### Template Context

**All templates receive:**

```
site.config          # bamboo.toml contents
site.config.title    # Site title
site.config.base_url # Base URL
site.config.extra.*  # Custom fields
site.pages           # All pages
site.posts           # All posts (newest first)
site.collections     # Map of collection name → collection
site.data            # Data from data/ directory
```

**Page templates (`page.html`):**

```
page.title           # Page title
page.content         # Rendered HTML
page.slug            # URL slug
page.path            # Output path
```

**Post templates (`post.html`):**

```
post.title           # Post title
post.content         # Rendered HTML
post.date            # Publication date
post.tags            # Tag list
post.excerpt         # Auto-generated excerpt
post.slug            # URL slug
```

**Collection templates:**

```
collection.name      # Collection name
collection.items     # List of items
item.title           # Item title
item.content         # Rendered HTML
```

### Template Example

```html
{% extends "base.html" %}

{% block content %}
<article>
  <h1>{{ post.title }}</h1>
  <time>{{ post.date | date(format="%B %d, %Y") }}</time>

  <div class="tags">
    {% for tag in post.tags %}
    <span class="tag">{{ tag }}</span>
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

Bamboo includes a built-in theme with light/dark mode toggle. Create custom themes by specifying a theme directory:

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
│   └── partials/
│       ├── header.html
│       └── footer.html
├── static/
│   └── css/
└── theme.toml
```

## Examples

The repository includes example sites:

| Example | Description | Command |
|---------|-------------|---------|
| `blog` | Standard blog with posts, pages, collections | `just example blog` |
| `changelog` | Software release notes and changelog | `just example changelog` |
| `docs` | Documentation site with sidebar navigation | `just example docs` |
| `landing` | Product landing page with pricing | `just example landing` |
| `portfolio` | Creative portfolio with project showcase | `just example portfolio` |
| `slideshow` | Presentation/slideshow site | `just example slideshow` |

## Output

Building generates:

```
dist/
├── index.html           # Home page
├── about/index.html     # Static pages
├── posts/
│   └── hello/index.html # Blog posts
├── projects/
│   ├── index.html       # Collection index
│   └── my-project/index.html
├── rss.xml              # RSS feed
├── sitemap.xml          # Sitemap
└── images/              # Copied from static/
```

## As a Library

Use `bamboo-ssg` as a library in your own tools:

```toml
[dependencies]
bamboo-ssg = "0.1"
```

```rust
use bamboo_ssg::{SiteBuilder, ThemeEngine};

let site = SiteBuilder::new("./my-site")
    .base_url("https://example.com")
    .include_drafts(false)
    .build()?;

let theme = ThemeEngine::new("default")?;
theme.render_site(&site, "./dist")?;
```

## License

Dual-licensed under MIT ([LICENSE-MIT](LICENSE-MIT)) or Apache 2.0 ([LICENSE-APACHE](LICENSE-APACHE)).
