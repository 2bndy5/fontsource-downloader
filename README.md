# fontsource-downloader

[![Python][python-ci-badge]][python-ci-link]
[![Rust][rust-ci-badge]][rust-ci-link]
[![codecov][coverage-badge]][coverage-link]
[![docs.rs][docs-badge]][docs-link]
[![Crates.io][crates-io-badge]][crates-io-link]
[![PyPI][pypi-badge]][pypi-link]

A library to download (and cache) fonts with [fontsource] REST API.

## Library Features

- `async` compatible
- caching enabled (with option to customize cache root dir)

## Examples

### Python

```python
import asyncio # the only supported async runtime in python
from pathlib import Path
from fontsource_downloader import FontQuery, FontSourceClient, Weight

client = FontSourceClient()
query = FontQuery(
    family="Roboto",
    weight=Weight.Normal,
)
font_file: Path = await client.download_font(&query)
# do what you want with the cached font file ...
```

### Rust

```rust
use std::path::PathBuf;
use fontsource_downloader::{FontQuery, FontSourceClient, Weight};

let client = FontSourceClient::new().unwrap();
let query = FontQuery {
    family: "Roboto".to_string(),
    weight: Weight::Normal,
    ..Default::default()
};
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
