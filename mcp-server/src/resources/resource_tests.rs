use crate::core::OrgModeRouter;
use std::fs;
use tempfile::TempDir;

fn create_test_router_with_files() -> (OrgModeRouter, TempDir) {
    let temp_dir = TempDir::new().unwrap();
    let temp_path = temp_dir.path();

    // Create basic org file with ID
    fs::write(
        temp_path.join("test.org"),
        r#"* First Heading
:PROPERTIES:
:ID: heading-123
:END:
This is the first heading content.

** Sub Heading
Some sub heading content.
"#,
    )
    .unwrap();

    // Create file with document-level ID
    fs::write(
        temp_path.join("with_doc_id.org"),
        r#":PROPERTIES:
:ID: doc-id-789
:TITLE: Test Document
:END:

* Some Content
Regular heading content.
"#,
    )
    .unwrap();

    let router = OrgModeRouter::with_directory(temp_path.to_str().unwrap()).unwrap();
    (router, temp_dir)
}

#[tokio::test]
async fn test_read_file_success() {
    let (router, _temp_dir) = create_test_router_with_files();
    let uri = "org://test.org".to_string();
    let path = "test.org".to_string();

    let result = router.read_file(uri.clone(), path).await;

    assert!(result.is_ok());
    let read_result = result.unwrap();
    assert_eq!(read_result.contents.len(), 1);

    if let rmcp::model::ResourceContents::TextResourceContents {
        uri: result_uri,
        text,
        ..
    } = &read_result.contents[0]
    {
        assert_eq!(*result_uri, uri);
        assert!(text.contains("* First Heading"));
        assert!(text.contains("This is the first heading content"));
    } else {
        panic!("Expected text content");
    }
}

#[tokio::test]
async fn test_read_file_not_found() {
    let (router, _temp_dir) = create_test_router_with_files();
    let uri = "org://nonexistent.org".to_string();
    let path = "nonexistent.org".to_string();

    let result = router.read_file(uri, path).await;

    assert!(result.is_err());
    let error = result.unwrap_err();
    assert_eq!(error.code, rmcp::model::ErrorCode::INTERNAL_ERROR);
    assert!(error.message.contains("Failed to read org file"));
}

#[tokio::test]
async fn test_id_success_heading() {
    let (router, _temp_dir) = create_test_router_with_files();
    let uri = "org://id/heading-123".to_string();
    let id = "heading-123".to_string();

    let result = router.id(uri.clone(), id).await;

    assert!(result.is_ok());
    let read_result = result.unwrap();
    assert_eq!(read_result.contents.len(), 1);

    if let rmcp::model::ResourceContents::TextResourceContents {
        uri: result_uri,
        text,
        ..
    } = &read_result.contents[0]
    {
        assert_eq!(*result_uri, uri);
        assert!(text.contains("* First Heading"));
        assert!(text.contains(":ID: heading-123"));
    } else {
        panic!("Expected text content");
    }
}

#[tokio::test]
async fn test_id_success_document_level() {
    let (router, _temp_dir) = create_test_router_with_files();
    let uri = "org://id/doc-id-789".to_string();
    let id = "doc-id-789".to_string();

    let result = router.id(uri.clone(), id).await;

    assert!(result.is_ok());
    let read_result = result.unwrap();
    assert_eq!(read_result.contents.len(), 1);

    if let rmcp::model::ResourceContents::TextResourceContents {
        uri: result_uri,
        text,
        ..
    } = &read_result.contents[0]
    {
        assert_eq!(*result_uri, uri);
        assert!(text.contains(":ID: doc-id-789"));
        assert!(text.contains(":TITLE: Test Document"));
    } else {
        panic!("Expected text content");
    }
}

#[tokio::test]
async fn test_id_not_found() {
    let (router, _temp_dir) = create_test_router_with_files();
    let uri = "org://id/nonexistent-id".to_string();
    let id = "nonexistent-id".to_string();

    let result = router.id(uri, id).await;

    assert!(result.is_err());
    let error = result.unwrap_err();
    assert_eq!(error.code, rmcp::model::ErrorCode::INTERNAL_ERROR);
    assert!(error.message.contains("Failed to get element by id"));
}
