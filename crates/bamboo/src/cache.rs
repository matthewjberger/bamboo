use crate::error::Result;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

const CACHE_DIR_NAME: &str = ".bamboo-cache";
const CACHE_FILE_NAME: &str = "build-state.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildState {
    pub content_hashes: HashMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChangeClassification {
    Full,
    Targeted { changed_files: Vec<PathBuf> },
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum RenderTarget {
    Page(String),
    Post(String),
    Collection(String),
    Pagination,
    AllTaxonomies,
    Feeds,
    Sitemap,
    SearchIndex,
    All,
}

pub fn load_cache(project_dir: &Path) -> Option<BuildState> {
    let cache_path = project_dir.join(CACHE_DIR_NAME).join(CACHE_FILE_NAME);
    let content = fs::read_to_string(cache_path).ok()?;
    serde_json::from_str(&content).ok()
}

pub fn save_cache(project_dir: &Path, state: &BuildState) -> Result<()> {
    let cache_dir = project_dir.join(CACHE_DIR_NAME);
    fs::create_dir_all(&cache_dir)?;
    let cache_path = cache_dir.join(CACHE_FILE_NAME);
    let content = serde_json::to_string_pretty(state)
        .map_err(|error| std::io::Error::other(error.to_string()))?;
    fs::write(cache_path, content)?;
    Ok(())
}

pub fn compute_content_hashes(input_dir: &Path) -> Result<HashMap<String, String>> {
    let mut hashes = HashMap::new();

    let dirs_to_hash = ["content", "data", "static", "templates"];
    for dir_name in &dirs_to_hash {
        let dir = input_dir.join(dir_name);
        if dir.exists() {
            hash_directory(&dir, input_dir, &mut hashes)?;
        }
    }

    let config_path = input_dir.join("bamboo.toml");
    if config_path.exists() {
        let hash = hash_file(&config_path)?;
        let relative = config_path
            .strip_prefix(input_dir)
            .unwrap_or(&config_path)
            .to_string_lossy()
            .replace('\\', "/");
        hashes.insert(relative, hash);
    }

    Ok(hashes)
}

fn hash_file(path: &Path) -> Result<String> {
    let content = fs::read(path)?;
    let mut hasher = Sha256::new();
    hasher.update(&content);
    let result = hasher.finalize();
    Ok(format!("{:x}", result))
}

fn hash_directory(
    directory: &Path,
    base_dir: &Path,
    hashes: &mut HashMap<String, String>,
) -> Result<()> {
    for entry in WalkDir::new(directory) {
        let entry = entry.map_err(|error| crate::error::BambooError::WalkDir {
            path: directory.to_path_buf(),
            message: error.to_string(),
        })?;

        if !entry.file_type().is_file() {
            continue;
        }

        let path = entry.path();
        let hash = hash_file(path)?;
        let relative = path
            .strip_prefix(base_dir)
            .unwrap_or(path)
            .to_string_lossy()
            .replace('\\', "/");
        hashes.insert(relative, hash);
    }

    Ok(())
}

pub fn classify_changes(
    old_hashes: &HashMap<String, String>,
    new_hashes: &HashMap<String, String>,
) -> ChangeClassification {
    let mut changed_files = Vec::new();

    for (path, new_hash) in new_hashes {
        match old_hashes.get(path) {
            Some(old_hash) if old_hash == new_hash => {}
            _ => {
                changed_files.push(PathBuf::from(path));
            }
        }
    }

    let has_deletions = old_hashes.keys().any(|path| !new_hashes.contains_key(path));

    if changed_files.is_empty() && !has_deletions {
        return ChangeClassification::Targeted {
            changed_files: vec![],
        };
    }

    let has_config_change = changed_files
        .iter()
        .any(|path| path.to_string_lossy() == "bamboo.toml");

    let has_template_change = changed_files
        .iter()
        .any(|path| path.to_string_lossy().starts_with("templates/"));

    if has_config_change || has_template_change || has_deletions {
        return ChangeClassification::Full;
    }

    ChangeClassification::Targeted { changed_files }
}

pub fn expand_targets(classification: &ChangeClassification) -> HashSet<RenderTarget> {
    match classification {
        ChangeClassification::Full => {
            let mut targets = HashSet::new();
            targets.insert(RenderTarget::All);
            targets
        }
        ChangeClassification::Targeted { changed_files } => {
            let mut targets = HashSet::new();

            if changed_files.is_empty() {
                return targets;
            }

            for path in changed_files {
                let path_str = path.to_string_lossy().replace('\\', "/");

                if path_str.starts_with("content/posts/") {
                    let filename = path
                        .file_name()
                        .map(|name| name.to_string_lossy().to_string())
                        .unwrap_or_default();
                    let slug = extract_post_slug(&filename);
                    targets.insert(RenderTarget::Post(slug));
                    targets.insert(RenderTarget::Pagination);
                    targets.insert(RenderTarget::Feeds);
                    targets.insert(RenderTarget::Sitemap);
                    targets.insert(RenderTarget::SearchIndex);
                    targets.insert(RenderTarget::AllTaxonomies);
                    targets.insert(RenderTarget::Page("index".to_string()));
                } else if let Some(relative) = path_str.strip_prefix("content/") {
                    let components: Vec<&str> = relative.split('/').collect();

                    if components.len() >= 2 {
                        targets.insert(RenderTarget::Collection(components[0].to_string()));
                    } else {
                        let filename = components[0];
                        let slug = filename.strip_suffix(".md").unwrap_or(filename);
                        if slug == "_index" {
                            targets.insert(RenderTarget::Page("index".to_string()));
                        } else {
                            targets.insert(RenderTarget::Page(slug.to_string()));
                        }
                    }
                    targets.insert(RenderTarget::Sitemap);
                    targets.insert(RenderTarget::SearchIndex);
                } else if path_str.starts_with("static/") || path_str.starts_with("data/") {
                    targets.insert(RenderTarget::All);
                }
            }

            targets
        }
    }
}

fn extract_post_slug(filename: &str) -> String {
    let without_extension = filename.strip_suffix(".md").unwrap_or(filename);

    if let Some((_date_part, slug)) = crate::parsing::parse_date_from_filename(without_extension) {
        slug
    } else {
        without_extension.to_string()
    }
}

pub fn should_render(targets: &HashSet<RenderTarget>, target: &RenderTarget) -> bool {
    if targets.contains(&RenderTarget::All) {
        return true;
    }
    targets.contains(target)
}

pub fn should_render_any_post(targets: &HashSet<RenderTarget>) -> bool {
    if targets.contains(&RenderTarget::All) {
        return true;
    }
    targets
        .iter()
        .any(|target| matches!(target, RenderTarget::Post(_)))
}

pub fn should_render_any_page(targets: &HashSet<RenderTarget>) -> bool {
    if targets.contains(&RenderTarget::All) {
        return true;
    }
    targets
        .iter()
        .any(|target| matches!(target, RenderTarget::Page(_)))
}

pub fn should_render_any_collection(targets: &HashSet<RenderTarget>) -> bool {
    if targets.contains(&RenderTarget::All) {
        return true;
    }
    targets
        .iter()
        .any(|target| matches!(target, RenderTarget::Collection(_)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_save_and_load_cache() {
        let dir = TempDir::new().unwrap();
        let state = BuildState {
            content_hashes: HashMap::from([
                ("content/about.md".to_string(), "abc123".to_string()),
                ("bamboo.toml".to_string(), "def456".to_string()),
            ]),
        };

        save_cache(dir.path(), &state).unwrap();
        let loaded = load_cache(dir.path()).unwrap();

        assert_eq!(loaded.content_hashes.len(), 2);
        assert_eq!(
            loaded.content_hashes.get("content/about.md").unwrap(),
            "abc123"
        );
    }

    #[test]
    fn test_load_cache_nonexistent() {
        let dir = TempDir::new().unwrap();
        assert!(load_cache(dir.path()).is_none());
    }

    #[test]
    fn test_compute_content_hashes() {
        let dir = TempDir::new().unwrap();
        fs::create_dir_all(dir.path().join("content")).unwrap();
        fs::write(dir.path().join("bamboo.toml"), "title = \"Test\"").unwrap();
        fs::write(dir.path().join("content/about.md"), "about page").unwrap();

        let hashes = compute_content_hashes(dir.path()).unwrap();

        assert!(hashes.contains_key("bamboo.toml"));
        assert!(hashes.contains_key("content/about.md"));
        assert_eq!(hashes.len(), 2);
    }

    #[test]
    fn test_compute_content_hashes_deterministic() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("bamboo.toml"), "title = \"Test\"").unwrap();

        let hashes1 = compute_content_hashes(dir.path()).unwrap();
        let hashes2 = compute_content_hashes(dir.path()).unwrap();

        assert_eq!(hashes1.get("bamboo.toml"), hashes2.get("bamboo.toml"));
    }

    #[test]
    fn test_classify_changes_no_changes() {
        let hashes = HashMap::from([("file.md".to_string(), "abc".to_string())]);
        let classification = classify_changes(&hashes, &hashes);

        assert_eq!(
            classification,
            ChangeClassification::Targeted {
                changed_files: vec![]
            }
        );
    }

    #[test]
    fn test_classify_changes_content_change() {
        let old = HashMap::from([("content/about.md".to_string(), "abc".to_string())]);
        let new = HashMap::from([("content/about.md".to_string(), "def".to_string())]);

        let classification = classify_changes(&old, &new);

        match classification {
            ChangeClassification::Targeted { changed_files } => {
                assert_eq!(changed_files.len(), 1);
                assert_eq!(changed_files[0], PathBuf::from("content/about.md"));
            }
            ChangeClassification::Full => panic!("expected Targeted"),
        }
    }

    #[test]
    fn test_classify_changes_config_change() {
        let old = HashMap::from([("bamboo.toml".to_string(), "abc".to_string())]);
        let new = HashMap::from([("bamboo.toml".to_string(), "def".to_string())]);

        let classification = classify_changes(&old, &new);
        assert_eq!(classification, ChangeClassification::Full);
    }

    #[test]
    fn test_classify_changes_template_change() {
        let old = HashMap::from([("templates/base.html".to_string(), "abc".to_string())]);
        let new = HashMap::from([("templates/base.html".to_string(), "def".to_string())]);

        let classification = classify_changes(&old, &new);
        assert_eq!(classification, ChangeClassification::Full);
    }

    #[test]
    fn test_classify_changes_new_file() {
        let old = HashMap::new();
        let new = HashMap::from([("content/new.md".to_string(), "abc".to_string())]);

        let classification = classify_changes(&old, &new);
        match classification {
            ChangeClassification::Targeted { changed_files } => {
                assert_eq!(changed_files.len(), 1);
            }
            ChangeClassification::Full => panic!("expected Targeted"),
        }
    }

    #[test]
    fn test_classify_changes_deleted_file() {
        let old = HashMap::from([("content/old.md".to_string(), "abc".to_string())]);
        let new = HashMap::new();

        let classification = classify_changes(&old, &new);
        assert_eq!(classification, ChangeClassification::Full);
    }

    #[test]
    fn test_expand_targets_full() {
        let targets = expand_targets(&ChangeClassification::Full);
        assert!(targets.contains(&RenderTarget::All));
    }

    #[test]
    fn test_expand_targets_empty() {
        let targets = expand_targets(&ChangeClassification::Targeted {
            changed_files: vec![],
        });
        assert!(targets.is_empty());
    }

    #[test]
    fn test_expand_targets_post_change() {
        let targets = expand_targets(&ChangeClassification::Targeted {
            changed_files: vec![PathBuf::from("content/posts/2024-01-15-hello.md")],
        });

        assert!(targets.contains(&RenderTarget::Post("hello".to_string())));
        assert!(targets.contains(&RenderTarget::Pagination));
        assert!(targets.contains(&RenderTarget::Feeds));
        assert!(targets.contains(&RenderTarget::Sitemap));
        assert!(targets.contains(&RenderTarget::SearchIndex));
        assert!(targets.contains(&RenderTarget::AllTaxonomies));
        assert!(targets.contains(&RenderTarget::Page("index".to_string())));
    }

    #[test]
    fn test_expand_targets_page_change() {
        let targets = expand_targets(&ChangeClassification::Targeted {
            changed_files: vec![PathBuf::from("content/about.md")],
        });

        assert!(targets.contains(&RenderTarget::Page("about".to_string())));
        assert!(targets.contains(&RenderTarget::Sitemap));
        assert!(targets.contains(&RenderTarget::SearchIndex));
        assert!(!targets.contains(&RenderTarget::Pagination));
    }

    #[test]
    fn test_expand_targets_collection_change() {
        let targets = expand_targets(&ChangeClassification::Targeted {
            changed_files: vec![PathBuf::from("content/docs/intro.md")],
        });

        assert!(targets.contains(&RenderTarget::Collection("docs".to_string())));
        assert!(targets.contains(&RenderTarget::Sitemap));
        assert!(targets.contains(&RenderTarget::SearchIndex));
        assert!(!targets.contains(&RenderTarget::Pagination));
    }

    #[test]
    fn test_expand_targets_static_change() {
        let targets = expand_targets(&ChangeClassification::Targeted {
            changed_files: vec![PathBuf::from("static/style.css")],
        });

        assert!(targets.contains(&RenderTarget::All));
    }

    #[test]
    fn test_should_render_with_all() {
        let mut targets = HashSet::new();
        targets.insert(RenderTarget::All);

        assert!(should_render(
            &targets,
            &RenderTarget::Page("about".to_string())
        ));
        assert!(should_render(
            &targets,
            &RenderTarget::Post("hello".to_string())
        ));
        assert!(should_render(&targets, &RenderTarget::Pagination));
    }

    #[test]
    fn test_should_render_specific() {
        let mut targets = HashSet::new();
        targets.insert(RenderTarget::Page("about".to_string()));

        assert!(should_render(
            &targets,
            &RenderTarget::Page("about".to_string())
        ));
        assert!(!should_render(
            &targets,
            &RenderTarget::Page("contact".to_string())
        ));
        assert!(!should_render(
            &targets,
            &RenderTarget::Post("hello".to_string())
        ));
    }

    #[test]
    fn test_extract_post_slug_with_date() {
        assert_eq!(extract_post_slug("2024-01-15-hello.md"), "hello");
    }

    #[test]
    fn test_extract_post_slug_without_date() {
        assert_eq!(extract_post_slug("my-post.md"), "my-post");
    }

    #[test]
    fn test_should_render_any_post() {
        let mut targets = HashSet::new();
        targets.insert(RenderTarget::Post("hello".to_string()));
        targets.insert(RenderTarget::Pagination);

        assert!(should_render_any_post(&targets));
        assert!(!should_render_any_page(&targets));
    }

    #[test]
    fn test_should_render_any_page() {
        let mut targets = HashSet::new();
        targets.insert(RenderTarget::Page("about".to_string()));

        assert!(should_render_any_page(&targets));
        assert!(!should_render_any_post(&targets));
    }
}
