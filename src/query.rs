use std::collections::{HashSet, hash_set::Iter};

#[cfg(feature = "pyo3")]
use pyo3::prelude::*;
use serde::{Deserialize, Serialize};

/// A struct describing the desired font(s) to query.
///
#[cfg_attr(
    feature = "pyo3",
    pyclass(module = "fontsource_downloader", frozen, get_all, from_py_object)
)]
#[cfg_attr(
    not(feature = "pyo3"),
    doc = "Use [`QueryBuilder`] to construct a [`FontQuery`]."
)]
#[derive(Debug, Clone)]
pub struct FontQuery {
    family: String,
    style: HashSet<String>,
    weight: HashSet<Weight>,
    subset: HashSet<String>,
    pub(crate) file_type: HashSet<FontFileType>,
}

impl Default for FontQuery {
    /// Create a default [`FontQuery`] with the following default values:
    ///
    /// - `family` = `"Roboto"`
    /// - `style` = `"normal"`
    /// - `weight` = [`Weight::Normal`]
    /// - `subset` = `"latin"`
    /// - `file_type` = [`FontFileType::Ttf`]
    fn default() -> Self {
        Self {
            family: String::from("Roboto"),
            style: HashSet::from_iter([String::from("normal")]),
            weight: HashSet::from_iter([Weight::default()]),
            subset: HashSet::from_iter([String::from("latin")]),
            file_type: HashSet::from_iter([FontFileType::default()]),
        }
    }
}

/// A builder for constructing a [`FontQuery`].
///
/// # Example
/// ```rust
/// use fontsource_downloader::{QueryBuilder, FontFileType, Weight};
/// let query = QueryBuilder::new("Roboto")
///     .with_weight(Weight::from(400))
///     .with_weight(Weight::from(700))
///     .with_file_type(FontFileType::Woff2)
///     .with_file_type(FontFileType::Ttf)
///     .build();
/// assert_eq!(query.family(), "Roboto");
/// assert!(query.weights().any(|w| *w == Weight::Normal));
/// assert!(query.weights().any(|w| *w == Weight::Bold));
/// assert!(query.file_types().any(|t| *t == FontFileType::Woff2));
/// assert!(query.file_types().any(|t| *t == FontFileType::Ttf));
/// ```
#[derive(Debug, Default)]
pub struct QueryBuilder {
    family: String,
    style: HashSet<String>,
    weight: HashSet<Weight>,
    subset: HashSet<String>,
    file_type: HashSet<FontFileType>,
}

impl From<FontQuery> for QueryBuilder {
    fn from(value: FontQuery) -> Self {
        Self::from(&value)
    }
}

impl From<&FontQuery> for QueryBuilder {
    fn from(value: &FontQuery) -> Self {
        Self {
            family: value.family.clone(),
            style: value.style.clone(),
            weight: value.weight.clone(),
            subset: value.subset.clone(),
            file_type: value.file_type.clone(),
        }
    }
}

impl QueryBuilder {
    /// Create a new [`QueryBuilder`] for the given font family (display name).
    pub fn new(family: &str) -> Self {
        Self {
            family: family.trim().to_string(),
            ..Default::default()
        }
    }

    /// Add a style to this query.
    ///
    /// The style will be normalized to "normal", "italic", or "oblique"
    /// (case-insensitive, with leading/trailing whitespace stripped).
    ///
    /// If not specified, the default style is "normal".
    pub fn with_style(self, style: &str) -> Self {
        let mut styles = self.style;
        styles.insert(Self::normalized_style(style).to_string());
        Self {
            style: styles,
            ..self
        }
    }

    fn normalized_style(style: &str) -> &'static str {
        let style = style.trim();
        if style.eq_ignore_ascii_case("italic") {
            return "italic";
        } else if style.eq_ignore_ascii_case("oblique") {
            return "oblique";
        }
        "normal"
    }

    /// Add a weight to this query.
    ///
    /// If not specified, the default weight is [`Weight::Normal`]
    /// (or `Weight::from(400)`).
    pub fn with_weight(self, weight: Weight) -> Self {
        let mut weights = self.weight;
        weights.insert(weight);
        Self {
            weight: weights,
            ..self
        }
    }

    /// Add a subset to this query.
    ///
    /// The subset will be normalized by trimming surrounding whitespace.
    /// If the resulting string is empty (or never specified),
    /// it will default to "latin".
    pub fn with_subset(self, subset: &str) -> Self {
        let mut subsets = self.subset;
        subsets.insert(Self::normalized_subset(subset).to_string());
        Self {
            subset: subsets,
            ..self
        }
    }

    fn normalized_subset(subset: &str) -> &str {
        let result = subset.trim();
        if !result.is_empty() { result } else { "latin" }
    }

    /// Add a file type to this query.
    ///
    /// If not specified, the default file type is [`FontFileType::Ttf`].
    pub fn with_file_type(self, file_type: FontFileType) -> Self {
        let mut file_types = self.file_type;
        file_types.insert(file_type);
        Self {
            file_type: file_types,
            ..self
        }
    }

    /// Build the [`FontQuery`] from this builder.
    ///
    /// This applies default values for any fields that were not set.
    /// See [`FontQuery::default()`] for the default values.
    pub fn build(self) -> FontQuery {
        // Ensure that at least one style, weight, subset, and file type is specified
        let style = if self.style.is_empty() {
            HashSet::from_iter(["normal".to_string()])
        } else {
            self.style
        };
        let weight = if self.weight.is_empty() {
            HashSet::from_iter([Weight::Normal])
        } else {
            self.weight
        };
        let subset = if self.subset.is_empty() {
            HashSet::from_iter(["latin".to_string()])
        } else {
            self.subset
        };
        let file_type = if self.file_type.is_empty() {
            HashSet::from_iter([FontFileType::Ttf])
        } else {
            self.file_type
        };
        FontQuery {
            family: self.family,
            style,
            weight,
            subset,
            file_type,
        }
    }
}

impl FontQuery {
    /// The font family's display name.
    pub fn family(&self) -> &str {
        &self.family
    }

    /// Return an iterator over the styles in this query.
    pub fn styles(&self) -> Iter<'_, String> {
        self.style.iter()
    }

    /// An iterator over the weights in this query.
    pub fn weights(&self) -> Iter<'_, Weight> {
        self.weight.iter()
    }

    /// An iterator over the subsets in this query.
    pub fn subsets(&self) -> Iter<'_, String> {
        self.subset.iter()
    }

    /// An iterator over the file types in this query.
    pub fn file_types(&'_ self) -> Iter<'_, FontFileType> {
        self.file_type.iter()
    }

    pub(crate) fn filter_subsets<'a>(&'a self, available: &[String]) -> Vec<&'a String> {
        self.subsets().filter(|v| available.contains(v)).collect()
    }

    pub(crate) fn filter_styles<'a>(&'a self, available: &[String]) -> Vec<&'a String> {
        self.styles().filter(|v| available.contains(v)).collect()
    }

    pub(crate) fn filter_weights(&self, available: &[u16]) -> Vec<u16> {
        self.weights()
            .filter_map(|v| {
                let int_weight: u16 = (*v).into();
                if available.contains(&int_weight) {
                    Some(int_weight)
                } else {
                    None
                }
            })
            .collect()
    }
}

#[cfg(feature = "pyo3")]
#[pymethods]
impl FontQuery {
    /// Create a new `FontQuery` with the given parameters.
    ///
    /// Required parameter `family` is the display name of the font family to query.
    ///
    /// Optional parameter defaults are as follows:
    ///
    /// - ``styles=["normal"]``
    /// - ``weights=[Weight(400)]``
    /// - ``subsets=["latin"]``
    /// - ``file_types=[FontFileType.Ttf]``
    #[new]
    #[pyo3(
        signature = (family, styles=None, weights=None, subsets=None, file_types=None),
        text_signature = "(family: str, styles: str | None = None, weights: list[Weight] | None = None, subsets: list[str] | None = None, file_types: list[FontFileType] | None = None) -> FontQuery"
    )]
    pub fn new_py(
        family: String,
        styles: Option<Vec<String>>,
        weights: Option<Vec<Weight>>,
        subsets: Option<Vec<String>>,
        file_types: Option<Vec<FontFileType>>,
    ) -> Self {
        let mut result = QueryBuilder::new(&family);
        if let Some(styles) = styles {
            for style in styles {
                result = result.with_style(&style);
            }
        }
        if let Some(weight) = weights {
            for weight in weight {
                result = result.with_weight(weight);
            }
        }
        if let Some(subsets) = subsets {
            for subset in subsets {
                result = result.with_subset(&subset);
            }
        }
        if let Some(file_types) = file_types {
            for file_type in file_types {
                result = result.with_file_type(file_type);
            }
        }
        result.build()
    }

    /// The font family's display name.
    #[getter]
    pub fn get_family(&self) -> &str {
        &self.family
    }

    /// A ``list`` of the styles in this query.
    #[getter]
    pub fn get_styles(&self) -> Vec<String> {
        self.styles().cloned().collect()
    }

    /// A ``list`` of the weights in this query.
    #[getter]
    pub fn get_weights(&self) -> Vec<Weight> {
        self.weights().cloned().collect()
    }

    /// A ``list`` of the subsets in this query.
    #[getter]
    pub fn get_subsets(&self) -> Vec<String> {
        self.subsets().cloned().collect()
    }

    /// A ``list`` of the file types in this query.
    #[getter]
    pub fn get_file_types(&self) -> Vec<FontFileType> {
        self.file_types().cloned().collect()
    }
}

/// An enum representing supported downloadable font file types.
#[cfg_attr(
    feature = "pyo3",
    pyclass(module = "fontsource_downloader", eq, from_py_object)
)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[allow(missing_docs)]
pub enum FontFileType {
    Woff2,
    Woff,
    #[default]
    Ttf,
}

impl FontFileType {
    pub(crate) fn extension(&self) -> &'static str {
        match self {
            FontFileType::Woff2 => "woff2",
            FontFileType::Woff => "woff",
            FontFileType::Ttf => "ttf",
        }
    }
}

#[cfg(feature = "pyo3")]
#[cfg_attr(feature = "pyo3", pymethods)]
impl FontFileType {
    /// Return the file-extension representation of this ``FontFileType``.
    pub fn __str__(&self) -> &'static str {
        self.extension()
    }
}

/// An enum representing the standard font weights.
#[cfg_attr(
    feature = "pyo3",
    pyclass(module = "fontsource_downloader", eq_int, eq, from_py_object)
)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Hash)]
#[allow(missing_docs)]
pub enum Weight {
    Thin = 100,
    ExtraLight = 200,
    Light = 300,
    #[default]
    Normal = 400,
    Medium = 500,
    SemiBold = 600,
    Bold = 700,
    ExtraBold = 800,
    Black = 900,
}

impl From<Weight> for u16 {
    fn from(value: Weight) -> Self {
        value as u16
    }
}

impl From<&Weight> for u16 {
    fn from(value: &Weight) -> Self {
        (*value).into()
    }
}

impl From<u16> for Weight {
    fn from(value: u16) -> Self {
        // Round to nearest hundred
        let value = (value.clamp(100, 900) / 100) * 100;
        match value {
            100 => Weight::Thin,
            200 => Weight::ExtraLight,
            300 => Weight::Light,
            500 => Weight::Medium,
            600 => Weight::SemiBold,
            700 => Weight::Bold,
            800 => Weight::ExtraBold,
            900 => Weight::Black,
            _ => Weight::Normal, // Default to Normal for non-standard weights
        }
    }
}

#[cfg(feature = "pyo3")]
#[cfg_attr(feature = "pyo3", pymethods)]
impl Weight {
    /// Create a new ``Weight`` from a given integer value.
    ///
    /// The value will be rounded to the nearest standard weight (100, 200, ..., 900).
    #[new]
    pub fn new(value: i32) -> Self {
        (value.clamp(100, 900) as u16).into()
    }

    /// Get the integer value of this ``Weight``.
    pub fn __int__(&self) -> u16 {
        (*self).into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn weight_conversion() {
        assert_eq!(Weight::from(50), Weight::Thin); // Clamped to 100
        assert_eq!(Weight::from(256), Weight::ExtraLight);
        assert_eq!(Weight::from(322), Weight::Light);
        assert_eq!(Weight::from(404), Weight::Normal);
        assert_eq!(Weight::from(555), Weight::Medium);
        assert_eq!(Weight::from(666), Weight::SemiBold);
        assert_eq!(Weight::from(750), Weight::Bold);
        assert_eq!(Weight::from(888), Weight::ExtraBold);
        assert_eq!(Weight::from(950), Weight::Black); // Clamped to 900

        assert_eq!(u16::from(Weight::Thin), 100);
        assert_eq!(u16::from(Weight::ExtraLight), 200);
        assert_eq!(u16::from(Weight::Light), 300);
        assert_eq!(u16::from(Weight::Normal), 400);
        assert_eq!(u16::from(Weight::Medium), 500);
        assert_eq!(u16::from(Weight::SemiBold), 600);
        assert_eq!(u16::from(Weight::Bold), 700);
        assert_eq!(u16::from(Weight::ExtraBold), 800);
        assert_eq!(u16::from(Weight::Black), 900);

        let weight_ref = &Weight::Medium;
        assert_eq!(u16::from(weight_ref), 500);
    }

    #[test]
    fn font_query_defaults() {
        let mut query = QueryBuilder::new("Roboto")
            .with_subset("") // Override default subset to test normalized_subset method
            .with_style(" Italic  ")
            .build(); // Test trimming and case insensitivity
        assert_eq!(query.family, "Roboto");
        assert_eq!(query.styles().collect::<Vec<&String>>(), vec!["italic"]);
        assert_eq!(
            query.weights().collect::<Vec<&Weight>>(),
            vec![&Weight::Normal]
        );
        assert_eq!(
            query.file_types().collect::<Vec<&FontFileType>>(),
            vec![&FontFileType::Ttf]
        );
        assert_eq!(query.subsets().collect::<Vec<&String>>(), vec!["latin"]);

        query = QueryBuilder::from(query)
            .with_subset(" cyrillic ")
            .with_style("Oblique")
            .build();
        let subsets = query.subsets().map(|v| v.as_str()).collect::<Vec<&str>>();
        assert!(subsets.contains(&"latin"));
        assert!(subsets.contains(&"cyrillic"));
        let styles = query.styles().map(|v| v.as_str()).collect::<Vec<&str>>();
        assert!(styles.contains(&"italic"));
        assert!(styles.contains(&"oblique"));

        let default_query = QueryBuilder::new(&query.family).build();
        assert_eq!(default_query.family(), "Roboto");
        assert_eq!(
            default_query.styles().collect::<Vec<&String>>(),
            vec!["normal"]
        );
        assert_eq!(
            default_query.weights().collect::<Vec<&Weight>>(),
            vec![&Weight::Normal]
        );
        assert_eq!(
            default_query.file_types().collect::<Vec<&FontFileType>>(),
            vec![&FontFileType::Ttf]
        );
        assert_eq!(
            default_query.subsets().collect::<Vec<&String>>(),
            vec!["latin"]
        );
    }
}
