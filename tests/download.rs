use fontsource_downloader::{FontQuery, FontSourceClient};

async fn test_download(query: &FontQuery) {
    let tmp_cache_dir = tempfile::tempdir().unwrap();
    let client = FontSourceClient::with_cache_root(tmp_cache_dir.path()).unwrap();
    let font_path = client.download_font(query).await.unwrap();
    assert!(font_path.exists());
    println!("Downloaded font to: {}", font_path.to_string_lossy());
    // fetch it again to verify cache hit
    let cached_font_path = client.download_font(query).await.unwrap();
    assert_eq!(font_path, cached_font_path);
}

#[tokio::test]
async fn download_roboto_regular() {
    let query = FontQuery {
        family: "Roboto".to_string(),
        ..Default::default()
    };
    test_download(&query).await;
}
