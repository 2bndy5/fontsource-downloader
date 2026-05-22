#[cfg(feature = "pyo3")]
use pyo3::prelude::*;

use std::{
    collections::HashMap,
    fs::{self, File},
    path::{Path, PathBuf},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use directories::ProjectDirs;
use serde::{Deserialize, Serialize};

use crate::{
    client::FontSourceClient,
    error::{FontSourceError, Result},
    responses::FontSourceFamily,
};

pub(crate) const DEFAULT_METADATA_TTL: Duration = Duration::from_hours(24);
const FONT_LIST_CACHE_FILE: &str = "fontsource-family-list-cache.json";
pub(crate) const FAMILY_CACHE_FILE: &str = "family-metadata-cache.json";
pub(crate) const CACHE_LOCK_EXT: &str = "lock";

/// The cached list of font families.
#[cfg_attr(
    feature = "pyo3",
    pyclass(module = "fontsource_downloader", get_all, frozen)
)]
#[derive(Debug, Serialize, Deserialize)]
pub struct FontListCacheInfo {
    /// The Unix timestamp (in seconds) when this cache entry expires.
    #[serde(alias = "expires_at_unix")]
    pub expiration: u64,
    /// A mapping of family IDs to their display names.
    pub families: HashMap<String, String>,
}

#[cfg_attr(feature = "pyo3", pymethods)]
impl FontListCacheInfo {
    /// Get the family ID for a given family name.
    ///
    /// Traverses the mapping's values to find the first matching key,
    /// where values are the `family` display name and keys are their corresponding ID.
    pub fn get_id_for_family<'a>(&'a self, family: &str) -> Option<&'a str> {
        let family = family.trim();
        for (id, name) in &self.families {
            if name == family {
                return Some(id.as_str());
            }
        }
        None
    }
}

/// The cached metadata for a single font family.
#[cfg_attr(
    feature = "pyo3",
    pyclass(module = "fontsource_downloader", get_all, frozen)
)]
#[derive(Debug, Serialize, Deserialize)]
pub struct FamilyCacheInfo {
    /// The Unix timestamp (in seconds) when this cache entry expires.
    #[serde(alias = "expires_at_unix")]
    pub expiration: u64,
    /// The metadata for the font family.
    pub family: FontSourceFamily,
}

impl FontSourceClient {
    /// Get the path to the cache directory for font families.
    ///
    /// This directory shall contain all cached fonts for each family in subdirectories.
    pub(crate) fn families_cache_path(&self) -> PathBuf {
        self.cache_dir.join("families")
    }

    /// Get the path to the cache directory for a specific font family.
    pub(crate) fn family_cache_dir(&self, family_id: &str) -> PathBuf {
        self.families_cache_path().join(family_id)
    }

    /// Get the path to the cache file for the list of font families.
    pub(crate) fn font_list_cache_path(&self) -> PathBuf {
        self.cache_dir.join(FONT_LIST_CACHE_FILE)
    }

    /// Get the cached list of font families.
    ///
    /// Returns an error if
    ///
    /// - the cache was not previously populated with
    ///   [`FontSourceClient::download_font()`]
    /// - the cached JSON file was modified by external actors in a
    ///   way that causes deserialization errors.
    pub fn font_list_cache_info(&self) -> Result<FontListCacheInfo> {
        let cache_path = self.font_list_cache_path();
        let raw = fs::read_to_string(&cache_path).map_err(|source| {
            FontSourceError::ReadCacheFileFailed {
                path: cache_path,
                source,
            }
        })?;
        let cache = serde_json::from_str(&raw)?;
        Ok(cache)
    }

    /// Get the cached metadata for a specific font family.
    ///
    /// Returns an error if
    ///
    /// - the cache was not previously populated with
    ///   [`FontSourceClient::download_font()`]
    /// - the cached JSON file was modified by external actors in a
    ///   way that causes deserialization errors.
    pub fn family_cache_info(&self, family_id: &str) -> Result<FamilyCacheInfo> {
        let cache_path = self.family_cache_dir(family_id).join(FAMILY_CACHE_FILE);
        let raw = fs::read_to_string(&cache_path).map_err(|source| {
            FontSourceError::ReadCacheFileFailed {
                path: cache_path,
                source,
            }
        })?;
        let cache = serde_json::from_str(&raw)?;
        Ok(cache)
    }

    pub(crate) fn write_cache_json_locked<T: Serialize>(
        &self,
        path: &Path,
        value: &T,
    ) -> Result<()> {
        let parent = path
            .parent()
            .ok_or_else(|| FontSourceError::CreateFontCacheDirFailed {
                path: path.display().to_string(),
                source: std::io::Error::other("cache path has no parent"),
            })?;
        fs::create_dir_all(parent).map_err(|source| FontSourceError::CreateFontCacheDirFailed {
            path: parent.display().to_string(),
            source,
        })?;

        let lock_path = path.with_extension(CACHE_LOCK_EXT);
        let lock_file = open_lock_file(&lock_path)?;

        let serialized =
            serde_json::to_vec(value).map_err(|source| FontSourceError::WriteFileFailed {
                path: path.display().to_string(),
                source: std::io::Error::other(source),
            })?;
        fs::write(path, &serialized).map_err(|source| FontSourceError::WriteFileFailed {
            path: path.display().to_string(),
            source,
        })?;

        lock_file
            .unlock()
            .map_err(|source| FontSourceError::CacheLockPoisoned {
                path: lock_path,
                source,
            })
    }
}

pub(crate) fn open_lock_file(lock_path: &Path) -> Result<File> {
    let lock_file = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(lock_path)
        .map_err(|source| FontSourceError::WriteFileFailed {
            path: lock_path.display().to_string(),
            source,
        })?;
    lock_file
        .lock()
        .map_err(|source| FontSourceError::CacheLockPoisoned {
            path: lock_path.to_path_buf(),
            source,
        })?;
    Ok(lock_file)
}

pub(crate) fn expires_at(ttl: Duration) -> u64 {
    now_unix().saturating_add(ttl.as_secs())
}

pub(crate) fn parse_max_age(
    cache_control: Option<&reqwest::header::HeaderValue>,
) -> Option<Duration> {
    let cache_control = cache_control?.to_str().ok()?;
    for part in cache_control.split(',') {
        let directive = part.trim();
        if let Some(seconds) = directive.strip_prefix("max-age=")
            && let Ok(parsed) = seconds.parse::<u64>()
        {
            return Some(Duration::from_secs(parsed));
        }
    }
    None
}

pub(crate) fn default_cache_root() -> PathBuf {
    if let Some(dirs) = ProjectDirs::from("", "2bndy5", "fontsource-downloader") {
        dirs.cache_dir().join("fonts")
    } else {
        std::env::temp_dir()
            .join("fontsource-downloader")
            .join("fonts")
    }
}

pub(crate) fn now_unix() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_secs())
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use std::{collections::HashMap, fs, time::Duration};

    use serde::Serialize;

    use super::*;

    struct FailingSerialize;

    impl Serialize for FailingSerialize {
        fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
        where
            S: serde::Serializer,
        {
            let _ = serializer;
            Err(serde::ser::Error::custom("value is not serializable"))
        }
    }

    fn test_client(cache_root: &std::path::Path) -> FontSourceClient {
        FontSourceClient::with_cache_root(cache_root).unwrap()
    }

    #[test]
    fn parse_max_age_variants() {
        assert_eq!(parse_max_age(None), None);

        let valid = reqwest::header::HeaderValue::from_static("public, max-age=123, immutable");
        assert_eq!(parse_max_age(Some(&valid)), Some(Duration::from_secs(123)));

        let non_numeric = reqwest::header::HeaderValue::from_static("max-age=abc");
        assert_eq!(parse_max_age(Some(&non_numeric)), None);

        let invalid_utf8 = reqwest::header::HeaderValue::from_bytes(b"max-age=10\xFF").unwrap();
        assert_eq!(parse_max_age(Some(&invalid_utf8)), None);
    }

    #[test]
    fn expires_at_adds_ttl() {
        let before = now_unix();
        let expires = expires_at(Duration::from_secs(2));
        let after = now_unix();

        assert!(expires >= before + 2);
        assert!(expires <= after + 2);
    }

    #[test]
    fn default_cache_root_ends_with_fonts() {
        let root = default_cache_root();
        assert_eq!(root.file_name().unwrap(), "fonts");
    }

    #[test]
    fn open_lock_file_open_failure_maps_to_write_error() {
        let temp_dir = tempfile::tempdir().unwrap();
        let lock_path = temp_dir.path().join("missing-parent").join("file.lock");

        let err = open_lock_file(&lock_path).unwrap_err();
        assert!(matches!(
            err,
            FontSourceError::WriteFileFailed { path, .. } if path == lock_path.display().to_string()
        ));
    }

    #[test]
    fn write_cache_json_locked_writes_and_creates_lock() {
        let temp_dir = tempfile::tempdir().unwrap();
        let client = test_client(temp_dir.path());
        let target = temp_dir.path().join("nested").join("font-list.json");

        let mut families = HashMap::new();
        families.insert("roboto".to_string(), "Roboto".to_string());
        let cache = FontListCacheInfo {
            expiration: 42,
            families,
        };

        client.write_cache_json_locked(&target, &cache).unwrap();

        let raw = fs::read_to_string(&target).unwrap();
        let parsed: FontListCacheInfo = serde_json::from_str(&raw).unwrap();
        assert_eq!(parsed.expiration, 42);
        assert_eq!(parsed.families.get("roboto").unwrap(), "Roboto");
        assert!(target.with_extension(CACHE_LOCK_EXT).exists());
    }

    #[test]
    fn write_cache_json_locked_errors_when_parent_is_file() {
        let temp_dir = tempfile::tempdir().unwrap();
        let blocker = temp_dir.path().join("blocker");
        fs::write(&blocker, b"x").unwrap();
        let target = blocker.join("cache.json");
        let client = test_client(temp_dir.path());

        let err = client
            .write_cache_json_locked(&target, &serde_json::json!({"ok": true}))
            .unwrap_err();

        assert!(matches!(
            err,
            FontSourceError::CreateFontCacheDirFailed { path, .. } if path == blocker.display().to_string()
        ));
    }

    #[test]
    fn write_cache_json_locked_errors_when_path_has_no_parent() {
        let temp_dir = tempfile::tempdir().unwrap();
        let client = test_client(temp_dir.path());

        let err = client
            .write_cache_json_locked(std::path::Path::new(""), &serde_json::json!({"ok": true}))
            .unwrap_err();

        assert!(matches!(
            err,
            FontSourceError::CreateFontCacheDirFailed { path, .. } if path.is_empty()
        ));
    }

    #[test]
    fn write_cache_json_locked_errors_for_non_serializable_value() {
        let temp_dir = tempfile::tempdir().unwrap();
        let client = test_client(temp_dir.path());
        let target = temp_dir.path().join("cache.json");

        let err = client
            .write_cache_json_locked(&target, &FailingSerialize)
            .unwrap_err();

        assert!(matches!(
            err,
            FontSourceError::WriteFileFailed { path, .. } if path == target.display().to_string()
        ));
    }

    #[test]
    fn write_cache_json_locked_errors_when_target_is_directory() {
        let temp_dir = tempfile::tempdir().unwrap();
        let client = test_client(temp_dir.path());
        let target_dir = temp_dir.path().join("as-dir");
        fs::create_dir(&target_dir).unwrap();

        let err = client
            .write_cache_json_locked(&target_dir, &serde_json::json!({"ok": true}))
            .unwrap_err();

        assert!(matches!(
            err,
            FontSourceError::WriteFileFailed { path, .. } if path == target_dir.display().to_string()
        ));
    }

    #[test]
    fn helper_paths_are_composed_from_cache_dir() {
        let temp_dir = tempfile::tempdir().unwrap();
        let client = test_client(temp_dir.path());

        assert_eq!(
            client.families_cache_path(),
            temp_dir.path().join("families")
        );
        assert_eq!(
            client.font_list_cache_path(),
            temp_dir.path().join(FONT_LIST_CACHE_FILE)
        );
        assert_eq!(
            client.family_cache_dir("test-family"),
            temp_dir.path().join("families").join("test-family")
        );
    }
}
