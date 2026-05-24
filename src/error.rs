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

    /// Error when the requested font variant is not available for the requested family.
    #[error("Fontsource family '{family}' does not provide {field} {requested_value}")]
    FontVariantNotAvailable {
        /// The font family for which the requested weight is not available.
        family: String,
        /// The field of the font variant for which the requested weight is not available.
        ///
        /// E.g. weight, style, subset
        field: &'static str,
        /// The requested value (as a string) that is not available for the requested family.
        requested_value: String,
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

    /// Error when a spawned concurrent task fails to complete.
    #[error("Failed while running concurrent task for {task}")]
    ConcurrentTaskFailed {
        /// A description of the concurrent operation.
        task: &'static str,
        /// The underlying task join failure from Tokio.
        #[source]
        source: tokio::task::JoinError,
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

    /// Error when reading cache files.
    #[error("Failed to read cache file '{path}'")]
    ReadCacheFileFailed {
        /// The path of the cache file that failed to read.
        path: PathBuf,
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

    /// Error when deserializing cache JSON files.
    #[error("Failed to deserialize JSON file from cache")]
    JsonError(#[from] serde_json::Error),

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

#[cfg(feature = "pyo3")]
impl From<FontSourceError> for pyo3::PyErr {
    fn from(value: FontSourceError) -> pyo3::PyErr {
        use pyo3::exceptions::*;

        match &value {
            FontSourceError::ClientBuildFailed { source: _ } => {
                PyOSError::new_err(format!("{value:?}"))
            }
            FontSourceError::MetadataRequestFailed {
                request: _,
                source: _,
            } => PyRuntimeError::new_err(format!("{value:?}")),
            FontSourceError::MetadataDecodeFailed {
                request: _,
                source: _,
            } => PyValueError::new_err(format!("{value:?}")),
            FontSourceError::FontFamilyNotFound { family: _ } => {
                PyValueError::new_err(format!("{value:?}"))
            }
            FontSourceError::FontVariantNotAvailable {
                family: _,
                field: _,
                requested_value: _,
            } => PyValueError::new_err(format!("{value:?}")),
            FontSourceError::CreateFontCacheDirFailed { path: _, source: _ } => {
                PyOSError::new_err(format!("{value:?}"))
            }
            FontSourceError::FontDownloadFailed { url: _, source: _ } => {
                PyRuntimeError::new_err(format!("{value:?}"))
            }
            FontSourceError::ConcurrentTaskFailed { task: _, source: _ } => {
                PyRuntimeError::new_err(format!("{value:?}"))
            }
            FontSourceError::WriteFileFailed { path: _, source: _ } => {
                PyOSError::new_err(format!("{value:?}"))
            }
            FontSourceError::ReadCacheFileFailed { path: _, source: _ } => {
                PyOSError::new_err(format!("{value:?}"))
            }
            FontSourceError::ParseResponseFailed { task: _, source: _ } => {
                PyRuntimeError::new_err(format!("{value:?}"))
            }
            FontSourceError::JsonError(_) => PyValueError::new_err(format!("{value:?}")),
            FontSourceError::CacheLockPoisoned { path: _, source: _ } => {
                PyOSError::new_err(format!("{value:?}"))
            }
        }
    }
}

/// A convenient alias for results returned by FontSource client operations.
pub type Result<T> = std::result::Result<T, FontSourceError>;
