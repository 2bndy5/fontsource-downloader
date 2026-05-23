use fontsource_downloader::{FontQuery, FontSourceClient, QueryBuilder};

async fn test_download(query: &FontQuery) {
    let tmp_cache_dir = tempfile::tempdir().unwrap();
    let client = FontSourceClient::with_cache_root(tmp_cache_dir.path()).unwrap();
    let font_paths = client.download_font(query).await.unwrap();
    assert!(!font_paths.is_empty());
    for p in &font_paths {
        assert!(p.exists());
        println!("Downloaded font to: {}", p.to_string_lossy());
    }
    // fetch it again to verify cache hit
    let cached_font_paths = client.download_font(query).await.unwrap();
    assert_eq!(font_paths, cached_font_paths);
}

#[tokio::test]
async fn download_roboto_regular() {
    let query = QueryBuilder::new("Roboto").build();
    test_download(&query).await;
}
