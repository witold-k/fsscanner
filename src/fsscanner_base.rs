// SPDX-License-Identifier: Apache-2.0
// Copyright (c) 2026 Witold Kaminski

//! Non-recursive, stack-based filesystem scanning utilities.
//!
//! Normal directories use a cheap fast path. Symbolic links are resolved only when
//! encountered, and canonical link targets are tracked to prevent cycles.

use crate::pathfilter::Pathfilter;
use std::{collections::HashSet, path::{Path, PathBuf}};

#[derive(Clone)]
struct PendingDir {
    path: PathBuf,
    symlink_ancestors: Vec<PathBuf>,
}

fn collect_files(
    root: &Path,
    extensions: Option<&HashSet<&str>>,
    filter: Option<&Pathfilter>,
    out: &mut Vec<PathBuf>,
) {
    let mut stack = Vec::with_capacity(128);
    let mut visited_symlink_targets = HashSet::with_capacity(128);
    stack.push(PendingDir { path: root.to_path_buf(), symlink_ancestors: Vec::new() });

    while let Some(pending) = stack.pop() {
        let entries = match std::fs::read_dir(&pending.path) {
            Ok(entries) => entries,
            Err(_) => continue,
        };

        for entry in entries.flatten() {
            let path = entry.path();
            let file_type = match entry.file_type() {
                Ok(file_type) => file_type,
                Err(_) => continue,
            };

            if file_type.is_dir() {
                stack.push(PendingDir {
                    path,
                    symlink_ancestors: pending.symlink_ancestors.clone(),
                });
                continue;
            }

            if file_type.is_file() {
                if filter.is_none_or(|filter| filter.contains(&path))
                    && matches_extension(&path, extensions)
                {
                    out.push(path);
                }
                continue;
            }

            if !file_type.is_symlink() {
                continue;
            }

            let target = match path.canonicalize() {
                Ok(target) => target,
                Err(_) => continue,
            };
            if !filter.is_none_or(|filter| filter.contains_resolved(&target)) {
                continue;
            }

            let metadata = match std::fs::metadata(&target) {
                Ok(metadata) => metadata,
                Err(_) => continue,
            };

            if metadata.is_file() {
                if matches_extension(&target, extensions) {
                    out.push(target);
                }
            } else if metadata.is_dir()
                && !pending.symlink_ancestors.contains(&target)
                && visited_symlink_targets.insert(target.clone())
            {
                let mut ancestors = pending.symlink_ancestors.clone();
                if let Ok(current) = pending.path.canonicalize() {
                    ancestors.push(current);
                }
                ancestors.push(target.clone());
                stack.push(PendingDir { path: target, symlink_ancestors: ancestors });
            }
        }
    }
}

fn matches_extension(path: &Path, extensions: Option<&HashSet<&str>>) -> bool {
    extensions.is_none_or(|extensions| {
        path.extension()
            .and_then(|extension| extension.to_str())
            .is_some_and(|extension| extensions.contains(extension))
    })
}

/// Traverses a directory tree non-recursively and collects all regular files.
/// Symbolic links are followed; broken links are skipped.
pub fn collect_files_all(root: &Path, out: &mut Vec<PathBuf>) {
    collect_files(root, None, None, out);
}

/// Collects files matching one extension while following symbolic links.
pub fn collect_files_fast(root: &Path, extension: &str, out: &mut Vec<PathBuf>) {
    let extensions = HashSet::from([extension]);
    collect_files(root, Some(&extensions), None, out);
}

/// Collects files matching any supplied extension while following symbolic links.
pub fn collect_files_fast_multi(root: &Path, extensions: &[&str], out: &mut Vec<PathBuf>) {
    let extensions = extensions.iter().copied().collect::<HashSet<_>>();
    collect_files(root, Some(&extensions), None, out);
}

/// Collects files matching one extension while enforcing `filter` on ordinary
/// paths and on canonical symbolic-link targets.
pub fn collect_files_all_filtered(root: &Path, filter: &Pathfilter, out: &mut Vec<PathBuf>) {
    collect_files(root, None, Some(filter), out);
}

/// Collects files matching one extension while enforcing the filter on ordinary
/// paths and on canonical symbolic-link targets.
pub fn collect_files_filtered(
    root: &Path,
    extension: &str,
    filter: &Pathfilter,
    out: &mut Vec<PathBuf>,
) {
    let extensions = HashSet::from([extension]);
    collect_files(root, Some(&extensions), Some(filter), out);
}

/// Collects files matching any supplied extension while enforcing the filter on
/// ordinary paths and on canonical symbolic-link targets.
pub fn collect_files_multi_filtered(
    root: &Path,
    extensions: &[&str],
    filter: &Pathfilter,
    out: &mut Vec<PathBuf>,
) {
    let extensions = extensions.iter().copied().collect::<HashSet<_>>();
    collect_files(root, Some(&extensions), Some(filter), out);
}
