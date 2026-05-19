//! Error types for FontSource client operations.

use std::path::PathBuf;

/// Errors that can occur during FontSource client operations.
#[derive(Debug, thiserror::Error)]
pub enum FontSourceError {
    /// Error when the HTTP client fails to initialize.
    #[error("Failed to build HTTP client for Fontsource")]
    ClientBuildFailed {
        /// The underlying error from the HTTP client builder.
        #[source]
        source: reqwest::Error,
    },

    /// Error when an HTTP request is unsuccessful.
    #[error("Failed to query Fontsource when {request}")]
    MetadataRequestFailed {
        /// A description of the request that failed to complete successfully.
        request: String,
        /// The underlying error from the HTTP client.
        #[source]
        source: reqwest::Error,
    },

    /// Errors when decoding Fontsource metadata responses into text.
    #[error("Failed to decode Fontsource metadata when {request}")]
    MetadataDecodeFailed {
        /// A description of the request for which the response payload that failed to decode into text.
        request: String,
        /// The underlying error when decoding the payload.
        #[source]
        source: reqwest::Error,
    },

    /// Error when the requested font family is not found in Fontsource.
    #[error("Fontsource does not provide a family named '{family}'")]
    FontFamilyNotFound {
        /// The requested font family that was not found.
        family: String,
    },

    /// Error when the requested font weight is not available for the requested family.
    #[error("Fontsource family '{family}' does not provide weight {weight}")]
    FontWeightNotAvailable {
        /// The font family for which the requested weight is not available.
        family: String,
        /// The requested weight that is not available for the requested family.
        weight: u16,
    },

    /// Error when the requested font style is not available for the requested family.
    #[error("Fontsource family '{family}' does not provide style '{style}'")]
    FontStyleNotAvailable {
        /// The font family for which the requested style is not available.
        family: String,
        /// The requested style that is not available for the requested family.
        style: String,
    },

    /// Error when the requested font subset is not available for the requested family.
    #[error("Fontsource family '{family}' does not provide subset '{subset}'")]
    FontSubsetNotAvailable {
        /// The font family for which the requested subset is not available.
        family: String,
        /// The requested subset that is not available for the requested family.
        subset: String,
    },

    /// Error when failing to create a cache directory.
    #[error("Failed to create font cache directory '{path}'")]
    CreateFontCacheDirFailed {
        /// The path of the cache directory that failed to create.
        path: String,
        /// The underlying error from the file system.
        #[source]
        source: std::io::Error,
    },

    /// Errors when downloading font files.
    #[error("Failed to download font file from '{url}'")]
    FontDownloadFailed {
        /// The URL of the font file that failed to download.
        url: String,
        /// The underlying error from the HTTP client.
        #[source]
        source: reqwest::Error,
    },

    /// Error when writing files (in cache directory)
    #[error("Failed to write file '{path}'")]
    WriteFileFailed {
        /// The path of the file that failed to write.
        path: String,
        /// The underlying error from the file system.
        #[source]
        source: std::io::Error,
    },

    /// Error when parsing Fontsource API responses.
    #[error("Failed to parse response payload about {task} from Fontsource")]
    ParseResponseFailed {
        /// A description of the request for which the response payload that failed to parse.
        task: &'static str,
        /// The underlying error from the JSON parser.
        #[source]
        source: serde_json::Error,
    },

    /// Error when the cache lock file is poisoned.
    ///
    /// This can occur when multiple processes acquire a lock on the same cache file.
    #[error("Cache lock file is poisoned for '{path}'")]
    CacheLockPoisoned {
        /// The path of the cache's lock file
        path: PathBuf,
        /// The underlying error from the file system.
        #[source]
        source: std::io::Error,
    },
}

/// A convenient alias for results returned by FontSource client operations.
pub type Result<T> = std::result::Result<T, FontSourceError>;
