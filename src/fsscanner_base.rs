// SPDX-License-Identifier: Apache-2.0
// Copyright (c) 2026 Witold Kaminski

//! Non-recursive, stack-based filesystem scanning utilities.
//!
//! This module provides functions to crawl directory trees efficiently without
//! utilizing standard recursion, avoiding potential stack overflow scenarios on deep
//! nesting branches. It includes loop-safety tracking guards to handle cyclic symlinks safely.

/// Generic parallel directory processor.
/// - Scans for files with `extension`
/// - Builds output paths under `output_root`
/// - Replaces extension with `suffix`
/// - Calls `callback(input_path, output_path)`
use std::{
    collections::HashSet,
    path::{Path, PathBuf}
};

/// Traverses a directory tree to collect paths matching a single target file extension.
///
/// Iterates over directory branches using an internal allocation stack. If it encounters
/// symbolic link directories, it automatically resolves their canonical path targets
/// to avoid traversing circular reference traps.
///
/// # Arguments
///
/// * `root` - The root directory path slice where traversal starts.
/// * `extension` - The targeted extension filter to match files against (e.g., `"rs"`).
/// * `out` - A mutable destination vector where discovered file paths are collected.
///
/// # Examples
///
/// ```rust
/// use std::path::Path;
/// use fsscanner::fsscanner_base::collect_files_fast;
///
/// let root = Path::new(".");
/// let mut matches = Vec::new();
/// collect_files_fast(root, "toml", &mut matches);
/// ```
pub fn collect_files_fast(root: &Path, extension: &str, out: &mut Vec<PathBuf>) {
    let mut stack = Vec::with_capacity(128);
    let mut visited_symlink_dirs = HashSet::with_capacity(128);

    stack.push(root.to_path_buf());

    while let Some(dir) = stack.pop() {
        let entries = match std::fs::read_dir(&dir) {
            Ok(e) => e,
            Err(_) => continue,
        };

        for entry in entries.flatten() {
            let file_type = match entry.file_type() {
                Ok(ft) => ft,
                Err(_) => continue,
            };

            let path = entry.path();

            if file_type.is_dir() {
                // Fast path: normal directory -> no cycle possible
                if !file_type.is_symlink() {
                    stack.push(path);
                    continue;
                }

                // Slow path: symlinked directory -> must check for cycles
                match path.canonicalize() {
                    Ok(canon) => {
                        if visited_symlink_dirs.insert(canon) {
                            stack.push(path);
                        }
                    }
                    Err(_) => continue,
                }
            } else if
                file_type.is_file() &&
                path.extension().map(|ext| ext == extension).unwrap_or(false)
            {
                out.push(path);
            }
        }
    }
}

/// Traverses a directory tree to collect paths matching any extension in a provided list.
///
/// Precomputes an internal hash map index out of the passed extensions to query file entries
/// inside large directories at optimal runtime thresholds.
///
/// # Arguments
///
/// * `root` - The root directory path slice where traversal starts.
/// * `extensions` - A slice of reference strings containing extensions (e.g., `&["rs", "toml"]`).
/// * `out` - A mutable destination vector where discovered file paths are collected.
///
/// # Examples
///
/// ```rust
/// use std::path::Path;
/// use fsscanner::fsscanner_base::collect_files_fast_multi;
///
/// let root = Path::new(".");
/// let mut target_files = Vec::new();
/// collect_files_fast_multi(root, &["json", "yaml"], &mut target_files);
/// ```
#[allow(clippy::collapsible_if)]
pub fn collect_files_fast_multi(
    root: &Path,
    extensions: &[&str],
    out: &mut Vec<PathBuf>,
) {
    let mut stack = Vec::with_capacity(128);
    let mut visited_symlink_dirs = HashSet::with_capacity(128);

    // Precompute extension set for fast lookup
    let ext_set: HashSet<&str> = extensions.iter().copied().collect();

    stack.push(root.to_path_buf());

    while let Some(dir) = stack.pop() {
        let entries = match std::fs::read_dir(&dir) {
            Ok(e) => e,
            Err(_) => continue,
        };

        for entry in entries.flatten() {
            let file_type = match entry.file_type() {
                Ok(ft) => ft,
                Err(_) => continue,
            };

            let path = entry.path();

            if file_type.is_dir() {
                // Fast path: normal directory -> no cycle possible
                if !file_type.is_symlink() {
                    stack.push(path);
                    continue;
                }

                // Slow path: symlinked directory -> must check for cycles
                match path.canonicalize() {
                    Ok(canon) => {
                        if visited_symlink_dirs.insert(canon) {
                            stack.push(path);
                        }
                    }
                    Err(_) => continue,
                }
            } else if file_type.is_file() {
                if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                    if ext_set.contains(ext) {
                        out.push(path);
                    }
                }
            }
        }
    }
}

