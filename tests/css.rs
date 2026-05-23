use std::fs;

use fontsource_downloader::{FontFileType, FontSourceClient, QueryBuilder};

const VARIANT_BASENAME: &str = "latin-400-normal";

fn variant_file(ext: &str) -> String {
    format!("{VARIANT_BASENAME}.{ext}")
}

fn test_client() -> (tempfile::TempDir, FontSourceClient) {
    let cache_root = tempfile::tempdir().unwrap();
    let client = FontSourceClient::with_cache_root(cache_root.path()).unwrap();
    (cache_root, client)
}

#[tokio::test]
async fn generate_cdn_css() {
    let (cache_root, client) = test_client();
    assert_eq!(client.cache_root(), cache_root.path());
    let query = QueryBuilder::new("Roboto")
        .with_file_type(FontFileType::Woff2)
        .with_file_type(FontFileType::Woff)
        .with_file_type(FontFileType::Ttf)
        .build();

    let css = client.css(&query).await.unwrap();

    assert!(css.contains("@font-face"));
    assert!(css.contains("font-family: 'Roboto';"));
    assert!(css.contains(r#"format("woff2")"#));
    assert!(css.contains(r#"format("woff")"#));
    assert!(css.contains(r#"format("truetype")"#));
    assert!(css.contains("https://"));
}

#[tokio::test]
async fn generate_self_hosted_css() {
    let (cache_root, client) = test_client();
    let query = QueryBuilder::new("Roboto")
        .with_file_type(FontFileType::Woff2)
        .with_file_type(FontFileType::Woff)
        .with_file_type(FontFileType::Ttf)
        .build();

    let export_dir = cache_root.path().join("export");
    fs::create_dir_all(&export_dir).unwrap();
    let woff2_name = variant_file("woff2");
    let woff_name = variant_file("woff");
    let ttf_name = variant_file("ttf");
    let dest = export_dir.join(&woff2_name);

    let css = client.css_self_hosted(&query, &dest).await.unwrap();

    let cache_family_dir = cache_root.path().join("families/roboto");
    let cached_font_woff2 = cache_family_dir.join(&woff2_name);
    let cached_font_woff = cache_family_dir.join(&woff_name);
    let cached_font_ttf = cache_family_dir.join(&ttf_name);
    let exported_font = dest;

    assert!(css.contains("@font-face"));
    assert!(css.contains("font-family: 'Roboto';"));
    assert!(css.contains(format!(r#"url("roboto/{woff2_name}") format("woff2")"#).as_str()));
    assert!(css.contains(format!(r#"url("roboto/{woff_name}") format("woff")"#).as_str()));
    assert!(css.contains(format!(r#"url("roboto/{ttf_name}") format("truetype")"#).as_str()));
    assert!(cached_font_woff2.exists());
    assert!(cached_font_woff.exists());
    assert!(cached_font_ttf.exists());
    assert!(exported_font.exists());
}
