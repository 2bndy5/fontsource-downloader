# fontsource-downloader

A library to download (and cache) fonts with [fontsource] REST API.

## Library Features

- `async` compatible
- caching enabled (with option to customize cache root dir)

## Examples

### Python

```python
import asyncio # only supported async runtime in python
from fontsource_downloader import FontQuery, FontSourceClient, Weight

client = FontSourceClient()
query = FontQuery(
    family: "Roboto",
    weight: Weight.Normal,
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
