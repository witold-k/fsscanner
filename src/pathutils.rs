// SPDX-License-Identifier: Apache-2.0
// Copyright (c) 2026 Witold Kaminski

use std::collections::HashSet;
use std::env;
use std::path::{Component, Path, PathBuf};

pub fn from_versioned_project(current: &Path) -> PathBuf {
    // Known VCS directory markers
    const VCS_DIRS: &[&str] = &[
        ".git", ".svn", ".hg", ".cvs", ".bzr", "_darcs", "_MTN",
        "BitKeeper", "BK", ".fslckout", "_FOSSIL_", ".fossil-settings",
    ];

    // Resolve starting directory
    let mut dir = current
        .canonicalize()
        .unwrap_or_else(|_| current.to_path_buf());

    // Walk upward through parents
    loop {
        for vcs in VCS_DIRS {
            if dir.join(vcs).exists() {
                return dir;
            }
        }

        // If we've reached the root, stop
        match dir.parent() {
            Some(parent) => dir = parent.to_path_buf(),
            None => return dir,
        }
    }
}

pub fn normalize_path(p: &Path) -> PathBuf {
    let mut out = PathBuf::new();

    for component in p.components() {
        match component {
            Component::Prefix(prefix) => out.push(prefix.as_os_str()),
            Component::RootDir => out.push(component.as_os_str()),
            Component::CurDir => {}
            Component::ParentDir => {
                // Collapse "foo/.." lexically, but preserve leading ".." in
                // relative paths. Absolute paths never escape their root.
                if matches!(out.components().next_back(), Some(Component::Normal(_))) {
                    out.pop();
                } else if !out.has_root() {
                    out.push(component.as_os_str());
                }
            }
            Component::Normal(part) => out.push(part),
        }
    }

    out
}

#[cfg(test)]
mod normalize_path_tests {
    use super::normalize_path;
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
}

/// Tries to flexibly find the path given by the LLM within the project directory.
/// Allows for relative deviations, incorrect prefixes (like ../), and partial paths.
pub fn resolve_relaxed_path<P1, P2>(projdir: P1, llm_path: P2) -> Option<PathBuf>
where
    P1: AsRef<Path>,
    P2: AsRef<Path>,
{
    let projdir = projdir.as_ref();
    let llm_path = llm_path.as_ref();

    // 1. Clean the LLM path from typical prefix errors (e.g., "../", "./", leading slashes)
    // We normalize slashes first, then filter out empty, current dir, and parent dir components.
    let cleaned_str = llm_path.to_string_lossy().replace("\\", "/");
    let cleaned_path_buf: PathBuf = Path::new(&cleaned_str)
        .components()
        .filter(|c| match c {
            std::path::Component::Normal(_) => true,
            _ => false, // Skips Prefixes, RootDir, CurDir (.), and ParentDir (..)
        })
        .collect();

    if cleaned_path_buf.as_os_str().is_empty() {
        return None;
    }

    // Strategy 1: Direct combining (projdir + cleaned path)
    let direct_match = projdir.join(&cleaned_path_buf);
    if direct_match.is_file() {
        return Some(direct_match);
    }

    // Strategy 2: If the LLM path is deep, step-by-step strip the leading directories
    let mut components: Vec<&str> = cleaned_path_buf
        .iter()
        .map(|c| c.to_str().unwrap_or(""))
        .filter(|s| !s.is_empty())
        .collect();

    while components.len() > 1 {
        components.remove(0); // Strip the leftmost directory
        let sub_path: PathBuf = components.iter().collect();
        let test_match = projdir.join(sub_path);
        if test_match.is_file() {
            return Some(test_match);
        }
    }

    // Strategy 3: Last resort via custom iterative filesystem heuristics
    if let Some(target_file_name) = cleaned_path_buf.file_name() {
        let mut best_match: Option<PathBuf> = None;
        let mut highest_match_count = 0;

        // Custom iterative stack-based traversal (adapted from your collect_files implementation)
        let mut stack = Vec::with_capacity(128);
        let mut visited_symlink_dirs = HashSet::with_capacity(128);
        stack.push(projdir.to_path_buf());

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
                    if !file_type.is_symlink() {
                        stack.push(path);
                        continue;
                    }
                    match path.canonicalize() {
                        Ok(canon) => {
                            if visited_symlink_dirs.insert(canon) {
                                stack.push(path);
                            }
                        }
                        Err(_) => continue,
                    }
                } else if file_type.is_file()
                    && let Some(current_file_name) = path.file_name()
                        && current_file_name == target_file_name {
                            // Count how many path segments match starting from the tail
                            let mut match_count = 0;
                            let mut cur_iter = path.iter().rev();
                            let mut llm_iter = cleaned_path_buf.iter().rev();

                            while let (Some(c), Some(l)) = (cur_iter.next(), llm_iter.next()) {
                                if c == l {
                                    match_count += 1;
                                } else {
                                    break;
                                }
                            }

                            if match_count > highest_match_count {
                                highest_match_count = match_count;
                                best_match = Some(path);
                            }
                        }
            }
        }

        if best_match.is_some() {
            return best_match;
        }
    }

    None
}

