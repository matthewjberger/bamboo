+++
title = "Bamboo"
template = "slideshow.html"
+++

<!-- Slide 1 -->
<section class="slide">
<h1>Bamboo</h1>
<p class="subtitle">A fast static site generator written in Rust</p>
</section>

<!-- Slide 2 -->
<section class="slide">
<h2>Features</h2>
<ul>
<li>Markdown with TOML/YAML frontmatter</li>
<li>Syntax highlighting</li>
<li>Tera templating</li>
<li>Live reload dev server</li>
<li>RSS & Sitemap generation</li>
</ul>
</section>

<!-- Slide 3 -->
<section class="slide">
<h2>Quick Start</h2>
<pre><code>bamboo new my-site
cd my-site
bamboo serve</code></pre>
</section>

<!-- Slide 4 -->
<section class="slide">
<h2>Project Structure</h2>
<pre><code>my-site/
├── bamboo.toml
├── content/
│   ├── _index.md
│   └── posts/
├── data/
└── static/</code></pre>
</section>

<!-- Slide 5 -->
<section class="slide">
<h2>Frontmatter</h2>
<pre><code>+++
title = "My Post"
tags = ["rust", "web"]
draft = false
+++

Your content here...</code></pre>
</section>

<!-- Slide 6 -->
<section class="slide">
<h2>Why Bamboo?</h2>
<ul>
<li>Fast builds</li>
<li>Simple configuration</li>
<li>Flexible theming</li>
<li>Great developer experience</li>
</ul>
</section>

<!-- Slide 7 -->
<section class="slide">
<h1>Thank You</h1>
<p class="subtitle">github.com/matthewjberger/bamboo</p>
</section>
