#[cfg(feature = "pyo3")]
use pyo3::prelude::*;

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

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

    /// The mapping of font variants (weight, style, subset) to their corresponding URLs.
    #[serde(default)]
    pub(crate) variants: FontSourceVariants,
}

#[cfg_attr(feature = "pyo3", pymethods)]
impl FontSourceFamily {
    /// Retrieves the TTF URL for a specific font variant based on weight, style, and subset.
    ///
    /// Return `None` if the specified variant does not exist.
    pub fn variant_ttf_url(&self, weight: u16, style: &str, subset: &str) -> Option<&str> {
        self.variants
            .weight(weight)
            .and_then(|styles| styles.style(style))
            .and_then(|subsets| subsets.subset(subset))
            .map(|variant| variant.url.ttf.as_str())
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
/// Only contains the TTF URL.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FontSourceVariantSubset {
    url: FontSourceVariantUrls,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FontSourceVariantUrls {
    ttf: String,
}
