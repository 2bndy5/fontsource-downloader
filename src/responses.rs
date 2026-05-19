use std::collections::HashMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FontSourceFamily {
    pub id: String,
    pub family: String,
    pub subsets: Vec<String>,
    pub weights: Vec<u16>,
    pub styles: Vec<String>,
    pub def_subset: String,
    #[serde(default)]
    pub variants: FontSourceVariants,
}

impl FontSourceFamily {
    pub fn variant_ttf_url(&self, weight: u16, style: &str, subset: &str) -> Option<&str> {
        self.variants
            .weight(weight)
            .and_then(|styles| styles.style(style))
            .and_then(|subsets| subsets.subset(subset))
            .map(|variant| variant.url.ttf.as_str())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(transparent)]
pub struct FontSourceVariants(HashMap<u16, FontSourceStyles>);

impl FontSourceVariants {
    fn weight(&self, weight: u16) -> Option<&FontSourceStyles> {
        self.0.get(&weight)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(transparent)]
pub struct FontSourceStyles(HashMap<String, FontSourceSubsets>);

impl FontSourceStyles {
    fn style(&self, style: &str) -> Option<&FontSourceSubsets> {
        self.0.get(style)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(transparent)]
pub struct FontSourceSubsets(HashMap<String, FontSourceVariantSubset>);

impl FontSourceSubsets {
    fn subset(&self, subset: &str) -> Option<&FontSourceVariantSubset> {
        self.0.get(subset)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FontSourceVariantSubset {
    url: FontSourceVariantUrls,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FontSourceVariantUrls {
    ttf: String,
}
