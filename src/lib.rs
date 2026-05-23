#![doc = include_str!("../README.md")]
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, missing_docs)]

#[cfg(feature = "pyo3")]
use pyo3::prelude::*;

pub mod error;

mod client;
pub use client::FontSourceClient;

pub(crate) mod caching;
pub use caching::{FamilyCacheInfo, FontListCacheInfo};

pub(crate) mod responses;
pub use responses::FontSourceFamily;

pub(crate) mod query;
pub use query::{FontFileType, FontQuery, QueryBuilder, Weight};

/// A library to interface with Fontsource REST API.
#[cfg(feature = "pyo3")]
#[cfg_attr(feature = "pyo3", pymodule)]
fn fontsource_downloader(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<FontFileType>()?;
    m.add_class::<FontQuery>()?;
    m.add_class::<Weight>()?;
    m.add_class::<FontSourceClient>()?;
    m.add_class::<FontListCacheInfo>()?;
    m.add_class::<FamilyCacheInfo>()?;
    m.add_class::<FontSourceFamily>()?;
    Ok(())
}
