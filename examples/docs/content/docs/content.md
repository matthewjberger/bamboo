+++
title = "Content"
weight = 3
template = "docs.html"
+++

# Content

Write content using Markdown with frontmatter metadata.

## Frontmatter

Every content file starts with frontmatter in TOML or YAML format.

### TOML Frontmatter

```markdown
+++
title = "My Page"
date = 2024-01-15
tags = ["rust", "web"]
draft = false
+++

Your content here...
```

### YAML Frontmatter

```markdown
---
title: My Page
date: 2024-01-15
tags:
  - rust
  - web
---

Your content here...
```

## Frontmatter Fields

| Field | Type | Description |
|-------|------|-------------|
| `title` | string | Page title (required) |
| `date` | date | Publication date |
| `draft` | bool | Exclude from build |
| `tags` | array | Post tags |
| `weight` | number | Sort order (lower first) |
| `template` | string | Override template |

## Markdown Features

Bamboo supports CommonMark with extensions:

- **Tables** - GitHub-flavored tables
- **Footnotes** - Reference-style footnotes
- **Strikethrough** - ~~deleted text~~
- **Task lists** - `- [x] completed`
- **Syntax highlighting** - Fenced code blocks with language

## Content Organization

```
content/
├── _index.md           # Home page
├── about.md            # /about/
├── posts/              # Blog posts
│   └── 2024-01-15-hello.md
└── projects/           # Collection
    └── my-project.md
```
