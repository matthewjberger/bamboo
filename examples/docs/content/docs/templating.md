+++
title = "Templating"
weight = 4
template = "docs.html"
+++

# Templating

Bamboo uses Tera for templating, a Jinja2-inspired template engine.

## Template Inheritance

Create a base template and extend it:

```html
<!-- base.html -->
<!DOCTYPE html>
<html>
<head>
  <title>{% block title %}{{ site.config.title }}{% endblock %}</title>
</head>
<body>
  {% block content %}{% endblock %}
</body>
</html>
```

```html
<!-- page.html -->
{% extends "base.html" %}

{% block title %}{{ page.title }} | {{ site.config.title }}{% endblock %}

{% block content %}
<article>
  <h1>{{ page.title }}</h1>
  {{ page.content | safe }}
</article>
{% endblock %}
```

## Template Context

### All Templates

```
site.config.title
site.config.base_url
site.config.extra.*
site.pages
site.posts
site.collections
site.data
```

### Page Templates

```
page.title
page.content
page.slug
page.path
```

### Post Templates

```
post.title
post.content
post.date
post.tags
post.excerpt
post.slug
```

## Filters

Tera provides many built-in filters:

```html
{{ post.date | date(format="%B %d, %Y") }}
{{ post.title | upper }}
{{ post.content | safe }}
{{ items | length }}
```

## Includes

Include partial templates:

```html
{% include "partials/header.html" %}
```
