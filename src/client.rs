#[cfg(feature = "pyo3")]
use pyo3::prelude::*;

use reqwest::{Client, ClientBuilder};
use tokio::task::JoinSet;

use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
};

use crate::{
    caching::{
        CACHE_LOCK_EXT, DEFAULT_METADATA_TTL, FAMILY_CACHE_FILE, FamilyCacheInfo,
        FontListCacheInfo, default_cache_root, expires_at, now_unix, open_lock_file, parse_max_age,
    },
    error::{FontSourceError, Result},
    query::FontQuery,
    responses::FontSourceFamily,
};

#[cfg(not(test))]
const FONTSOURCE_API: &str = "https://api.fontsource.org/";

const FONTSOURCE_FONT_URL_PATH: &str = "v1/fonts/";
const FONTSOURCE_FONT_LIST_PATH: &str = "fontlist?family";

/// A client for downloading (and caching) font files from Fontsource.
#[cfg_attr(
    feature = "pyo3",
    pyclass(module = "fontsource_downloader", from_py_object)
)]
#[derive(Clone)]
pub struct FontSourceClient {
    client: Client,
    pub(crate) cache_dir: PathBuf,
}

#[cfg(feature = "pyo3")]
#[cfg_attr(feature = "pyo3", pymethods)]
impl FontSourceClient {
    /// Create a new ``FontSourceClient`` with an optional cache root directory.
    ///
    /// If ``cache_root`` is not provided, a default cache directory will be used.
    /// The default cache directory is a platform-specific location based on
    /// this package's name (and author on some platforms).
    ///
    /// Throws an ``OSError`` if the client fails to initialize the (native) TLS backend.
    #[new]
    #[pyo3(
        signature = (cache_root=None),
        text_signature = "(cache_root: Path | None = None) -> FontSourceClient"
    )]
    pub fn new_py(cache_root: Option<PathBuf>) -> PyResult<Self> {
        Self::with_cache_root(cache_root.unwrap_or_else(default_cache_root)).map_err(PyErr::from)
    }

    /// Asynchronously download a font file matching the given query.
    ///
    /// The `FontQuery.subset` field may be overridden with
    /// the family's default subset if the requested subset is
    /// not available for the `FontQuery.family`.
    ///
    /// Returns the `Path` to the downloaded font file.
    /// Throws an exception if the font could not be downloaded and/or
    /// the query is somehow invalid.
    ///
    /// This method automatically uses cached metadata and font files when available.
    /// To configure the cache location, pass a `Path` to the `FontSourceClient` constructor.
    #[pyo3(name = "download_font")]
    pub fn download_font_py<'py>(
        &self,
        py: Python<'py>,
        font: FontQuery,
    ) -> PyResult<Bound<'py, PyAny>> {
        let this = self.clone();
        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            this.download_font(&font).await.map_err(PyErr::from)
        })
    }

    /// Get the cached list of font families.
    ///
    /// Returns an error if
    ///
    /// - the cache was not previously populated with
    ///   `FontSourceClient.download_font()`
    /// - the cached JSON file was modified by external actors in a
    ///   way that causes deserialization errors.
    #[pyo3(name = "font_list_cache_info")]
    pub fn font_list_cache_info_py(&self) -> PyResult<FontListCacheInfo> {
        self.font_list_cache_info().map_err(PyErr::from)
    }

    /// Get the cache info for the specified font family.
    ///
    /// Returns an error if
    ///
    /// - the cache was not previously populated with
    ///   `FontSourceClient.download_font()`
    /// - the cached JSON file was modified by external actors in a
    ///   way that causes deserialization errors.
    #[pyo3(name = "family_cache_info")]
    pub fn family_cache_info_py(&self, id: &str) -> PyResult<FamilyCacheInfo> {
        self.family_cache_info(id).map_err(PyErr::from)
    }

    /// Generate CDN-based CSS for the requested `FontQuery` and return it as a string.
    #[pyo3(name = "css")]
    pub fn css_py<'py>(&self, py: Python<'py>, query: FontQuery) -> PyResult<Bound<'py, PyAny>> {
        let this = self.clone();
        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            this.css(&query).await.map_err(PyErr::from)
        })
    }

    /// Generate self-hosted CSS (paths relative to the client's cache) for the requested `FontQuery`.
    /// Missing files will be downloaded into the cache if necessary.
    #[pyo3(name = "css_self_hosted")]
    pub fn css_self_hosted_py<'py>(
        &self,
        py: Python<'py>,
        query: FontQuery,
        relative_url_prefix: String,
        dest: Option<PathBuf>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let this = self.clone();
        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            this.css_self_hosted(&query, relative_url_prefix.as_str(), dest.as_deref())
                .await
                .map_err(PyErr::from)
        })
    }
}

impl FontSourceClient {
    /// Create a new ``FontSourceClient`` with a default cache root directory.
    ///
    /// The default cache directory is a platform-specific location based on
    /// this package's name (and author on some platforms).
    ///
    /// Returns an `FontSourceError` if the client fails to initialize the TLS backend.
    pub fn new() -> Result<Self> {
        Self::with_cache_root(default_cache_root())
    }

    /// Create a new [`FontSourceClient`] using the given `cache_root` directory.
    ///
    /// Returns an [`FontSourceError`] if the client fails to initialize the TLS backend.
    pub fn with_cache_root<P: AsRef<Path>>(cache_root: P) -> Result<Self> {
        Ok(Self {
            client: ClientBuilder::new()
                .user_agent(concat!("fontsource-downloader/", env!("CARGO_PKG_VERSION")))
                .build()
                .map_err(|e| FontSourceError::ClientBuildFailed { source: e })?,
            cache_dir: cache_root.as_ref().to_path_buf(),
        })
    }

    /// Asynchronously download font file(s) matching the given query.
    ///
    /// The [`FontQuery::subsets`] value may be overridden with
    /// the family's default subset if the requested subsets are
    /// not available for the [`FontQuery::family`].
    ///
    /// Returns the paths to the downloaded font files.
    /// Returns a [`FontSourceError`] if the font could
    /// not be downloaded and/or the query is somehow invalid.
    ///
    /// This method automatically uses cached metadata and font files when available.
    /// To configure the cache location, instantiate the client with
    /// [`FontSourceClient::with_cache_root()`] constructor.
    pub async fn download_font(&self, query: &FontQuery) -> Result<Vec<PathBuf>> {
        let family = self.load_family_metadata(query.family()).await?;
        let subsets = family.get_variant_subsets(query)?;

        let family_cache_dir = self.family_cache_dir(&family.id);
        fs::create_dir_all(&family_cache_dir).map_err(|source| {
            FontSourceError::CreateFontCacheDirFailed {
                path: family_cache_dir.display().to_string(),
                source,
            }
        })?;
        let mut download_tasks = JoinSet::new();
        for ((weight, style, subset), urls) in &subsets {
            for (file_type, font_url) in &urls.url {
                if !query.file_type.contains(file_type) {
                    continue;
                }
                let file_name = format!("{subset}-{weight}-{style}.{}", file_type.extension());
                let font_path = family_cache_dir.join(&file_name);
                let client = self.clone();
                let font_url = font_url.to_string();
                log::debug!("Downloading font file: {}/{file_name}", family.id);
                download_tasks.spawn(async move {
                    client.download_font_file(&font_path, &font_url).await?;
                    Ok(font_path)
                });
            }
        }
        if download_tasks.is_empty() {
            return Err(FontSourceError::FontVariantNotAvailable {
                family: query.family().to_string(),
                field: "file_type",
                requested_value: query
                    .file_types()
                    .map(|t| t.extension())
                    .collect::<Vec<&str>>()
                    .join(","),
            });
        }
        Self::collect_concurrent_tasks(&mut download_tasks, "downloading requested font files")
            .await
    }

    pub(crate) async fn collect_concurrent_tasks<T: 'static>(
        tasks: &mut JoinSet<Result<T>>,
        task: &'static str,
    ) -> Result<Vec<T>> {
        let mut result = vec![];
        while let Some(joined) = tasks.join_next().await {
            let item = joined
                .map_err(|source| FontSourceError::ConcurrentTaskFailed { task, source })??;
            result.push(item);
        }
        Ok(result)
    }

    pub(crate) async fn download_font_file(&self, font_path: &Path, font_url: &str) -> Result<()> {
        // Cheap pre-lock fast path: if the file already exists, skip locking entirely.
        if font_path.exists() {
            return Ok(());
        }

        // Acquire a lock before writing the font file so concurrent processes
        // don't race to download and overwrite the same file.
        let lock_path = font_path.with_extension(CACHE_LOCK_EXT);
        let lock_file = open_lock_file(&lock_path)?;

        // Re-check inside the lock: another process may have written the file
        // while we were waiting.
        if !font_path.exists() {
            let bytes = self
                .client
                .get(font_url)
                .send()
                .await
                .map_err(|source| FontSourceError::FontDownloadFailed {
                    url: font_url.to_string(),
                    source,
                })?
                .error_for_status()
                .map_err(|source| FontSourceError::FontDownloadFailed {
                    url: font_url.to_string(),
                    source,
                })?
                .bytes()
                .await
                .map_err(|source| FontSourceError::FontDownloadFailed {
                    url: font_url.to_string(),
                    source,
                })?;
            fs::write(font_path, &bytes).map_err(|source| FontSourceError::WriteFileFailed {
                path: font_path.display().to_string(),
                source,
            })?;
        }
        lock_file
            .unlock()
            .map_err(|source| FontSourceError::CacheLockPoisoned {
                path: lock_path,
                source,
            })?;
        Ok(())
    }

    /// Load the metadata for a given font family and return it
    async fn load_family_metadata(&self, family: &str) -> Result<FontSourceFamily> {
        let Some(fam_id) = self.load_font_list_families(family).await? else {
            return Err(FontSourceError::FontFamilyNotFound {
                family: family.trim().to_string(),
            });
        };

        if let Some(cached) = self
            .family_cache_info(&fam_id)
            .ok()
            .filter(|cached| cached.expiration > now_unix())
        {
            return Ok(cached.family);
        }

        #[cfg(not(test))]
        let family_url = format!("{FONTSOURCE_API}{FONTSOURCE_FONT_URL_PATH}{fam_id}");
        #[cfg(test)]
        #[allow(clippy::unwrap_used, reason = "tests should panic on missing env var")]
        let family_url = format!(
            "{}{FONTSOURCE_FONT_URL_PATH}{fam_id}",
            std::env::var("FONTSOURCE_API").unwrap()
        );

        let detail_response = self
            .client
            .get(&family_url)
            .send()
            .await
            .map_err(|source| FontSourceError::MetadataRequestFailed {
                request: format!("getting metadata about the font {family}"),
                source,
            })?
            .error_for_status()
            .map_err(|source| FontSourceError::MetadataRequestFailed {
                request: format!("getting metadata about the font {family}"),
                source,
            })?;
        let ttl = parse_max_age(
            detail_response
                .headers()
                .get(reqwest::header::CACHE_CONTROL),
        )
        .unwrap_or(DEFAULT_METADATA_TTL);
        let metadata: FontSourceFamily = serde_json::from_str(
            detail_response
                .text()
                .await
                .map_err(|source| FontSourceError::MetadataDecodeFailed {
                    request: format!("getting metadata about the font family {family}"),
                    source,
                })?
                .as_str(),
        )
        .map_err(|e| FontSourceError::ParseResponseFailed {
            task: "getting font metadata",
            source: e,
        })?;

        let cache_payload = FamilyCacheInfo {
            expiration: expires_at(ttl),
            family: metadata,
        };
        let family_cache_path = self.family_cache_dir(&fam_id).join(FAMILY_CACHE_FILE);
        self.write_cache_json_locked(&family_cache_path, &cache_payload)?;

        Ok(cache_payload.family)
    }

    /// Load the list of font families and return the ID corresponding to the given `family` name.
    async fn load_font_list_families(&self, family: &str) -> Result<Option<String>> {
        if let Some(cached) = self
            .font_list_cache_info()
            .ok()
            .filter(|cached| cached.expiration > now_unix())
        {
            return Ok(cached.get_id_for_family(family).map(|v| v.to_string()));
        }

        #[cfg(not(test))]
        let font_list_url = format!("{FONTSOURCE_API}{FONTSOURCE_FONT_LIST_PATH}");
        #[cfg(test)]
        #[allow(clippy::unwrap_used, reason = "tests should panic on missing env var")]
        let font_list_url = format!(
            "{}{FONTSOURCE_FONT_LIST_PATH}",
            std::env::var("FONTSOURCE_API").unwrap()
        );
        let response = self
            .client
            .get(&font_list_url)
            .send()
            .await
            .map_err(|source| FontSourceError::MetadataRequestFailed {
                request: "getting the supported list of font families".to_string(),
                source,
            })?
            .error_for_status()
            .map_err(|source| FontSourceError::MetadataRequestFailed {
                request: "getting the supported list of font families".to_string(),
                source,
            })?;
        let ttl = parse_max_age(response.headers().get(reqwest::header::CACHE_CONTROL))
            .unwrap_or(DEFAULT_METADATA_TTL);
        let families: HashMap<String, String> = serde_json::from_str(
            response
                .text()
                .await
                .map_err(|source| FontSourceError::MetadataDecodeFailed {
                    request: format!(
                        "getting list of fonts to translate {family} name into font ID"
                    ),
                    source,
                })?
                .as_str(),
        )
        .map_err(|e| FontSourceError::ParseResponseFailed {
            task: "getting list of font families",
            source: e,
        })?;

        let cache_file = FontListCacheInfo {
            expiration: expires_at(ttl),
            families,
        };
        self.write_cache_json_locked(&self.font_list_cache_path(), &cache_file)?;

        Ok(cache_file.get_id_for_family(family).map(|v| v.to_string()))
    }

    /// Generate CDN-based CSS for the given font `query`.
    ///
    /// This does not download any fonts, but the requested family's metadata
    /// will be fetched if the cached metadata is expired or not present.
    ///
    /// Use [`FontSourceClient::css_self_hosted()`] if you intend to self-host fonts
    /// from the site's static resources.
    pub async fn css(&self, query: &FontQuery) -> Result<String> {
        let family = self.load_family_metadata(query.family()).await?;
        family.to_css(query, None).await
    }

    /// Return the configured cache root directory for this client.
    ///
    /// Useful for clients that use the default platform-specific cache directory.
    pub fn cache_root(&self) -> PathBuf {
        self.cache_dir.clone()
    }

    /// Generate self-hosted CSS for the given font `query`
    /// and copy font files into `dest` path.
    ///
    /// This ensures any missing files required for the requested variants are
    /// downloaded into the client's cache directory. The cached font files are then
    /// copied to given `dest` path.
    ///
    /// When done, the returned string is the CSS `@font-face` rules where
    /// URLs are relative paths rooted in the given `dest` path. The copied font
    /// files in `dest` are organized in subdirectories by family ID
    /// (e.g. `dest/roboto/latin-400-normal.ttf`).
    pub async fn css_self_hosted(
        &self,
        query: &FontQuery,
        relative_url_prefix: &str,
        dest: Option<&Path>,
    ) -> Result<String> {
        let family = self.load_family_metadata(query.family()).await?;
        family
            .to_css(query, Some((self, Path::new(relative_url_prefix), dest)))
            .await
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::panic)]

    use super::*;
    use crate::query::{FontFileType, QueryBuilder, Weight};

    // ── helpers ──────────────────────────────────────────────────────────────

    /// Load the static font-list asset, substituting `{{TTF_URL}}` with `url`.
    fn font_list_json() -> String {
        include_str!("../tests/assets/font_list.json").to_string()
    }

    /// Load the static family-metadata asset, substituting `{{TTF_URL}}` with `url`.
    fn family_metadata_json(ttf_url: &str) -> String {
        include_str!("../tests/assets/family_metadata.json").replace("{{TTF_URL}}", ttf_url)
    }

    /// Build a client whose `FONTSOURCE_API` env-var points at the mockito server.
    fn client_for(server: &mockito::Server, tmp: &tempfile::TempDir) -> FontSourceClient {
        unsafe {
            std::env::set_var("FONTSOURCE_API", server.url() + "/");
        }
        FontSourceClient::with_cache_root(tmp.path()).unwrap()
    }

    // ── basic sanity ─────────────────────────────────────────────────────────

    #[test]
    fn default_cache_dir() {
        let client = FontSourceClient::new().unwrap();
        let default_cache = default_cache_root();
        assert_eq!(client.cache_dir, default_cache);
    }

    // ── font-list errors ─────────────────────────────────────────────────────

    /// `load_font_list_families` → HTTP transport error (server hangs up).
    #[tokio::test]
    async fn font_list_request_failed() {
        let mut server = mockito::Server::new_async().await;
        let _m = server
            .mock("GET", mockito::Matcher::Any)
            .with_status(503)
            .create_async()
            .await;
        let tmp = tempfile::tempdir().unwrap();
        let client = client_for(&server, &tmp);
        let query = QueryBuilder::new("Test Family").build();
        let err = client.download_font(&query).await.unwrap_err();
        assert!(
            matches!(err, FontSourceError::MetadataRequestFailed { .. }),
            "expected MetadataRequestFailed, got {err:?}"
        );
    }

    /// `load_font_list_families` → response body is not valid JSON.
    #[tokio::test]
    async fn font_list_parse_failed() {
        let mut server = mockito::Server::new_async().await;
        // fontlist endpoint returns garbage
        let _m = server
            .mock("GET", mockito::Matcher::Regex(r"fontlist".to_string()))
            .with_status(200)
            .with_body("not json at all")
            .create_async()
            .await;
        let tmp = tempfile::tempdir().unwrap();
        let client = client_for(&server, &tmp);
        let query = QueryBuilder::new("Test Family").build();
        let err = client.download_font(&query).await.unwrap_err();
        assert!(
            matches!(
                err,
                FontSourceError::ParseResponseFailed {
                    task: "getting list of font families",
                    ..
                }
            ),
            "expected ParseResponseFailed, got {err:?}"
        );
    }

    /// `load_font_list_families` → writing the cache file fails because the
    /// cache directory is read-only (simulated by using a non-writable path).
    #[tokio::test]
    async fn font_list_cache_write_failed() {
        let mut server = mockito::Server::new_async().await;
        let _m = server
            .mock("GET", mockito::Matcher::Regex(r"fontlist".to_string()))
            .with_status(200)
            .with_body(font_list_json())
            .create_async()
            .await;
        let tmp = tempfile::tempdir().unwrap();
        unsafe {
            std::env::set_var("FONTSOURCE_API", server.url() + "/");
        }
        let client = FontSourceClient::with_cache_root(tmp.path()).unwrap();
        // Force write failure for font-list cache by making the target path a directory.
        std::fs::create_dir_all(client.font_list_cache_path()).unwrap();
        let query = QueryBuilder::new("Test Family").build();
        let err = client.download_font(&query).await.unwrap_err();
        assert!(
            matches!(err, FontSourceError::WriteFileFailed { .. }),
            "expected WriteFileFailed, got {err:?}"
        );
    }

    // ── family-not-found error ────────────────────────────────────────────────

    /// `load_family_metadata` → family name has no match in the font list.
    #[tokio::test]
    async fn family_not_found() {
        let mut server = mockito::Server::new_async().await;
        let _m = server
            .mock("GET", mockito::Matcher::Regex(r"fontlist".to_string()))
            .with_status(200)
            .with_body(font_list_json())
            .create_async()
            .await;
        let tmp = tempfile::tempdir().unwrap();
        let client = client_for(&server, &tmp);
        let query = QueryBuilder::new("Nonexistent Font").build();
        let err = client.download_font(&query).await.unwrap_err();
        assert!(
            matches!(err, FontSourceError::FontFamilyNotFound { .. }),
            "expected FontFamilyNotFound, got {err:?}"
        );
    }

    // ── family-metadata errors ────────────────────────────────────────────────

    /// `load_family_metadata` → family-detail HTTP request fails.
    #[tokio::test]
    async fn family_metadata_request_failed() {
        let mut server = mockito::Server::new_async().await;
        let _font_list = server
            .mock("GET", mockito::Matcher::Regex(r"fontlist".to_string()))
            .with_status(200)
            .with_body(font_list_json())
            .create_async()
            .await;
        let _metadata = server
            .mock("GET", mockito::Matcher::Regex(r"v1/fonts".to_string()))
            .with_status(500)
            .create_async()
            .await;
        let tmp = tempfile::tempdir().unwrap();
        let client = client_for(&server, &tmp);
        let query = QueryBuilder::new("Test Family").build();
        let err = client.download_font(&query).await.unwrap_err();
        assert!(
            matches!(err, FontSourceError::MetadataRequestFailed { .. }),
            "expected MetadataRequestFailed, got {err:?}"
        );
    }

    /// `load_family_metadata` → family-detail response body is not valid JSON.
    #[tokio::test]
    async fn family_metadata_parse_failed() {
        let mut server = mockito::Server::new_async().await;
        let _font_list = server
            .mock("GET", mockito::Matcher::Regex(r"fontlist".to_string()))
            .with_status(200)
            .with_body(font_list_json())
            .create_async()
            .await;
        let _metadata = server
            .mock("GET", mockito::Matcher::Regex(r"v1/fonts".to_string()))
            .with_status(200)
            .with_body("not json")
            .create_async()
            .await;
        let tmp = tempfile::tempdir().unwrap();
        let client = client_for(&server, &tmp);
        let query = QueryBuilder::new("Test Family").build();
        let err = client.download_font(&query).await.unwrap_err();
        assert!(
            matches!(
                err,
                FontSourceError::ParseResponseFailed {
                    task: "getting font metadata",
                    ..
                }
            ),
            "expected ParseResponseFailed (font metadata), got {err:?}"
        );
    }

    // ── weight / style / subset validation errors ─────────────────────────────

    async fn setup_mocks_for_download(server: &mut mockito::Server, ttf_url: &str) {
        server
            .mock("GET", mockito::Matcher::Regex(r"fontlist".to_string()))
            .with_status(200)
            .with_body(font_list_json())
            .create_async()
            .await;
        server
            .mock("GET", mockito::Matcher::Regex(r"v1/fonts".to_string()))
            .with_status(200)
            .with_body(family_metadata_json(ttf_url))
            .create_async()
            .await;
    }

    /// `download_font` → requested weight is not available.
    #[tokio::test]
    async fn weight_not_available() {
        let mut server = mockito::Server::new_async().await;
        setup_mocks_for_download(&mut server, "http://example.com/dummy.ttf").await;
        let tmp = tempfile::tempdir().unwrap();
        let client = client_for(&server, &tmp);
        let query = QueryBuilder::new("Test Family")
            .with_weight(Weight::Bold) // only 400 in the asset
            .build();
        let Err(FontSourceError::FontVariantNotAvailable {
            family: _,
            field,
            requested_value,
        }) = client.download_font(&query).await
        else {
            panic!("expected FontVariantNotAvailable");
        };
        assert_eq!(field, "weight");
        assert_eq!(requested_value.as_str(), "700");
    }

    /// `download_font` → requested style is not available.
    #[tokio::test]
    async fn style_not_available() {
        let mut server = mockito::Server::new_async().await;
        setup_mocks_for_download(&mut server, "http://example.com/dummy.ttf").await;
        let tmp = tempfile::tempdir().unwrap();
        let client = client_for(&server, &tmp);
        let query = QueryBuilder::new("Test Family")
            .with_style("italic") // only "normal" in the asset
            .build();
        let Err(FontSourceError::FontVariantNotAvailable {
            family: _,
            field,
            requested_value,
        }) = client.download_font(&query).await
        else {
            panic!("expected FontVariantNotAvailable");
        };
        assert_eq!(field, "style");
        assert_eq!(requested_value.as_str(), "italic");
    }

    /// `download_font` -> requesting only unavailable file formats returns an error.
    #[tokio::test]
    async fn format_not_available() {
        let mut server = mockito::Server::new_async().await;
        // Fixture metadata currently only provides `ttf` URLs.
        setup_mocks_for_download(&mut server, "http://example.com/dummy.ttf").await;
        let tmp = tempfile::tempdir().unwrap();
        let client = client_for(&server, &tmp);
        let query = QueryBuilder::new("Test Family")
            .with_file_type(FontFileType::Woff)
            .build();
        let Err(FontSourceError::FontVariantNotAvailable {
            family,
            field,
            requested_value,
        }) = client.download_font(&query).await
        else {
            panic!("expected FontVariantNotAvailable");
        };
        assert_eq!(family, "Test Family");
        assert_eq!(field, "file_type");
        assert_eq!(requested_value.as_str(), "woff");
    }

    /// `download_font` → font cache dir cannot be created (cache_dir is a file).
    #[tokio::test]
    async fn create_cache_dir_failed() {
        let mut server = mockito::Server::new_async().await;
        let ttf_url = format!("{}/dummy.ttf", server.url());
        // Keep font-list ID different from metadata ID so metadata caching succeeds
        // at one path while download_font() tries creating another blocked path.
        let _font_list = server
            .mock("GET", mockito::Matcher::Regex(r"fontlist".to_string()))
            .with_status(200)
            .with_body(r#"{"cache-safe-id":"Test Family"}"#)
            .create_async()
            .await;
        let metadata = format!(
            r#"{{
                "id": "blocked-id",
                "family": "Test Family",
                "subsets": ["latin"],
                "weights": [400],
                "styles": ["normal"],
                "defSubset": "latin",
                "variants": {{
                    "400": {{
                        "normal": {{
                            "latin": {{ "url": {{ "ttf": "{ttf_url}" }} }}
                        }}
                    }}
                }}
            }}"#
        );
        let _metadata = server
            .mock("GET", mockito::Matcher::Regex(r"v1/fonts".to_string()))
            .with_status(200)
            .with_body(metadata)
            .create_async()
            .await;
        let tmp = tempfile::tempdir().unwrap();
        let families_root = tmp.path().join("families");
        std::fs::create_dir_all(&families_root).unwrap();
        // Block the exact directory download_font() will try to create.
        std::fs::write(families_root.join("blocked-id"), b"").unwrap();
        unsafe {
            std::env::set_var("FONTSOURCE_API", server.url() + "/");
        }
        let client = FontSourceClient::with_cache_root(tmp.path()).unwrap();
        let query = QueryBuilder::new("Test Family").build();
        let err = client.download_font(&query).await.unwrap_err();
        assert!(
            matches!(err, FontSourceError::CreateFontCacheDirFailed { .. }),
            "expected CreateFontCacheDirFailed, got {err:?}"
        );
    }

    /// `download_font` → font TTF download fails (server returns 404).
    #[tokio::test]
    async fn font_download_failed() {
        let mut server = mockito::Server::new_async().await;
        let ttf_url = format!("{}/dummy.ttf", server.url());
        setup_mocks_for_download(&mut server, &ttf_url).await;
        // TTF endpoint returns 404
        let _ttf = server
            .mock("GET", "/dummy.ttf")
            .with_status(404)
            .create_async()
            .await;
        let tmp = tempfile::tempdir().unwrap();
        let client = client_for(&server, &tmp);
        let query = QueryBuilder::new("Test Family").build();
        let err = client.download_font(&query).await.unwrap_err();
        assert!(
            matches!(err, FontSourceError::FontDownloadFailed { .. }),
            "expected FontDownloadFailed, got {err:?}"
        );
    }

    /// `download_font` → font subset present in `variants` key is absent for the
    /// requested (valid) subset — triggers `FontSubsetNotAvailable`.
    ///
    /// We do this by providing a family where `subsets` lists an extra entry
    /// ("cyrillic") but `variants` has no matching subtree.
    #[tokio::test]
    async fn subset_not_available_in_variants() {
        let mut server = mockito::Server::new_async().await;
        let _font_list = server
            .mock("GET", mockito::Matcher::Regex(r"fontlist".to_string()))
            .with_status(200)
            .with_body(font_list_json())
            .create_async()
            .await;
        // Family declares "cyrillic" in subsets, but variants only has "latin".
        let metadata = r#"{
            "id": "test-family",
            "family": "Test Family",
            "subsets": ["latin", "cyrillic"],
            "weights": [400],
            "styles": ["normal"],
            "defSubset": "latin",
            "variants": {
                "400": {
                    "normal": {
                        "latin": { "url": { "ttf": "http://example.com/dummy.ttf" } }
                    }
                }
            }
        }"#;
        let _metadata_mock = server
            .mock("GET", mockito::Matcher::Regex(r"v1/fonts".to_string()))
            .with_status(200)
            .with_body(metadata)
            .create_async()
            .await;
        let tmp = tempfile::tempdir().unwrap();
        let client = client_for(&server, &tmp);
        let query = QueryBuilder::new("Test Family")
            .with_subset("cyrillic")
            .build();
        let Err(FontSourceError::FontVariantNotAvailable {
            family: _,
            field,
            requested_value,
        }) = client.download_font(&query).await
        else {
            panic!("expected FontVariantNotAvailable");
        };
        assert_eq!(field, "subset");
        assert_eq!(requested_value.as_str(), "cyrillic");
    }

    /// Second call to `download_font` for the same font returns the cached path
    /// (exercises the pre-lock fast path).
    #[tokio::test]
    async fn cache_hit_fast_path() {
        let mut server = mockito::Server::new_async().await;
        let ttf_url = format!("{}/dummy.ttf", server.url());
        setup_mocks_for_download(&mut server, &ttf_url).await;
        let _ttf = server
            .mock("GET", "/dummy.ttf")
            .with_status(200)
            .with_body(b"FAKE_TTF")
            .create_async()
            .await;
        let tmp = tempfile::tempdir().unwrap();
        let client = client_for(&server, &tmp);
        let query = QueryBuilder::new("Test Family")
            .with_subset("french") // should be overridden by default_subset "latin"
            .build();
        let first = client.download_font(&query).await.unwrap();
        let query = QueryBuilder::new("Test Family").build();
        // Second call should hit the fast path (file already exists).
        let second = client.download_font(&query).await.unwrap();
        assert_eq!(first, second);
    }

    /// Cached font-list and family-metadata files are reused when not expired.
    #[tokio::test]
    async fn cached_metadata_used_on_second_call() {
        let mut server = mockito::Server::new_async().await;
        let ttf_url = format!("{}/dummy.ttf", server.url());
        // Each mock is set up to only match once.
        let _font_list = server
            .mock("GET", mockito::Matcher::Regex(r"fontlist".to_string()))
            .with_status(200)
            .with_body(font_list_json())
            .expect(1)
            .create_async()
            .await;
        let _metadata = server
            .mock("GET", mockito::Matcher::Regex(r"v1/fonts".to_string()))
            .with_status(200)
            .with_body(family_metadata_json(&ttf_url))
            .expect(1)
            .create_async()
            .await;
        let _ttf = server
            .mock("GET", "/dummy.ttf")
            .with_status(200)
            .with_body(b"FAKE_TTF")
            .expect(1)
            .create_async()
            .await;
        let tmp = tempfile::tempdir().unwrap();
        let client = client_for(&server, &tmp);
        let query = QueryBuilder::new("Test Family").build();
        client.download_font(&query).await.unwrap();
        // Second call uses cache – mocks would panic if hit again.
        client.download_font(&query).await.unwrap();
    }

    /// `collect_concurrent_tasks` maps a Tokio join failure into `ConcurrentTaskFailed`.
    #[tokio::test]
    async fn concurrent_task_failed_error() {
        let mut tasks = JoinSet::new();
        tasks.spawn(async {
            panic!("simulated task panic");
            #[allow(unreachable_code)]
            Ok(())
        });

        let err = FontSourceClient::collect_concurrent_tasks(&mut tasks, "test join set")
            .await
            .unwrap_err();

        assert!(
            matches!(err, FontSourceError::ConcurrentTaskFailed { .. }),
            "expected ConcurrentTaskFailed, got {err:?}"
        );
    }
}
