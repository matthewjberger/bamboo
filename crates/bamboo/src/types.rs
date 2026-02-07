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
pub struct Content {
    pub slug: String,
    pub title: String,
    #[serde(rename = "content")]
    pub html: String,
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
pub struct Page {
    #[serde(flatten)]
    pub content: Content,
    #[serde(default)]
    pub draft: bool,
    #[serde(default)]
    pub redirect_from: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Post {
    #[serde(flatten)]
    pub content: Content,
    pub date: DateTime<Utc>,
    #[serde(default)]
    pub excerpt: Option<String>,
    #[serde(default)]
    pub draft: bool,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub categories: Vec<String>,
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
    #[serde(flatten)]
    pub content: Content,
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
        self.raw.get(key).and_then(|value| {
            if let Some(string) = value.as_str() {
                Some(string.to_string())
            } else {
                eprintln!(
                    "Warning: frontmatter key '{}' expected string, got {}",
                    key, value
                );
                None
            }
        })
    }

    pub fn get_bool(&self, key: &str) -> Option<bool> {
        self.raw.get(key).and_then(|value| {
            if let Some(boolean) = value.as_bool() {
                Some(boolean)
            } else {
                eprintln!(
                    "Warning: frontmatter key '{}' expected bool, got {}",
                    key, value
                );
                None
            }
        })
    }

    pub fn get_i64(&self, key: &str) -> Option<i64> {
        self.raw.get(key).and_then(|value| {
            if let Some(integer) = value.as_i64() {
                Some(integer)
            } else {
                eprintln!(
                    "Warning: frontmatter key '{}' expected integer, got {}",
                    key, value
                );
                None
            }
        })
    }

    pub fn get_array(&self, key: &str) -> Option<Vec<String>> {
        self.raw.get(key).and_then(|value| {
            if let Some(array) = value.as_array() {
                Some(
                    array
                        .iter()
                        .filter_map(|item| item.as_str().map(String::from))
                        .collect(),
                )
            } else {
                eprintln!(
                    "Warning: frontmatter key '{}' expected array, got {}",
                    key, value
                );
                None
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frontmatter_with(key: &str, value: Value) -> Frontmatter {
        let mut raw = HashMap::new();
        raw.insert(key.to_string(), value);
        Frontmatter { raw }
    }

    #[test]
    fn test_get_string_valid() {
        let frontmatter = frontmatter_with("title", Value::String("Hello".to_string()));
        assert_eq!(frontmatter.get_string("title"), Some("Hello".to_string()));
    }

    #[test]
    fn test_get_string_missing() {
        let frontmatter = Frontmatter::default();
        assert_eq!(frontmatter.get_string("title"), None);
    }

    #[test]
    fn test_get_string_wrong_type() {
        let frontmatter = frontmatter_with("title", Value::Bool(true));
        assert_eq!(frontmatter.get_string("title"), None);
    }

    #[test]
    fn test_get_bool_valid() {
        let frontmatter = frontmatter_with("draft", Value::Bool(true));
        assert_eq!(frontmatter.get_bool("draft"), Some(true));
    }

    #[test]
    fn test_get_bool_missing() {
        let frontmatter = Frontmatter::default();
        assert_eq!(frontmatter.get_bool("draft"), None);
    }

    #[test]
    fn test_get_bool_wrong_type() {
        let frontmatter = frontmatter_with("draft", Value::String("true".to_string()));
        assert_eq!(frontmatter.get_bool("draft"), None);
    }

    #[test]
    fn test_get_i64_valid() {
        let frontmatter = frontmatter_with("weight", serde_json::json!(42));
        assert_eq!(frontmatter.get_i64("weight"), Some(42));
    }

    #[test]
    fn test_get_i64_wrong_type() {
        let frontmatter = frontmatter_with("weight", Value::String("42".to_string()));
        assert_eq!(frontmatter.get_i64("weight"), None);
    }

    #[test]
    fn test_get_array_valid() {
        let frontmatter = frontmatter_with("tags", serde_json::json!(["rust", "web"]));
        assert_eq!(
            frontmatter.get_array("tags"),
            Some(vec!["rust".to_string(), "web".to_string()])
        );
    }

    #[test]
    fn test_get_array_wrong_type() {
        let frontmatter = frontmatter_with("tags", Value::String("rust".to_string()));
        assert_eq!(frontmatter.get_array("tags"), None);
    }

    #[test]
    fn test_get_generic() {
        let frontmatter = frontmatter_with("count", serde_json::json!(5));
        assert_eq!(frontmatter.get::<i64>("count"), Some(5));
    }

    #[test]
    fn test_default_posts_per_page() {
        assert_eq!(default_posts_per_page(), 10);
    }
}
