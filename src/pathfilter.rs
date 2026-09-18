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
        if self.paths.is_empty() {
            return false;
        }

        if name.is_absolute() {
            let normalized = normalize_path(name);
            return !Self::is_blocked(&normalized)
                && self
                    .paths
                    .iter()
                    .any(|base| normalized.strip_prefix(base).is_ok());
        }

        let normalized = normalize_path(name);
        if Self::is_blocked(&normalized) {
            return false;
        }

        self.paths
            .iter()
            .any(|base| Self::relative_path_stays_within(base, &normalized))
    }

    /// Checks whether a path is allowed to be written to.
    ///
    /// Writes are restricted to the first configured root.
    pub fn can_write(&self, name: &Path) -> bool {
        let Some(base) = self.paths.first() else {
            return false;
        };

        if name.is_absolute() {
            let normalized = normalize_path(name);
            return !Self::is_blocked(&normalized)
                && normalized.strip_prefix(base).is_ok();
        }

        let normalized = normalize_path(name);
        !Self::is_blocked(&normalized)
            && Self::relative_path_stays_within(base, &normalized)
    }

    #[inline]
    fn relative_path_stays_within(base: &Path, path: &Path) -> bool {
        let mut depth = base.components().count();

        for component in path.components() {
            match component {
                Component::ParentDir => {
                    if depth == 0 {
                        return false;
                    }
                    depth -= 1;
                }
                Component::Normal(_) => depth += 1,
                Component::CurDir => {}
                Component::Prefix(_) | Component::RootDir => return false,
            }
        }

        depth >= base.components().count()
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
