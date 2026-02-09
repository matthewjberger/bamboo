use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum BambooError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("{operation} '{path}': {source}")]
    IoPath {
        operation: &'static str,
        path: PathBuf,
        source: std::io::Error,
    },

    #[error("TOML parse error in {path}: {message}")]
    TomlParse { path: PathBuf, message: String },

    #[error("YAML parse error in {path}: {message}")]
    YamlParse { path: PathBuf, message: String },

    #[error("JSON parse error in {path}: {message}")]
    JsonParse { path: PathBuf, message: String },

    #[error("Template error: {0}")]
    Template(#[from] tera::Error),

    #[error("Invalid frontmatter in file: {path}")]
    InvalidFrontmatter { path: PathBuf },

    #[error("Missing required field '{field}' in file: {path}")]
    MissingField { field: String, path: PathBuf },

    #[error("Invalid date format in file: {path}")]
    InvalidDate { path: PathBuf },

    #[error("Config file not found: {path}")]
    ConfigNotFound { path: PathBuf },

    #[error("Theme not found: {name}")]
    ThemeNotFound { name: String },

    #[error("Invalid path: {path}")]
    InvalidPath { path: PathBuf },

    #[error("Directory walk error in {path}: {message}")]
    WalkDir { path: PathBuf, message: String },

    #[error("Shortcode parse error: {message}")]
    ShortcodeParse { message: String },

    #[error("Shortcode render error in '{name}': {message}")]
    ShortcodeRender { name: String, message: String },

    #[error("Image processing error: {message}")]
    ImageProcessing { message: String },

    #[error("Sass compilation error in {path}: {message}")]
    SassCompile { path: PathBuf, message: String },

    #[error("Broken reference '{{{{< ref \"{reference}\" >}}}}': no page found matching that path")]
    BrokenReference { reference: String },

    #[error("Duplicate page slug '{slug}' in {path} conflicts with {existing_path}")]
    DuplicatePage {
        slug: String,
        path: PathBuf,
        existing_path: PathBuf,
    },
}

pub type Result<T> = std::result::Result<T, BambooError>;

pub trait IoContext<T> {
    fn io_context(self, operation: &'static str, path: &Path) -> Result<T>;
}

impl<T> IoContext<T> for std::result::Result<T, std::io::Error> {
    fn io_context(self, operation: &'static str, path: &Path) -> Result<T> {
        self.map_err(|source| BambooError::IoPath {
            operation,
            path: path.to_path_buf(),
            source,
        })
    }
}
