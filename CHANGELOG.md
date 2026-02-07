## [Unreleased]

### Features

- Pagination with configurable `posts_per_page`
- Tag pages with index (`/tags/`) and per-tag listings (`/tags/<slug>/`)
- Category pages with index (`/categories/`) and per-category listings
- Tag and category pagination for large post sets
- Table of contents generation from headings
- Heading anchor links with unique IDs
- Word count and estimated reading time for all content
- Auto-generated excerpts from first paragraph
- Atom feed generation alongside RSS
- 404 page support
- `redirect_from` frontmatter for URL redirects (posts and pages)
- Prev/next post navigation in post templates
- Nested page support with `_index.md` for directory indexes
- Site-level template overrides (place templates in `templates/` to override theme)
- Custom Tera filters: `slugify`, `reading_time`, `word_count`, `toc`
- `--open` flag for `bamboo serve` to open browser automatically
- Theme directory file watching in dev server
- `bamboo init` command to initialize site in current directory
- CSS/JS/HTML minification (`minify = true` in config)
- Content-hash asset fingerprinting (`fingerprint = true` in config)
- Responsive image generation with configurable widths, quality, and formats
- Automatic `<picture>` srcset generation in HTML output
- Client-side search with auto-generated JSON index and Fuse.js
- Shortcode system with inline (`{{< >}}`) and block (`{{% %}}`) syntax
- Built-in shortcodes: youtube, figure, gist, note, details
- Custom shortcode templates in `templates/shortcodes/`
- Nested shortcode support in block bodies
- Code fence protection (shortcodes not processed inside code blocks)
- Standalone CSS theme replacing Tailwind CDN dependency
- Draft support for pages (not just posts)
- `url` field on pages, posts, and collection items
- `SiteMetadata` for efficient template serialization
- Deterministic sitemap output (sorted entries)
- RSS/Atom feed XML escaping
- Search index with HTML stripping and content truncation

### Bug Fixes

- Heading inline markdown (bold, code, links) now renders correctly in headings
- Tag links use `slugify` filter consistently across all templates
- Index page only shows first `posts_per_page` posts instead of all
- Pagination page numbering starts at page 2 (page 1 is index)
- Redirect pages check for file conflicts before writing
- Generated route pages (tags, categories, search) don't overwrite user pages
- Collection index pages don't overwrite user pages at same slug
- `posts_per_page = 0` treated as "all on one page"
- Asset fingerprinting restricted to attribute contexts (prevents corrupting content)
- HTML minification runs after fingerprinting to preserve attribute quotes
- TOML frontmatter with `+++` in multiline strings parsed correctly
- Dev server rebuilds without cleaning output (prevents race condition)
- User images with `-<number>w` suffix not skipped by image processor
- Duplicate page slugs detected and reported as errors
- Data files and directories sharing a name merged instead of silently discarded
- Search template fetch URL not double-escaped in script context
- `parse_date_from_filename` validates dates (rejects invalid month/day)
- Image tag detection uses allocation-free byte comparison
- `clean_output_dir` prevents deletion of project root, current directory, filesystem root, home directory
- Tera filters strip HTML before counting words/reading time
- XML entity unescaping handles numeric entities
- CSS minifier preserves strings and comments correctly
- JS minifier handles template literals, regex literals, and character classes
- Shortcode parser handles escape sequences in quoted values
- Sitemap includes all content types with correct priorities

## [0.1.1] - 2026-02-04

### Features

- Add docs, portfolio, landing, changelog example sites
- Add Tailwind CSS via CDN to all templates
- Show build duration in output
- Add light/dark mode toggle and improved template styling
- Add official site with GitHub Pages deployment

### Bug Fixes

- Add `_collection.toml` for docs and changelog examples
- Fix main content padding under header
- Use `base_url` for all internal links in templates
- Use relative links in docs content
- Render collection item content in docs template
- Shorten keyword for crates.io compliance

## [0.1.0] - 2026-02-03

- Initial release
