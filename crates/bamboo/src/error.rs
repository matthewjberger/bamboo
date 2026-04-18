//! The [`BambooError`] type and the crate-wide [`Result`] alias.

use std::path::{Path, PathBuf};
use thiserror::Error;

/// Every error produced by the bamboo-ssg pipeline, from parsing
/// `bamboo.toml` through rendering the final HTML.
#[derive(Error, Debug)]
pub enum BambooError {
    /// A raw filesystem error with no path context attached. Prefer
    /// [`BambooError::IoPath`] via [`IoContext::io_context`] when possible.
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// A filesystem operation failed on a specific path.
    #[error("{operation} '{path}': {source}")]
    IoPath {
        /// Short description of what was being attempted (e.g. `"read"`,
        /// `"write"`, `"create_dir_all"`).
        operation: &'static str,
        /// Path the operation targeted.
        path: PathBuf,
        /// Underlying I/O error.
        source: std::io::Error,
    },

    /// TOML frontmatter or data file failed to parse.
    #[error("TOML parse error in {path}: {message}")]
    TomlParse {
        /// Path of the offending file.
        path: PathBuf,
        /// Parser message (line/column info if the parser provided it).
        message: String,
    },

    /// YAML frontmatter or data file failed to parse.
    #[error("YAML parse error in {path}: {message}")]
    YamlParse {
        /// Path of the offending file.
        path: PathBuf,
        /// Parser message.
        message: String,
    },

    /// JSON data file failed to parse.
    #[error("JSON parse error in {path}: {message}")]
    JsonParse {
        /// Path of the offending file.
        path: PathBuf,
        /// Parser message.
        message: String,
    },

    /// Tera failed to compile or render a template.
    #[error("Template error: {0}")]
    Template(#[from] tera::Error),

    /// A content file's frontmatter block was malformed (unclosed delimiter,
    /// unrecognized format, etc.).
    #[error("Invalid frontmatter in file: {path}")]
    InvalidFrontmatter {
        /// Path of the offending file.
        path: PathBuf,
    },

    /// A required frontmatter field was absent.
    #[error("Missing required field '{field}' in file: {path}")]
    MissingField {
        /// Name of the missing field.
        field: String,
        /// Path of the content file.
        path: PathBuf,
    },

    /// A date frontmatter field or filename prefix couldn't be parsed.
    #[error("Invalid date format in file: {path}")]
    InvalidDate {
        /// Path of the offending file.
        path: PathBuf,
    },

    /// No `bamboo.toml` found at the expected location.
    #[error("Config file not found: {path}")]
    ConfigNotFound {
        /// Path that was probed.
        path: PathBuf,
    },

    /// The requested theme directory doesn't exist or isn't a valid theme.
    #[error("Theme not found: {name}")]
    ThemeNotFound {
        /// Theme name as supplied by the caller (directory path or built-in
        /// theme name).
        name: String,
    },

    /// A path couldn't be normalized into a form bamboo could work with
    /// (non-UTF-8, escapes the project root, etc.).
    #[error("Invalid path: {path}")]
    InvalidPath {
        /// The offending path.
        path: PathBuf,
    },

    /// `walkdir` failed to traverse a directory.
    #[error("Directory walk error in {path}: {message}")]
    WalkDir {
        /// Directory being walked when the failure occurred.
        path: PathBuf,
        /// Underlying walkdir message.
        message: String,
    },

    /// A shortcode tag couldn't be parsed (unclosed delimiter, malformed
    /// arguments, etc.).
    #[error("Shortcode parse error: {message}")]
    ShortcodeParse {
        /// Parser message.
        message: String,
    },

    /// A shortcode template failed to render.
    #[error("Shortcode render error in '{name}': {message}")]
    ShortcodeRender {
        /// Shortcode name.
        name: String,
        /// Underlying Tera error message.
        message: String,
    },

    /// An image in the responsive-image pipeline couldn't be decoded,
    /// resized, or re-encoded.
    #[error("Image processing error: {message}")]
    ImageProcessing {
        /// Underlying image error.
        message: String,
    },

    /// Sass/SCSS compilation failed.
    #[error("Sass compilation error in {path}: {message}")]
    SassCompile {
        /// Path of the offending stylesheet.
        path: PathBuf,
        /// Compiler message.
        message: String,
    },

    /// A `{{< ref "..." >}}` shortcode references a page that doesn't exist.
    #[error("Broken reference '{{{{< ref \"{reference}\" >}}}}': no page found matching that path")]
    BrokenReference {
        /// The reference string as written in the shortcode.
        reference: String,
    },

    /// Two content files resolved to the same output URL.
    #[error("Duplicate page slug '{slug}' in {path} conflicts with {existing_path}")]
    DuplicatePage {
        /// The conflicting slug.
        slug: String,
        /// Path of the file being processed.
        path: PathBuf,
        /// Path of the file that already claimed the slug.
        existing_path: PathBuf,
    },
}

/// Convenience alias for `Result<T, BambooError>` used throughout the crate.
pub type Result<T> = std::result::Result<T, BambooError>;

/// Attaches filesystem-operation context to a bare `io::Result`, producing a
/// [`BambooError::IoPath`] with the operation name and target path.
pub trait IoContext<T> {
    /// Wraps an `Err(io::Error)` into [`BambooError::IoPath`]. `operation`
    /// should be a short verb describing the attempt (e.g. `"read"`).
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
