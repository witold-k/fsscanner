// SPDX-License-Identifier: Apache-2.0
// Copyright (c) 2026 Witold Kaminski

use std::path::PathBuf;
use fsscanner::fsscanner_base::collect_files_all;
use fsscanner::fsscanner_mt::process_dir_map;

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::{Path, PathBuf};
    use std::sync::{Arc, Mutex};

    // A simple local walker (same logic as your optimized version)
    fn collect_rs_files(root: &Path, out: &mut Vec<PathBuf>) {
        let mut stack = vec![root.to_path_buf()];

        while let Some(dir) = stack.pop() {
            let entries = match std::fs::read_dir(&dir) {
                Ok(e) => e,
                Err(_) => continue,
            };

            for entry in entries.flatten() {
                let path = entry.path();
                let ft = match entry.file_type() {
                    Ok(ft) => ft,
                    Err(_) => continue,
                };

                if ft.is_dir() {
                    stack.push(path);
                } else if ft.is_file() &&
                    path.extension().map(|e| e == "rs").unwrap_or(false) {
                        out.push(path);
                    }
            }
        }
    }

    #[test]
    fn test_process_dir_map_scans_crate_rs_files() {
        // The root of THIS crate
        let crate_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));

        // Collect expected files using our local walker
        let mut expected = Vec::new();
        collect_rs_files(&crate_root, &mut expected);
        expected.sort();

        // Track files seen by process_dir_map
        let seen = Arc::new(Mutex::new(Vec::<PathBuf>::new()));

        // Run the directory processor
        process_dir_map(
            crate_root.to_str().unwrap(),
            "/dev/null", // output_root unused for this test
            "rs",
            "ignored",
            {
                let seen = seen.clone();
                move |input, _output| {
                    seen.lock().unwrap().push(input.to_path_buf());
                    Ok(())
                }
            },
        )
        .unwrap();

        // Compare results
        let mut seen = seen.lock().unwrap().clone();
        seen.sort();

        assert_eq!(seen, expected);
    }
}

#[test]
fn test_collect_files_all_scans_crate() {
    let crate_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));

    let mut files = Vec::new();
    collect_files_all(&crate_root, &mut files);

    files.sort();

    // Known files from the existing crate tree.
    assert!(files.contains(&crate_root.join("Cargo.toml")));
    assert!(files.contains(&crate_root.join("README.md")));
    assert!(files.contains(&crate_root.join("LICENSE")));

    assert!(files.contains(&crate_root.join("src/lib.rs")));
    assert!(files.contains(&crate_root.join("src/fsscanner_mt.rs")));
    assert!(files.contains(&crate_root.join("src/fsscanner_st.rs")));
    assert!(files.contains(&crate_root.join("src/pathfilter.rs")));

    assert!(files.contains(
        &crate_root.join("src/bin/collect_to_md.rs")
    ));

    assert!(files.contains(
        &crate_root.join("tests/fsscanner_test.rs")
    ));
}

#[test]
fn test_collect_files_all_scans_src_recursively() {
    let crate_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let src_root = crate_root.join("src");

    let mut files = Vec::new();
    collect_files_all(&src_root, &mut files);

    files.sort();

    // Files directly below src/.
    assert!(files.contains(&src_root.join("lib.rs")));
    assert!(files.contains(&src_root.join("fileentry.rs")));
    assert!(files.contains(&src_root.join("fsscanner_base.rs")));
    assert!(files.contains(&src_root.join("fsscanner_mt.rs")));
    assert!(files.contains(&src_root.join("fsscanner_st.rs")));
    assert!(files.contains(&src_root.join("pathfilter.rs")));
    assert!(files.contains(&src_root.join("pathutils.rs")));
    assert!(files.contains(&src_root.join("threadpool.rs")));

    // File in nested directory.
    assert!(files.contains(
        &src_root.join("bin/collect_to_md.rs")
    ));
}

#[test]
fn test_collect_files_all_collects_non_rs_files() {
    let crate_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));

    let mut files = Vec::new();
    collect_files_all(&crate_root, &mut files);

    // This function collects ALL files, not only Rust files.
    assert!(files.contains(&crate_root.join("Cargo.toml")));
    assert!(files.contains(&crate_root.join("README.md")));
    assert!(files.contains(&crate_root.join("LICENSE")));

    assert!(files.contains(&crate_root.join("src/lib.rs")));
}

#[test]
fn test_collect_files_all_does_not_collect_directories() {
    let crate_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));

    let mut files = Vec::new();
    collect_files_all(&crate_root, &mut files);

    assert!(!files.contains(&crate_root));
    assert!(!files.contains(&crate_root.join("src")));
    assert!(!files.contains(&crate_root.join("src/bin")));
    assert!(!files.contains(&crate_root.join("tests")));
}

#[test]
fn test_collect_files_all_on_file() {
    let crate_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let file = crate_root.join("src/lib.rs");

    let mut files = Vec::new();
    collect_files_all(&file, &mut files);

    // The current implementation expects a directory as root.
    assert!(files.is_empty());
}

#[test]
fn test_collect_files_all_on_empty_or_invalid_path() {
    let crate_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));

    let nonexistent = crate_root.join("this-path-does-not-exist");

    let mut files = Vec::new();
    collect_files_all(&nonexistent, &mut files);

    assert!(files.is_empty());
}

