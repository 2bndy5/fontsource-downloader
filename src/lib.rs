#![doc = include_str!("../README.md")]
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, missing_docs)]

#[cfg(feature = "pyo3")]
use pyo3::prelude::*;

pub mod error;

mod client;
pub use client::FontSourceClient;

pub(crate) mod caching;
pub(crate) mod responses;

pub(crate) mod query;
pub use query::{FontQuery, Weight};

/// A library to interface with Fontsource REST API.
#[cfg(feature = "pyo3")]
#[cfg_attr(feature = "pyo3", pymodule)]
fn fontsource_downloader(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<FontQuery>()?;
    m.add_class::<Weight>()?;
    m.add_class::<FontSourceClient>()?;
    Ok(())
}
