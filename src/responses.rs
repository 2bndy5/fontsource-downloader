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
    #[cfg(feature = "pyo3")]
    #[pyo3(get)]
    #[serde(default)]
    pub variable: bool,
    /// Whether this font family supports variable fonts.
    #[cfg(not(feature = "pyo3"))]
    #[serde(default)]
    pub variable: bool,

    /// The mapping of font variants (weight, style, subset) to their corresponding URLs.
    #[serde(default)]
    pub(crate) variants: FontSourceVariants,
}

impl FontSourceFamily {
    /// Returns whether this font family supports variable fonts.
    pub fn is_variable(&self) -> bool {
        self.variable
    }
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
                        log::debug!("Downloading font file: {}/{file_name}", self.id);
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

    /// Synthesize `@font-face` CSS for a variable font `query` using variable axes metadata.
    pub(crate) async fn to_variable_css(
        &self,
        query: &FontQuery,
        var_meta: &FontSourceVariable,
        client_dest: Option<(&FontSourceClient, &Path, Option<&Path>)>,
    ) -> Result<String> {
        if !self.is_variable() {
            return Err(FontSourceError::VariableFontNotSupported {
                family: self.family.clone(),
            });
        }

        let mut result = String::new();
        let mut download_tasks = JoinSet::new();

        let mut subsets = query.filter_subsets(&self.subsets);
        if subsets.is_empty() {
            subsets.push(&self.default_subset);
        }

        let filtered_styles = query.filter_styles(&self.styles);
        let styles: Vec<&str> = if filtered_styles.is_empty() {
            vec!["normal"]
        } else {
            filtered_styles.into_iter().map(|s| s.as_str()).collect()
        };

        let axes: Vec<&str> = if query.axes().len() == 0 {
            vec!["wght"]
        } else {
            query.axes().map(|a| a.as_str()).collect()
        };

        let family_name = if query.append_variable_suffix()
            && !self.family.to_lowercase().ends_with(" variable")
        {
            format!("{} Variable", self.family)
        } else {
            self.family.clone()
        };

        let weight_css = if let Some((min_w, max_w)) = var_meta.weight_range() {
            if (min_w - max_w).abs() < f64::EPSILON {
                format!("{min_w:.0}")
            } else {
                format!("{min_w:.0} {max_w:.0}")
            }
        } else {
            let weights = query.filter_weights(&self.weights);
            if weights.is_empty() {
                "400".to_string()
            } else {
                weights
                    .iter()
                    .map(|w| w.to_string())
                    .collect::<Vec<String>>()
                    .join(" ")
            }
        };

        let client_dest = client_dest.map(|(c, p, d)| (c, p.join(&self.id), d));

        for subset in &subsets {
            for style in &styles {
                let style_css = if *style == "oblique"
                    && let Some((min_s, max_s)) = var_meta.slant_range()
                {
                    if (min_s - max_s).abs() < f64::EPSILON {
                        format!("oblique {min_s:.0}deg")
                    } else {
                        format!("oblique {min_s:.0}deg {max_s:.0}deg")
                    }
                } else {
                    style.to_string()
                };

                for axis in &axes {
                    let file_name = format!("{subset}-{axis}-{style}.woff2");
                    let cdn_url = format!(
                        "https://cdn.jsdelivr.net/fontsource/fonts/{}:vf@latest/{file_name}",
                        self.id
                    );

                    let src_entry = if let Some((client, rel_prefix, dest)) = &client_dest {
                        if let Some(dest) = dest {
                            let family_cache_root = client.family_cache_dir(&self.id);
                            let font_path = family_cache_root.join(&file_name);
                            let dest = dest.to_path_buf();
                            let client = (*client).to_owned();
                            let font_url = cdn_url.clone();
                            log::debug!("Downloading variable font file: {}/{file_name}", self.id);
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
                        let full_path = rel_prefix.join(&file_name);
                        format!(
                            r#"url("{}") format("woff2-variations")"#,
                            full_path.to_string_lossy().replace('\\', "/")
                        )
                    } else {
                        format!(r#"url("{cdn_url}") format("woff2-variations")"#)
                    };

                    let mut css = format!(
                        r#"
@font-face {{
  font-family: '{family_name}';
  font-style: {style_css};
  font-weight: {weight_css};
  src:
    {src_entry};"#
                    );
                    if let Some(range) = self.unicode_range.get(*subset) {
                        css.push_str(&format!("\n  unicode-range: {range};"));
                    }
                    css.push_str("\n}\n");
                    result.push_str(&css);
                }
            }
        }

        FontSourceClient::collect_concurrent_tasks(
            &mut download_tasks,
            "downloading and copying self-hosted variable font files",
        )
        .await?;

        Ok(result)
    }
}

fn deserialize_f64_or_string<'de, D>(deserializer: D) -> std::result::Result<f64, D::Error>
where
    D: serde::Deserializer<'de>,
{
    struct F64OrStringVisitor;

    impl<'de> serde::de::Visitor<'de> for F64OrStringVisitor {
        type Value = f64;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("a float or string representing a float")
        }

        fn visit_i64<E>(self, value: i64) -> std::result::Result<f64, E>
        where
            E: serde::de::Error,
        {
            Ok(value as f64)
        }

        fn visit_u64<E>(self, value: u64) -> std::result::Result<f64, E>
        where
            E: serde::de::Error,
        {
            Ok(value as f64)
        }

        fn visit_f64<E>(self, value: f64) -> std::result::Result<f64, E>
        where
            E: serde::de::Error,
        {
            Ok(value)
        }

        fn visit_str<E>(self, value: &str) -> std::result::Result<f64, E>
        where
            E: serde::de::Error,
        {
            value
                .trim()
                .parse::<f64>()
                .map_err(serde::de::Error::custom)
        }
    }

    deserializer.deserialize_any(F64OrStringVisitor)
}

/// The metric parameters for a variable font axis.
#[cfg_attr(
    feature = "pyo3",
    pyclass(module = "fontsource_downloader", frozen, from_py_object)
)]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VariableAxis {
    /// The default value for this axis.
    #[cfg(feature = "pyo3")]
    #[pyo3(get)]
    #[serde(deserialize_with = "deserialize_f64_or_string")]
    pub default: f64,
    /// The default value for this axis.
    #[cfg(not(feature = "pyo3"))]
    #[serde(deserialize_with = "deserialize_f64_or_string")]
    pub default: f64,

    /// The minimum permitted value for this axis.
    #[cfg(feature = "pyo3")]
    #[pyo3(get)]
    #[serde(deserialize_with = "deserialize_f64_or_string")]
    pub min: f64,
    /// The minimum permitted value for this axis.
    #[cfg(not(feature = "pyo3"))]
    #[serde(deserialize_with = "deserialize_f64_or_string")]
    pub min: f64,

    /// The maximum permitted value for this axis.
    #[cfg(feature = "pyo3")]
    #[pyo3(get)]
    #[serde(deserialize_with = "deserialize_f64_or_string")]
    pub max: f64,
    /// The maximum permitted value for this axis.
    #[cfg(not(feature = "pyo3"))]
    #[serde(deserialize_with = "deserialize_f64_or_string")]
    pub max: f64,

    /// The incremental step size recommended for this axis.
    #[cfg(feature = "pyo3")]
    #[pyo3(get)]
    #[serde(deserialize_with = "deserialize_f64_or_string")]
    pub step: f64,
    /// The incremental step size recommended for this axis.
    #[cfg(not(feature = "pyo3"))]
    #[serde(deserialize_with = "deserialize_f64_or_string")]
    pub step: f64,
}

#[cfg(feature = "pyo3")]
#[pymethods]
impl VariableAxis {
    /// Create a new `VariableAxis`.
    #[new]
    #[pyo3(
        signature = (default, min, max, step),
        text_signature = "(default: float, min: float, max: float, step: float) -> VariableAxis"
    )]
    pub fn new(default: f64, min: f64, max: f64, step: f64) -> Self {
        Self {
            default,
            min,
            max,
            step,
        }
    }
}

/// The variable font metadata and axes for a font family.
#[cfg_attr(
    feature = "pyo3",
    pyclass(module = "fontsource_downloader", frozen, from_py_object)
)]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FontSourceVariable {
    /// The display name of the font family.
    #[cfg(feature = "pyo3")]
    #[pyo3(get)]
    pub family: String,
    /// The display name of the font family.
    #[cfg(not(feature = "pyo3"))]
    pub family: String,

    /// A map of variable font axis tags (e.g. "wght", "slnt", "wdth") to their metric constraints.
    #[cfg(feature = "pyo3")]
    #[pyo3(get)]
    pub axes: HashMap<String, VariableAxis>,
    /// A map of variable font axis tags (e.g. "wght", "slnt", "wdth") to their metric constraints.
    #[cfg(not(feature = "pyo3"))]
    pub axes: HashMap<String, VariableAxis>,
}

#[cfg_attr(feature = "pyo3", pymethods)]
impl FontSourceVariable {
    /// Create a new `FontSourceVariable`.
    #[cfg(feature = "pyo3")]
    #[new]
    #[pyo3(
        signature = (family, axes),
        text_signature = "(family: str, axes: dict[str, VariableAxis]) -> FontSourceVariable"
    )]
    pub fn new_py(family: String, axes: HashMap<String, VariableAxis>) -> Self {
        Self { family, axes }
    }

    /// Check whether a specific axis tag (e.g. "wght", "slnt", "wdth", "opsz") is available.
    pub fn has_axis(&self, tag: &str) -> bool {
        self.axes.contains_key(tag)
    }

    /// Get the axis metadata for a given tag.
    pub fn get_axis(&self, tag: &str) -> Option<VariableAxis> {
        self.axes.get(tag).cloned()
    }

    /// Get the weight range `(min, max)` if the "wght" axis is supported.
    pub fn weight_range(&self) -> Option<(f64, f64)> {
        self.axes.get("wght").map(|axis| (axis.min, axis.max))
    }

    /// Get the slant range `(min, max)` if the "slnt" axis is supported.
    pub fn slant_range(&self) -> Option<(f64, f64)> {
        self.axes.get("slnt").map(|axis| (axis.min, axis.max))
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

    #[test]
    fn deserialize_variable_axes_numbers_and_strings() {
        let json_with_strings = r#"{
            "family": "Inter",
            "axes": {
                "slnt": { "default": "0", "min": -10, "max": "0", "step": 0.5 },
                "wght": { "default": 400, "min": 100, "max": 900, "step": 1 }
            }
        }"#;

        let parsed: FontSourceVariable = serde_json::from_str(json_with_strings).unwrap();
        assert_eq!(parsed.family, "Inter");
        assert!(parsed.has_axis("wght"));
        assert!(parsed.has_axis("slnt"));
        assert!(!parsed.has_axis("wdth"));

        assert_eq!(parsed.weight_range(), Some((100.0, 900.0)));
        assert_eq!(parsed.slant_range(), Some((-10.0, 0.0)));

        let wght = parsed.get_axis("wght").unwrap();
        assert_eq!(wght.default, 400.0);
        assert_eq!(wght.min, 100.0);
        assert_eq!(wght.max, 900.0);
        assert_eq!(wght.step, 1.0);

        let slnt = parsed.get_axis("slnt").unwrap();
        assert_eq!(slnt.min, -10.0);
        assert_eq!(slnt.step, 0.5);

        let invalid = r#"{
            "family": "Inter",
            "axes": {
                "wght": { "default": null, "min": 100, "max": 900, "step": 1 }
            }
        }"#;
        let err = serde_json::from_str::<FontSourceVariable>(invalid).unwrap_err();
        assert!(
            err.to_string()
                .contains("a float or string representing a float")
        );
    }

    #[tokio::test]
    async fn variable_css_generation() {
        let mut unicode_range = HashMap::new();
        unicode_range.insert("latin".to_string(), "U+0000-00FF".to_string());

        let family = FontSourceFamily {
            id: "inter".to_string(),
            family: "Inter".to_string(),
            subsets: vec!["latin".to_string()],
            weights: vec![400],
            styles: vec!["normal".to_string(), "oblique".to_string()],
            default_subset: "latin".to_string(),
            unicode_range,
            variable: true,
            variants: FontSourceVariants::default(),
        };

        let mut axes = HashMap::new();
        axes.insert(
            "wght".to_string(),
            VariableAxis {
                default: 400.0,
                min: 100.0,
                max: 900.0,
                step: 1.0,
            },
        );
        axes.insert(
            "slnt".to_string(),
            VariableAxis {
                default: 0.0,
                min: -10.0,
                max: 0.0,
                step: 1.0,
            },
        );
        let var_meta = FontSourceVariable {
            family: "Inter".to_string(),
            axes,
        };

        // 1. Basic CDN variable CSS without suffix
        let query_basic = QueryBuilder::new("Inter").with_variable(true).build();
        let css_basic = family
            .to_variable_css(&query_basic, &var_meta, None)
            .await
            .unwrap();

        assert!(css_basic.contains("font-family: 'Inter';"));
        assert!(css_basic.contains("font-weight: 100 900;"));
        assert!(css_basic.contains("font-style: normal;"));
        assert!(css_basic.contains(
            r#"url("https://cdn.jsdelivr.net/fontsource/fonts/inter:vf@latest/latin-wght-normal.woff2") format("woff2-variations")"#
        ));
        assert!(css_basic.contains("unicode-range: U+0000-00FF;"));

        // 2. Opt-in suffix appends " Variable"
        let query_suffix = QueryBuilder::new("Inter")
            .with_variable(true)
            .with_variable_suffix(true)
            .build();
        let css_suffix = family
            .to_variable_css(&query_suffix, &var_meta, None)
            .await
            .unwrap();
        assert!(css_suffix.contains("font-family: 'Inter Variable';"));

        // If family already has "Variable", it doesn't double-append
        let mut fam_with_var = family.clone();
        fam_with_var.family = "Inter Variable".to_string();
        let css_no_double = fam_with_var
            .to_variable_css(&query_suffix, &var_meta, None)
            .await
            .unwrap();
        assert!(css_no_double.contains("font-family: 'Inter Variable';"));
        assert!(!css_no_double.contains("Variable Variable"));

        // 3. Oblique style with slant range
        let query_oblique = QueryBuilder::new("Inter")
            .with_variable(true)
            .with_style("oblique")
            .build();
        let css_oblique = family
            .to_variable_css(&query_oblique, &var_meta, None)
            .await
            .unwrap();
        assert!(css_oblique.contains("font-style: oblique -10deg 0deg;"));

        // 4. Non-variable font family errors
        let mut non_var_family = family.clone();
        non_var_family.variable = false;
        let err = non_var_family
            .to_variable_css(&query_basic, &var_meta, None)
            .await
            .unwrap_err();
        assert!(matches!(
            err,
            FontSourceError::VariableFontNotSupported { .. }
        ));
    }

    #[tokio::test]
    async fn variable_css_fallbacks() {
        let mut family = test_family("");
        family.variable = true;
        family.subsets = vec!["cyrillic".to_string()];
        family.weights = vec![700];
        family.styles = vec!["italic".to_string()];

        let var_meta = FontSourceVariable {
            family: family.family.clone(),
            axes: HashMap::new(),
        };
        let query = FontQuery::default();
        let css = family
            .to_variable_css(&query, &var_meta, None)
            .await
            .unwrap();

        assert!(css.contains("font-style: normal;"));
        assert!(css.contains("font-weight: 400;"));
        assert!(css.contains(
            r#"url("https://cdn.jsdelivr.net/fontsource/fonts/roboto:vf@latest/latin-wght-normal.woff2") format("woff2-variations")"#
        ));
        assert!(!css.contains("unicode-range:"));

        family.weights = vec![400];
        let css = family
            .to_variable_css(&query, &var_meta, None)
            .await
            .unwrap();
        assert!(css.contains("font-weight: 400;"));
    }

    #[tokio::test]
    async fn variable_css_fixed_weight_and_slant_ranges() {
        let mut family = test_family("");
        family.variable = true;
        family.styles = vec!["oblique".to_string()];

        let mut axes = HashMap::new();
        axes.insert(
            "wght".to_string(),
            VariableAxis {
                default: 600.0,
                min: 600.0,
                max: 600.0,
                step: 1.0,
            },
        );
        axes.insert(
            "slnt".to_string(),
            VariableAxis {
                default: -10.0,
                min: -10.0,
                max: -10.0,
                step: 1.0,
            },
        );
        let var_meta = FontSourceVariable {
            family: family.family.clone(),
            axes,
        };
        let query = QueryBuilder::new("Roboto")
            .with_variable(true)
            .with_style("oblique")
            .build();
        let css = family
            .to_variable_css(&query, &var_meta, None)
            .await
            .unwrap();

        assert!(css.contains("font-weight: 600;"));
        assert!(css.contains("font-style: oblique -10deg;"));
    }

    #[tokio::test]
    async fn variable_css_self_hosted_without_copy() {
        let mut family = test_family("");
        family.variable = true;

        let var_meta = FontSourceVariable {
            family: family.family.clone(),
            axes: HashMap::new(),
        };
        let query = QueryBuilder::new("Roboto").with_variable(true).build();
        let cache_root = tempfile::tempdir().unwrap();
        let client = FontSourceClient::with_cache_root(cache_root.path()).unwrap();
        let css = family
            .to_variable_css(&query, &var_meta, Some((&client, Path::new("../"), None)))
            .await
            .unwrap();

        assert!(
            css.contains(r#"url("../roboto/latin-wght-normal.woff2") format("woff2-variations")"#)
        );
    }

    #[tokio::test]
    async fn variable_css_self_hosted_copy() {
        let mut family = test_family("");
        family.variable = true;

        let var_meta = FontSourceVariable {
            family: family.family.clone(),
            axes: HashMap::new(),
        };
        let query = QueryBuilder::new("Roboto").with_variable(true).build();
        let cache_root = tempfile::tempdir().unwrap();
        let client = FontSourceClient::with_cache_root(cache_root.path()).unwrap();

        let font_path = client
            .family_cache_dir(&family.id)
            .join("latin-wght-normal.woff2");
        fs::create_dir_all(font_path.parent().unwrap()).unwrap();
        fs::write(&font_path, b"cached variable font").unwrap();

        let export_dir = cache_root.path().join("export");
        fs::create_dir_all(&export_dir).unwrap();
        let destination = export_dir.join("latin-wght-normal.woff2");
        let css = family
            .to_variable_css(
                &query,
                &var_meta,
                Some((&client, Path::new("../"), Some(&destination))),
            )
            .await
            .unwrap();

        assert!(
            css.contains(r#"url("../roboto/latin-wght-normal.woff2") format("woff2-variations")"#)
        );
        assert_eq!(fs::read(&destination).unwrap(), b"cached variable font");

        let destination_dir = tempfile::tempdir().unwrap();
        let err = family
            .to_variable_css(
                &query,
                &var_meta,
                Some((&client, Path::new("../"), Some(destination_dir.path()))),
            )
            .await
            .unwrap_err();
        assert!(matches!(err, FontSourceError::WriteFileFailed { .. }));
    }
}
