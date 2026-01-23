//! Tests for the read function.

use config_locate::{ReadError, read};

#[tokio::test]
async fn test_read_first_file_exists() {
    let paths = vec!["tests/fixtures/config1.txt"];
    let result = read(&paths).await.unwrap();
    assert_eq!(result.payload, "config1 content");
    assert_eq!(
        result.path,
        std::path::PathBuf::from("tests/fixtures/config1.txt")
    );
}

#[tokio::test]
async fn test_read_second_file_when_first_missing() {
    let paths = vec![
        "tests/fixtures/nonexistent.txt",
        "tests/fixtures/config2.txt",
    ];
    let result = read(&paths).await.unwrap();
    assert_eq!(result.payload, "config2 content");
    assert_eq!(
        result.path,
        std::path::PathBuf::from("tests/fixtures/config2.txt")
    );
}

#[tokio::test]
async fn test_read_no_files_found() {
    let paths = vec![
        "tests/fixtures/nonexistent1.txt",
        "tests/fixtures/nonexistent2.txt",
    ];
    let result = read(&paths).await.unwrap_err();
    match result {
        ReadError::NotFound {
            paths: not_found_paths,
        } => {
            assert_eq!(not_found_paths.len(), 2);
            assert!(
                not_found_paths
                    .contains(&std::path::PathBuf::from("tests/fixtures/nonexistent1.txt"))
            );
            assert!(
                not_found_paths
                    .contains(&std::path::PathBuf::from("tests/fixtures/nonexistent2.txt"))
            );
        }
        _ => panic!("Expected NotFound error"),
    }
}
