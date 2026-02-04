+++
title = "Getting Started"
weight = 1
template = "docs.html"
+++

# Getting Started

Learn how to install Bamboo and create your first site.

## Installation

Install Bamboo using Cargo:

```bash
cargo install bamboo-cli
```

## Create a New Site

Use the `new` command to scaffold a new site:

```bash
bamboo new my-site
cd my-site
```

This creates a new directory with the following structure:

```
my-site/
├── bamboo.toml
├── content/
│   ├── _index.md
│   ├── about.md
│   └── posts/
│       └── 2024-01-01-hello-world.md
├── data/
└── static/
    └── images/
```

## Development Server

Start the development server with live reload:

```bash
bamboo serve
```

Open http://localhost:3000 to see your site. Edit any file and the browser will automatically refresh.

## Build for Production

Build your site for production:

```bash
bamboo build
```

The output is written to `dist/` by default. Deploy these files to any static hosting provider.
