use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::path::PathBuf;

use crate::images::ImageConfig;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Site {
    pub config: SiteConfig,
    pub home: Option<Page>,
    pub pages: Vec<Page>,
    pub posts: Vec<Post>,
    pub collections: HashMap<String, Collection>,
    pub data: HashMap<String, Value>,
    pub assets: Vec<Asset>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SiteConfig {
    pub title: String,
    pub base_url: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub author: Option<String>,
    #[serde(default)]
    pub language: Option<String>,
    #[serde(default = "default_posts_per_page")]
    pub posts_per_page: usize,
    #[serde(default)]
    pub minify: bool,
    #[serde(default)]
    pub fingerprint: bool,
    #[serde(default)]
    pub images: Option<ImageConfig>,
    #[serde(default)]
    pub extra: HashMap<String, Value>,
}

pub fn default_posts_per_page() -> usize {
    10
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TocEntry {
    pub level: u32,
    pub id: String,
    pub title: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Page {
    pub slug: String,
    pub title: String,
    pub content: String,
    pub raw_content: String,
    pub frontmatter: Frontmatter,
    pub path: PathBuf,
    #[serde(default)]
    pub template: Option<String>,
    #[serde(default)]
    pub weight: i32,
    #[serde(default)]
    pub word_count: usize,
    #[serde(default)]
    pub reading_time: usize,
    #[serde(default)]
    pub toc: Vec<TocEntry>,
    #[serde(default)]
    pub url: String,
    #[serde(default)]
    pub draft: bool,
    #[serde(default)]
    pub redirect_from: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Post {
    pub slug: String,
    pub title: String,
    pub date: DateTime<Utc>,
    pub content: String,
    pub raw_content: String,
    #[serde(default)]
    pub excerpt: Option<String>,
    pub frontmatter: Frontmatter,
    pub path: PathBuf,
    #[serde(default)]
    pub draft: bool,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub categories: Vec<String>,
    #[serde(default)]
    pub template: Option<String>,
    #[serde(default)]
    pub word_count: usize,
    #[serde(default)]
    pub reading_time: usize,
    #[serde(default)]
    pub toc: Vec<TocEntry>,
    #[serde(default)]
    pub url: String,
    #[serde(default)]
    pub redirect_from: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Collection {
    pub name: String,
    pub items: Vec<CollectionItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollectionItem {
    pub slug: String,
    pub title: String,
    pub content: String,
    pub raw_content: String,
    pub frontmatter: Frontmatter,
    pub path: PathBuf,
    #[serde(default)]
    pub template: Option<String>,
    #[serde(default)]
    pub weight: i32,
    #[serde(default)]
    pub word_count: usize,
    #[serde(default)]
    pub reading_time: usize,
    #[serde(default)]
    pub toc: Vec<TocEntry>,
    #[serde(default)]
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Asset {
    pub source: PathBuf,
    pub dest: PathBuf,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Frontmatter {
    #[serde(flatten)]
    pub raw: HashMap<String, Value>,
}

impl Frontmatter {
    pub fn get<T: serde::de::DeserializeOwned>(&self, key: &str) -> Option<T> {
        self.raw
            .get(key)
            .and_then(|v| serde_json::from_value(v.clone()).ok())
    }

    pub fn get_string(&self, key: &str) -> Option<String> {
        self.raw.get(key).and_then(|v| v.as_str().map(String::from))
    }

    pub fn get_bool(&self, key: &str) -> Option<bool> {
        self.raw.get(key).and_then(|v| v.as_bool())
    }

    pub fn get_i64(&self, key: &str) -> Option<i64> {
        self.raw.get(key).and_then(|v| v.as_i64())
    }

    pub fn get_array(&self, key: &str) -> Option<Vec<String>> {
        self.raw.get(key).and_then(|v| {
            v.as_array().map(|arr| {
                arr.iter()
                    .filter_map(|item| item.as_str().map(String::from))
                    .collect()
            })
        })
    }
}
