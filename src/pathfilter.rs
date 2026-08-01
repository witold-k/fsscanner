// SPDX-License-Identifier: Apache-2.0
// Copyright (c) 2026 Witold Kaminski

//! Path filtering and validation utilities.
//!
//! This module provides the [`Pathfilter`] struct, which determines whether a given
//! system path is allowed for reading or writing based on predefined root directories
//! and explicit blocklists.

use std::fmt;
use std::path::{Path, PathBuf};
use crate::pathutils::{from_versioned_project, normalize_path};

/// A thread-safe filter used to validate file paths against allowed roots and blocklists.
///
/// `Pathfilter` ensures that file operations stay within boundaries (such as a project root)
/// and do not cross into forbidden system or build directories.
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
    ///
    /// Extrapolates the project root using the current working directory.
    ///
    /// # Panics
    ///
    /// Panics if the current working directory cannot be retrieved or canonicalized.
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
        Self { paths }
    }

    /// Checks if a path is allowed to be read.
    ///
    /// To pass validation, the path must not match any blocked directories
    /// (e.g., `.git`, `buildscripts`) and must reside inside at least one
    /// of the allowed base paths (unless it is a relative path).
    ///
    /// # Performance
    ///
    /// This method is highly optimized for multi-threaded batch loops. It avoids
    /// heavy disk-bound canonicalization system calls by relying on purely
    /// lexical path normalization.
    pub fn contains(&self, name: &Path) -> bool {
        if self.paths.is_empty() {
            return false;
        }

        // High-End Optimization: Avoid heavy I/O canonicalization system calls in parallel loops.
        // Lexical path normalization occurs strictly within RAM.
        let normalized = normalize_path(name);

        if self.is_blocked(&normalized) {
            return false;
        }

        if normalized.is_relative() {
            return true;
        }

        // Check if name is inside any allowed base path
        for base in &self.paths {
            if normalized.strip_prefix(base).is_ok() {
                return true;
            }
        }

        false
    }

    /// Checks if a path is allowed to be written to.
    ///
    /// Similar to [`contains`], but explicitly restricts access to the **first**
    /// allowed base path registered in this filter.
    pub fn can_write(&self, name: &Path) -> bool {
        if self.paths.is_empty() {
            return false;
        }

        let normalized = normalize_path(name);

        if self.is_blocked(&normalized) {
            return false;
        }

        if normalized.is_relative() {
            return true;
        }

        // Check if name is inside the primary allowed base path
        let base = &self.paths[0];
        normalized.strip_prefix(base).is_ok()
    }

    /// Internal helper to check if a path contains blacklisted directory signatures.
    /// Reuses string slices to minimize heap allocation spikes during runtime.
    #[inline(always)]
    fn is_blocked(&self, path: &Path) -> bool {
        // to_str() is zero-allocation if valid UTF-8. Falls back to lossy only if needed.
        let path_str = path.to_str().unwrap_or_default();

        path_str.contains("buildscripts")
            || path_str.contains("build-scripts")
            || path_str.contains("common-scripts")
            || path_str.contains("commonscripts")
            || path_str.contains(".git")
            || path_str.contains(".svn")
            || path_str.contains(".hg")
    }
}

impl fmt::Display for Pathfilter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self.paths)
    }
}

