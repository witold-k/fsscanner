// SPDX-License-Identifier: Apache-2.0
// Copyright (c) 2026 Witold Kaminski

//! Path filtering and validation utilities.
//!
//! This module provides the [`Pathfilter`] struct, which determines whether a given
//! system path is allowed for reading or writing based on predefined root directories
//! and explicit blocklists.

use crate::pathutils::{from_versioned_project, normalize_path};
use std::fmt;
use std::path::{Component, Path, PathBuf};

const BLOCKED_COMPONENTS: &[&str] = &[
    "buildscripts",
    "build-scripts",
    "common-scripts",
    "commonscripts",
    ".git",
    ".svn",
    ".hg",
];

/// A thread-safe filter used to validate file paths against allowed roots and blocklists.
///
/// Relative paths are interpreted relative to the configured allowed roots, not relative
/// to the process working directory.
#[derive(Debug, Clone)]
pub struct Pathfilter {
    /// Allowed base paths. Any verified path must reside within at least one of these roots.
    paths: Vec<PathBuf>,
}

impl Pathfilter {
    /// Creates a `Pathfilter` rooted at the current working directory.
    ///
    /// # Panics
    ///
    /// Panics if the current working directory cannot be retrieved or canonicalized.
    pub fn from_current_dir() -> Self {
        let cwd = std::env::current_dir()
            .expect("Failed to get current directory")
            .canonicalize()
            .expect("Failed to canonicalize cwd");
        Self::new(vec![cwd])
    }

    /// Creates a `Pathfilter` rooted at the discovered versioned project root.
    pub fn from_versioned_project() -> Self {
        let cwd = std::env::current_dir()
            .expect("Failed to get current directory")
            .canonicalize()
            .expect("Failed to canonicalize cwd");

        let project_root = from_versioned_project(&cwd);
        Self::new(vec![project_root])
    }

    /// Creates a new `Pathfilter` with a custom list of allowed base paths.
    pub fn new(paths: Vec<PathBuf>) -> Self {
        Self {
            paths: paths.into_iter().map(|p| normalize_path(&p)).collect(),
        }
    }

    /// Checks whether a path is allowed to be read.
    ///
    /// Absolute paths must be contained by at least one configured root. Relative
    /// paths are resolved lexically against each configured root. Blocked directory
    /// names are matched as complete path components.
    pub fn contains(&self, name: &Path) -> bool {
        self.paths
            .iter()
            .any(|base| Self::is_allowed_under(base, name))
    }

    /// Checks whether a path is allowed to be written to.
    ///
    /// Writes are restricted to the first configured root.
    pub fn can_write(&self, name: &Path) -> bool {
        self.paths
            .first()
            .is_some_and(|base| Self::is_allowed_under(base, name))
    }

    fn is_allowed_under(base: &Path, name: &Path) -> bool {
        let normalized = if name.is_absolute() {
            normalize_path(name)
        } else {
            normalize_path(&base.join(name))
        };

        !Self::is_blocked(&normalized) && normalized.strip_prefix(base).is_ok()
    }

    /// Checks blocked directory names component by component to avoid substring
    /// false positives such as `my.git-data` or `buildscripts-old`.
    #[inline]
    fn is_blocked(path: &Path) -> bool {
        path.components().any(|component| {
            let Component::Normal(name) = component else {
                return false;
            };

            BLOCKED_COMPONENTS
                .iter()
                .any(|blocked| name == std::ffi::OsStr::new(blocked))
        })
    }
}

impl fmt::Display for Pathfilter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self.paths)
    }
}

#[cfg(test)]
mod tests {
    use super::Pathfilter;
    use std::path::{Path, PathBuf};

    fn filter() -> Pathfilter {
        Pathfilter::new(vec![PathBuf::from("/project")])
    }

    #[cfg(unix)]
    #[test]
    fn accepts_paths_inside_root() {
        let filter = filter();
        assert!(filter.contains(Path::new("/project/src/lib.rs")));
        assert!(filter.contains(Path::new("src/lib.rs")));
    }

    #[cfg(unix)]
    #[test]
    fn rejects_paths_outside_root_and_relative_escape() {
        let filter = filter();
        assert!(!filter.contains(Path::new("/other/src/lib.rs")));
        assert!(!filter.contains(Path::new("../outside.rs")));
    }

    #[cfg(unix)]
    #[test]
    fn relative_paths_are_resolved_against_each_allowed_root() {
        let filter = Pathfilter::new(vec![
            PathBuf::from("/first"),
            PathBuf::from("/second"),
        ]);

        assert!(filter.contains(Path::new("src/lib.rs")));
        assert!(filter.contains(Path::new("/second/src/lib.rs")));
        assert!(!filter.contains(Path::new("/third/src/lib.rs")));
    }

    #[cfg(unix)]
    #[test]
    fn writes_are_restricted_to_primary_root() {
        let filter = Pathfilter::new(vec![
            PathBuf::from("/first"),
            PathBuf::from("/second"),
        ]);

        assert!(filter.can_write(Path::new("out/result.txt")));
        assert!(filter.can_write(Path::new("/first/out/result.txt")));
        assert!(!filter.can_write(Path::new("/second/out/result.txt")));
        assert!(!filter.can_write(Path::new("../escape.txt")));
    }

    #[cfg(unix)]
    #[test]
    fn blocks_vcs_and_build_directories_as_components() {
        let filter = filter();

        for blocked in [".git", ".svn", ".hg", "buildscripts", "build-scripts",
                        "common-scripts", "commonscripts"] {
            let path = PathBuf::from("src").join(blocked).join("file");
            assert!(!filter.contains(&path), "component {blocked} must be blocked");
        }
    }

    #[cfg(unix)]
    #[test]
    fn does_not_block_partial_component_matches() {
        let filter = filter();

        for allowed in [
            "my.git-data/file",
            ".github/workflow.yml",
            "buildscripts-old/file",
            "common-scripts-backup/file",
        ] {
            assert!(filter.contains(Path::new(allowed)), "{allowed} must remain allowed");
        }
    }

    #[test]
    fn empty_filter_rejects_everything() {
        let filter = Pathfilter::new(Vec::new());
        assert!(!filter.contains(Path::new("file")));
        assert!(!filter.can_write(Path::new("file")));
    }
}
