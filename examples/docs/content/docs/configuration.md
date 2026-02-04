+++
title = "Configuration"
weight = 2
template = "docs.html"
+++

# Configuration

Configure your site using the `bamboo.toml` file in your project root.

## Basic Configuration

```toml
title = "My Site"
base_url = "https://example.com"
description = "A site built with Bamboo"
author = "Your Name"
language = "en"
```

## Custom Fields

Add any custom fields under the `[extra]` section:

```toml
[extra]
github = "https://github.com/username"
twitter = "@username"
analytics_id = "UA-XXXXX-Y"
```

These are available in templates as `{{ site.config.extra.github }}`.

## Configuration Reference

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `title` | string | yes | Site title |
| `base_url` | string | yes | Production URL |
| `description` | string | no | Site description for SEO |
| `author` | string | no | Default author name |
| `language` | string | no | Language code (default: en) |

## Environment-Specific URLs

Override the base URL when serving locally:

```bash
bamboo serve --port 8080
```

The development server automatically uses `http://localhost:<port>` as the base URL.
