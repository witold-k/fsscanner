use fsscanner::pathutils::normalize_path;
use std::path::{Path, PathBuf};

#[test]
fn keeps_relative_paths_relative() {
    assert_eq!(
        normalize_path(Path::new("src/./module/../lib.rs")),
        PathBuf::from("src/lib.rs")
    );
}

#[test]
fn preserves_leading_parent_components() {
    assert_eq!(
        normalize_path(Path::new("../../src/../Cargo.toml")),
        PathBuf::from("../../Cargo.toml")
    );
}

#[test]
fn empty_and_current_directory_normalize_to_empty_relative_path() {
    assert_eq!(normalize_path(Path::new("")), PathBuf::new());
    assert_eq!(normalize_path(Path::new(".")), PathBuf::new());
}

#[cfg(unix)]
#[test]
fn absolute_paths_do_not_escape_root() {
    assert_eq!(
        normalize_path(Path::new("/tmp/project/../../etc/../file")),
        PathBuf::from("/file")
    );
}
