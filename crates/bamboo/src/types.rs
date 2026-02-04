use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::path::PathBuf;

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
    #[serde(default)]
    pub extra: HashMap<String, Value>,
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
