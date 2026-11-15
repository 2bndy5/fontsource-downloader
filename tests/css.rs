use fontsource_downloader::{FontFileType, FontSourceClient, QueryBuilder, Weight};

const VARIANT_BASENAME: &str = "latin-400-normal";

fn variant_file(ext: &str) -> String {
    format!("{VARIANT_BASENAME}.{ext}")
}

fn test_client() -> (tempfile::TempDir, FontSourceClient) {
    let _ = env_logger::builder().is_test(true).try_init();
    let temp_root = tempfile::tempdir().unwrap();
    let client = FontSourceClient::with_cache_root(temp_root.path()).unwrap();
    (temp_root, client)
}

#[tokio::test]
async fn generate_cdn_css() {
    let (tmp, client) = test_client();
    assert_eq!(client.cache_root(), tmp.path());
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
    let (tmp, client) = test_client();
    let query = QueryBuilder::new("Roboto")
        .with_weight(Weight::Normal)
        .with_weight(Weight::Bold)
        .with_style("normal")
        .with_style("italic")
        .with_file_type(FontFileType::Woff2)
        .build();

    let export_dir = tmp.path().join("export");
    let woff2_names = [
        "latin-400-normal.woff2",
        "latin-400-italic.woff2",
        "latin-700-normal.woff2",
        "latin-700-italic.woff2",
    ];

    let css = client
        .css_self_hosted(&query, "", Some(&export_dir))
        .await
        .unwrap();

    let cache_family_dir = tmp.path().join("families/roboto");
    let export_family_dir = export_dir.join("roboto");

    assert!(css.contains("@font-face"));
    assert!(css.contains("font-family: 'Roboto';"));
    for file_name in woff2_names {
        assert!(css.contains(format!(r#"url("roboto/{file_name}") format("woff2")"#).as_str()));
        assert!(cache_family_dir.join(file_name).is_file());
        assert!(export_family_dir.join(file_name).is_file());
    }
}

#[tokio::test]
async fn generate_self_hosted_css_with_parent_prefix() {
    let (_tmp, client) = test_client();
    let query = QueryBuilder::new("Roboto")
        .with_file_type(FontFileType::Woff2)
        .with_file_type(FontFileType::Woff)
        .with_file_type(FontFileType::Ttf)
        .build();

    let rel_prefix = "../";

    let css = client
        .css_self_hosted(&query, rel_prefix, None)
        .await
        .unwrap();

    let expected_prefix = format!("{rel_prefix}roboto");
    let woff2_name = variant_file("woff2");
    let woff_name = variant_file("woff");
    let ttf_name = variant_file("ttf");
    assert!(
        css.contains(format!(r#"url("{expected_prefix}/{woff2_name}") format("woff2")"#).as_str())
    );
    assert!(
        css.contains(format!(r#"url("{expected_prefix}/{woff_name}") format("woff")"#).as_str())
    );
    assert!(
        css.contains(format!(r#"url("{expected_prefix}/{ttf_name}") format("truetype")"#).as_str())
    );
}
