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

#[test]
fn expands_leading_tilde_to_home_directory() {
    let home_var = if cfg!(windows) { "USERPROFILE" } else { "HOME" };
    let Some(home) = std::env::var_os(home_var) else {
        return;
    };

    assert_eq!(
        normalize_path(Path::new("~/project/file.rs")),
        PathBuf::from(home).join("project/file.rs")
    );
}

#[test]
fn does_not_expand_non_leading_tilde() {
    assert_eq!(
        normalize_path(Path::new("project/~/file.rs")),
        PathBuf::from("project/~/file.rs")
    );
}
