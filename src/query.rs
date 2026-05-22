#[cfg(feature = "pyo3")]
use pyo3::prelude::*;

/// A struct describing the desired font to download.
///
/// # Example
/// ```
/// use fontsource_downloader::FontQuery;
/// let query = FontQuery {
///     family: "Roboto".to_string(),
///     ..Default::default()
/// };
#[cfg_attr(
    feature = "pyo3",
    pyclass(module = "fontsource_downloader", frozen, get_all, from_py_object)
)]
#[derive(Debug, Clone)]
pub struct FontQuery {
    /// The font family's name.
    pub family: String,

    /// The font family's style.
    ///
    /// Fontsource typically only supports "normal" and "italic" styles,
    /// but some families do not have italic styles.
    pub style: String,

    /// The font's weight.
    ///
    /// Some families may not have all weights available.
    pub weight: Weight,

    /// The font family's lingual subset.
    ///
    /// The valid options for this can vary depending on the font family.
    pub subset: String,
}

impl Default for FontQuery {
    /// Create a default [`FontQuery`] with the following parameters:
    ///
    /// - [`FontQuery::family`] = `"Roboto"`
    /// - [`FontQuery::style`] = `"normal"`
    /// - [`FontQuery::weight`] = [`Weight::Normal`]
    /// - [`FontQuery::subset`] = `"latin"`
    fn default() -> Self {
        Self {
            family: String::from("Roboto"),
            style: String::from("normal"),
            weight: Weight::default(),
            subset: String::from("latin"),
        }
    }
}

impl FontQuery {
    pub(crate) fn normalized_style(&self) -> &'static str {
        let style = self.style.trim();
        if style.eq_ignore_ascii_case("italic") {
            return "italic";
        } else if style.eq_ignore_ascii_case("oblique") {
            return "oblique";
        }
        "normal"
    }

    pub(crate) fn normalized_subset(&self) -> &str {
        let result = self.subset.trim();
        if !result.is_empty() { result } else { "latin" }
    }
}

#[cfg(feature = "pyo3")]
#[pymethods]
impl FontQuery {
    /// Create a new ``FontQuery`` with the given parameters.
    ///
    /// Required parameter `family` is the display name of the font family to query.
    ///
    /// Optional parameter defaults are as follows:
    ///
    /// - ``style``: "normal"
    /// - ``weight``: `Weight.Normal` (or ``Weight(400)``)
    /// - ``subset``: "latin"
    #[new]
    #[pyo3(
        signature = (family, style=None, weight=None, subset=None),
        text_signature = "(family: str, style: str | None = None, weight: Weight | None = None, subset: str | None = None) -> FontQuery"
    )]
    pub fn new(
        family: String,
        style: Option<String>,
        weight: Option<Weight>,
        subset: Option<String>,
    ) -> Self {
        Self {
            family,
            style: style.unwrap_or_else(|| String::from("normal")),
            weight: weight.unwrap_or_default(),
            subset: subset.unwrap_or_else(|| String::from("latin")),
        }
    }
}

/// An enum representing the standard font weights.
#[cfg_attr(
    feature = "pyo3",
    pyclass(module = "fontsource_downloader", eq_int, eq, from_py_object)
)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
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
        let query = FontQuery {
            subset: String::new(), // Override default subset to test normalized_subset method
            style: String::from(" Italic  "), // Test trimming and case insensitivity
            ..Default::default()
        };
        assert_eq!(query.family, "Roboto");
        assert_eq!(query.normalized_style(), "italic");
        assert_eq!(query.weight, Weight::Normal);
        assert_eq!(query.normalized_subset(), "latin");

        assert_eq!(
            FontQuery {
                subset: String::from(" cyrillic "),
                ..Default::default()
            }
            .normalized_subset(),
            "cyrillic"
        );

        let query = FontQuery {
            style: String::from("Oblique"),
            ..Default::default()
        };
        assert_eq!(query.normalized_style(), "oblique");
    }
}
