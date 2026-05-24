# fontsource-downloader

[![Python][python-ci-badge]][python-ci-link]
[![Rust][rust-ci-badge]][rust-ci-link]
[![codecov][coverage-badge]][coverage-link]
[![docs.rs][docs-badge]][docs-link]
[![Crates.io][crates-io-badge]][crates-io-link]
[![PyPI][pypi-badge]][pypi-link]

A library to download (and cache) fonts with [fontsource] REST API.

## Library Features

- Download a batch of fonts per family (based on the given query).
- Asynchronous downloads with true parallelism.
- Generate CSS `@font-face` rules. Both CDN URLs and self-hosting (relative path)
  URLs are supported. Self-hosted font files can be optionally copied from cache
  to a given destination directory.
- Caching enabled (with option to customize cache root dir)
  for both metadata and font files.
- Minimal debug logs (when enabled).

Rust consumers get to choose their desired TLS backend. Only the required
features of the reqwest crate are enabled.

## Examples

### Python

```python
import asyncio # the only supported async runtime in python
from pathlib import Path
# enable logging before importing this library
from fontsource_downloader import FontQuery, FontSourceClient, Weight

client = FontSourceClient()
query = FontQuery(
    family="Roboto",
    weights=[Weight(400)],
)
font_file: Path = await client.download_font(&query)
# do what you want with the cached font file ...
```

### Rust

```rust
use std::path::PathBuf;
use fontsource_downloader::{
    FontQuery, FontSourceClient, QueryBuilder, Weight,
};

let client = FontSourceClient::new().unwrap();
let query: FontQuery = QueryBuilder::new("Roboto")
    .with_weight(Weight::from(400))
    .build();
let font_file: PathBuf = client.download_font(&query).await.unwrap();
// do what you want with the cached font file ...
```

[fontsource]: https://fontsource.org/docs/api/introduction
[python-ci-badge]: https://github.com/2bndy5/fontsource-downloader/actions/workflows/python.yml/badge.svg
[python-ci-link]: https://github.com/2bndy5/fontsource-downloader/actions/workflows/python.yml
[rust-ci-badge]: https://github.com/2bndy5/fontsource-downloader/actions/workflows/rust.yml/badge.svg
[rust-ci-link]: https://github.com/2bndy5/fontsource-downloader/actions/workflows/rust.yml
[docs-badge]: https://img.shields.io/docsrs/fontsource_downloader
[docs-link]: https://docs.rs/fontsource_downloader
[crates-io-badge]: https://img.shields.io/crates/v/fontsource_downloader
[crates-io-link]: https://crates.io/crates/fontsource_downloader
[pypi-badge]: https://img.shields.io/pypi/v/fontsource-downloader
[pypi-link]:https://pypi.org/project/fontsource-downloader
[coverage-badge]: https://codecov.io/gh/2bndy5/fontsource-downloader/graph/badge.svg?token=4OQA5DWNJC
[coverage-link]: https://codecov.io/gh/2bndy5/fontsource-downloader
