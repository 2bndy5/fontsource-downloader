#[cfg(feature = "pyo3")]
use pyo3::prelude::*;

use std::{collections::HashMap, fs, path::Path};

use serde::{Deserialize, Serialize};
use tokio::task::JoinSet;

use crate::{
    FontSourceClient,
    error::{FontSourceError, Result},
    query::{FontFileType, FontQuery},
};

/// The metadata for a single font family.
#[cfg_attr(
    feature = "pyo3",
    pyclass(module = "fontsource_downloader", frozen, from_py_object)
)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FontSourceFamily {
    /// The unique identifier for the font family (e.g., "roboto").
    #[cfg(feature = "pyo3")]
    #[pyo3(get)]
    pub id: String,
    /// The unique identifier for the font family (e.g., "roboto").
    #[cfg(not(feature = "pyo3"))]
    pub id: String,

    /// The display name of the font family (e.g., "Roboto").
    #[cfg(feature = "pyo3")]
    #[pyo3(get)]
    pub family: String,
    /// The display name of the font family (e.g., "Roboto").
    #[cfg(not(feature = "pyo3"))]
    pub family: String,

    /// The list of available subsets for this font family (e.g., ["latin", "latin-ext"]).
    #[cfg(feature = "pyo3")]
    #[pyo3(get)]
    pub subsets: Vec<String>,
    /// The list of available subsets for this font family (e.g., ["latin", "latin-ext"]).
    #[cfg(not(feature = "pyo3"))]
    pub subsets: Vec<String>,

    /// The list of available weights for this font family (e.g., [400, 700]).
    #[cfg(feature = "pyo3")]
    #[pyo3(get)]
    pub weights: Vec<u16>,
    /// The list of available weights for this font family (e.g., [400, 700]).
    #[cfg(not(feature = "pyo3"))]
    pub weights: Vec<u16>,

    /// The list of available styles for this font family (e.g., ["normal", "italic"]).
    #[cfg(feature = "pyo3")]
    #[pyo3(get)]
    pub styles: Vec<String>,
    /// The list of available styles for this font family (e.g., ["normal", "italic"]).
    #[cfg(not(feature = "pyo3"))]
    pub styles: Vec<String>,

    /// The default subset for this font family (e.g., "latin").
    #[cfg(feature = "pyo3")]
    #[pyo3(get)]
    #[serde(rename = "defSubset")]
    pub default_subset: String,
    /// The default subset for this font family (e.g., "latin").
    #[cfg(not(feature = "pyo3"))]
    #[serde(rename = "defSubset")]
    pub default_subset: String,

    /// Unicode ranges for each supported subset.
    #[serde(default, rename = "unicodeRange")]
    pub(crate) unicode_range: HashMap<String, String>,

    /// Whether this font family supports variable fonts.
    #[serde(default)]
    pub(crate) variable: bool,

    /// The mapping of font variants (weight, style, subset) to their corresponding URLs.
    #[serde(default)]
    pub(crate) variants: FontSourceVariants,
}

impl FontSourceFamily {
    /// Returns a map of (weight, style, subset) to `FontSourceVariantSubset` for the variants matching the query.
    pub(crate) fn get_variant_subsets<'a>(
        &'a self,
        query: &'a FontQuery,
    ) -> Result<HashMap<(u16, &'a str, &'a str), &'a FontSourceVariantSubset>> {
        let mut result = HashMap::default();

        let mut subsets = query.filter_subsets(&self.subsets);
        if subsets.is_empty() {
            subsets.push(&self.default_subset);
        }

        let weights = query.filter_weights(&self.weights);
        if weights.is_empty() {
            return Err(FontSourceError::FontVariantNotAvailable {
                family: query.family().to_string(),
                field: "weight",
                requested_value: query
                    .weights()
                    .map(|v| u16::from(v).to_string())
                    .collect::<Vec<String>>()
                    .join(", "),
            });
        }
        let styles = query.filter_styles(&self.styles);
        if styles.is_empty() {
            return Err(FontSourceError::FontVariantNotAvailable {
                family: query.family().to_string(),
                field: "style",
                requested_value: query
                    .styles()
                    .map(|v| v.to_string())
                    .collect::<Vec<String>>()
                    .join(", "),
            });
        }
        for weight in weights {
            let var_weight =
                self.variants
                    .weight(weight)
                    .ok_or(FontSourceError::FontVariantNotAvailable {
                        family: query.family().to_string(),
                        field: "weight",
                        requested_value: weight.to_string(),
                    })?;
            for style in &styles {
                let var_style =
                    var_weight
                        .style(style)
                        .ok_or(FontSourceError::FontVariantNotAvailable {
                            family: query.family().to_string(),
                            field: "style",
                            requested_value: style.to_string(),
                        })?;
                for subset in &subsets {
                    let var_subset = var_style.subset(subset).ok_or(
                        FontSourceError::FontVariantNotAvailable {
                            family: self.family.clone(),
                            field: "subset",
                            requested_value: subset.to_string(),
                        },
                    )?;
                    result.insert((weight, style.as_str(), subset.as_str()), var_subset);
                }
            }
        }
        Ok(result)
    }

    /// Synthesize `@font-face` CSS for the given `query` using the URLs
    /// available in this family's metadata.
    ///
    /// The optional `client_dest` tuple shall include:
    /// - index 0: The `FontSourceClient` to use for downloading font files
    /// - index 1: A relative path prefix. The family ID is always appended to this prefix
    ///   when building URL paths (e.g., `""` -> `"roboto/font-file"`, `".."` -> `"../roboto/font-file"`).
    /// - index 2: An optional path to copy downloaded font files to.
    ///   If omitted, files are not copied.
    ///
    /// If `client_dest` is not provided, metadata URLs (CDN) are used directly.
    pub(crate) async fn to_css(
        &self,
        query: &FontQuery,
        client_dest: Option<(&FontSourceClient, &Path, Option<&Path>)>,
    ) -> Result<String> {
        let mut result = String::new();
        let mut download_tasks = JoinSet::new();
        let var_subsets = self.get_variant_subsets(query)?;
        let client_dest = client_dest.map(|(c, p, d)| (c, p.join(&self.id), d));

        // Build a map of available formats -> URL and delegate formatting.
        for ((weight, style, subset), urls) in &var_subsets {
            let urls: HashMap<&FontFileType, &String> = urls
                .url
                .iter()
                .filter(|(k, _)| query.file_type.contains(*k))
                .collect();
            let self_hosted = if let Some((client, rel_prefix, dest)) = &client_dest {
                if let Some(dest) = dest {
                    let family_cache_root = client.family_cache_dir(&self.id);
                    for (font_type, font_url) in &urls {
                        let file_name =
                            format!("{subset}-{weight}-{style}.{}", font_type.extension());
                        let font_path = family_cache_root.join(&file_name);
                        let dest = dest.to_path_buf();
                        let client = (*client).to_owned();
                        let font_url = (*font_url).clone();
                        download_tasks.spawn(async move {
                            client.download_font_file(&font_path, &font_url).await?;
                            fs::copy(&font_path, &dest).map_err(|source| {
                                FontSourceError::WriteFileFailed {
                                    path: dest.to_string_lossy().to_string(),
                                    source,
                                }
                            })?;
                            Ok(())
                        });
                    }
                }
                Some(((*weight, *style, *subset), rel_prefix.as_path()))
            } else {
                None
            };
            let css_urls = css_url_map(&urls, self_hosted).join(",\n    ");
            let mut css = format!(
                r#"
@font-face {{
  font-family: '{}';
  font-style: {style};
  font-weight: {weight};
  src:
    {css_urls};"#,
                self.family,
            );
            if let Some(range) = self.unicode_range.get(*subset) {
                css.push_str(&format!("\n  unicode-range: {range};"));
            }
            css.push_str("\n}\n");
            result.push_str(&css);
        }

        FontSourceClient::collect_concurrent_tasks(
            &mut download_tasks,
            "downloading and copying self-hosted font files",
        )
        .await?;

        Ok(result)
    }
}

/// The mapping of font weights to their corresponding styles for a font family.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(transparent)]
pub struct FontSourceVariants(HashMap<u16, FontSourceStyles>);

impl FontSourceVariants {
    fn weight(&self, weight: u16) -> Option<&FontSourceStyles> {
        self.0.get(&weight)
    }
}

/// The mapping of font styles to their corresponding subsets for a font family.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(transparent)]
pub struct FontSourceStyles(HashMap<String, FontSourceSubsets>);

impl FontSourceStyles {
    fn style(&self, style: &str) -> Option<&FontSourceSubsets> {
        self.0.get(style)
    }
}

/// The mapping for a specific font variant's subsets to their corresponding download URL.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(transparent)]
pub struct FontSourceSubsets(HashMap<String, FontSourceVariantSubset>);

impl FontSourceSubsets {
    fn subset(&self, subset: &str) -> Option<&FontSourceVariantSubset> {
        self.0.get(subset)
    }
}

/// A specific font variant's download URLs.
///
/// A variant may not include all file formats.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FontSourceVariantSubset {
    pub url: HashMap<FontFileType, String>,
}

/// Returns a list of CSS `src` entries for this variant subset.
///
/// The optional `self_hosted` tuple shall include:
/// - index 0: The weight, style, and subset to construct the file name (e.g., "latin-400-normal.woff2").
/// - index 1: The relative path prefix used to construct a self-hosted URL (e.g., "../roboto").
///
/// If `self_hosted` is not provided, the original URLs from the metadata (pointing to fontsource's CDN)
/// will be used instead.
pub fn css_url_map(
    map: &HashMap<&FontFileType, &String>,
    self_hosted: Option<((u16, &str, &str), &Path)>,
) -> Vec<String> {
    let mut src_parts = Vec::new();
    if let Some(u) = map.get(&FontFileType::Woff2)
        && !u.is_empty()
    {
        match &self_hosted {
            Some(((weight, style, subset), path_prefix)) => {
                let file_name = format!(
                    "{subset}-{weight}-{style}.{}",
                    FontFileType::Woff2.extension()
                );
                let full_path = path_prefix.join(&file_name);
                src_parts.push(format!(
                    r#"url("{}") format("woff2")"#,
                    full_path.to_string_lossy().replace("\\", "/")
                ));
            }
            None => src_parts.push(format!(r#"url("{u}") format("woff2")"#)),
        }
    }
    if let Some(u) = map.get(&FontFileType::Woff)
        && !u.is_empty()
    {
        match &self_hosted {
            Some(((weight, style, subset), path_prefix)) => {
                let file_name = format!(
                    "{subset}-{weight}-{style}.{}",
                    FontFileType::Woff.extension()
                );
                let full_path = path_prefix.join(&file_name);
                src_parts.push(format!(
                    r#"url("{}") format("woff")"#,
                    full_path.to_string_lossy().replace("\\", "/")
                ));
            }
            None => src_parts.push(format!(r#"url("{u}") format("woff")"#)),
        }
    }
    if let Some(u) = map.get(&FontFileType::Ttf)
        && !u.is_empty()
    {
        match &self_hosted {
            Some(((weight, style, subset), path_prefix)) => {
                let file_name = format!(
                    "{subset}-{weight}-{style}.{}",
                    FontFileType::Ttf.extension()
                );
                let full_path = path_prefix.join(&file_name);
                src_parts.push(format!(
                    r#"url("{}") format("truetype")"#,
                    full_path.to_string_lossy().replace("\\", "/")
                ));
            }
            None => src_parts.push(format!(r#"url("{u}") format("truetype")"#)),
        }
    }
    src_parts
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::*;
    use crate::{FontSourceClient, QueryBuilder};
    use std::{collections::HashMap, fs};

    fn test_family(ttf_url: &str) -> FontSourceFamily {
        let mut url_map = HashMap::new();
        url_map.insert(FontFileType::Ttf, ttf_url.to_string());

        let mut subset_map = HashMap::new();
        subset_map.insert(
            "latin".to_string(),
            FontSourceVariantSubset { url: url_map },
        );

        let mut style_map = HashMap::new();
        style_map.insert("normal".to_string(), FontSourceSubsets(subset_map));

        let mut variant_map = HashMap::new();
        variant_map.insert(400_u16, FontSourceStyles(style_map));

        FontSourceFamily {
            id: "roboto".to_string(),
            family: "Roboto".to_string(),
            subsets: vec!["latin".to_string()],
            weights: vec![400],
            styles: vec!["normal".to_string()],
            default_subset: "latin".to_string(),
            unicode_range: HashMap::new(),
            variable: false,
            variants: FontSourceVariants(variant_map),
        }
    }

    #[test]
    fn css_url_maps_to_empty_list() {
        let empty: HashMap<&FontFileType, &String> = HashMap::new();

        assert!(css_url_map(&empty, None).is_empty());
        assert!(
            css_url_map(
                &empty,
                Some(((400, "normal", "latin"), Path::new("../roboto")))
            )
            .is_empty()
        );
    }

    #[tokio::test]
    async fn self_hosted_css_copy_failure() {
        let mut server = mockito::Server::new_async().await;
        let ttf_path = "/dummy.ttf";
        let ttf_url = format!("{}{}", server.url(), ttf_path);
        let _ttf_mock = server
            .mock("GET", ttf_path)
            .with_status(200)
            .with_body("dummy-font-data")
            .create_async()
            .await;

        let cache_root = tempfile::tempdir().unwrap();
        let client = FontSourceClient::with_cache_root(cache_root.path()).unwrap();
        fs::create_dir_all(client.family_cache_dir("roboto")).unwrap();

        // Passing a directory as destination makes fs::copy fail and should map
        // to FontSourceError::WriteFileFailed in FontSourceFamily::to_css.
        let dest_dir = tempfile::tempdir().unwrap();
        let family = test_family(&ttf_url);
        let query = QueryBuilder::new("Roboto").build();
        let err = family
            .to_css(
                &query,
                Some((&client, Path::new("../"), Some(dest_dir.path()))),
            )
            .await
            .unwrap_err();

        assert!(
            matches!(err, FontSourceError::WriteFileFailed { .. }),
            "expected WriteFileFailed, got {err:?}"
        );
    }
}
