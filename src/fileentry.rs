// SPDX-License-Identifier: Apache-2.0
// Copyright (c) 2026 Witold Kaminski

//! File content representation using a localized custom ThreadPool.
//!
//! This module provides the [`FileEntry`] struct, which pairs a file path with its
//! text content. It spawns a temporary, thread pool per batch operation
//! to read files in parallel, ensuring clean and deterministic join behavior via RAII.

use crate::pathfilter::Pathfilter;
use crate::threadpool::ThreadPool;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::io;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::sync::{Arc, Mutex};

/// Represents a file system entry containing its path and content.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileEntry {
    /// The path to the file on the file system.
    pub path: PathBuf,

    /// The UTF-8 encoded text content of the file.
    pub data: String,
}

impl FileEntry {
    /// Creates a new `FileEntry` instance from a file path.
    ///
    /// Returns an error if the file cannot be read as UTF-8 text.
    #[inline(always)]
    pub fn from_path(path: &Path) -> io::Result<Self> {
        let data = std::fs::read_to_string(path)?;

        Ok(Self {
            path: path.to_path_buf(),
            data,
        })
    }

    /// Load data from path, overwriting the old one.
    ///
    /// Returns an error if the file cannot be read as UTF-8 text.
    #[inline(always)]
    pub fn new_load(self) -> io::Result<Self> {
        let data = std::fs::read_to_string(&self.path)?;

        Ok(Self {
            path: self.path,
            data,
        })
    }

    /// Load data from path, overwriting the old one.
    ///
    /// Returns an error if the file cannot be read as UTF-8 text.
    #[inline(always)]
    pub fn load(&mut self) -> io::Result<&mut Self> {
        self.data = std::fs::read_to_string(&self.path)?;
        Ok(self)
    }

    /// Parallelly filters and reads a list of path strings by spinning up
    /// a local `ThreadPool`.
    ///
    /// If `filter` is `Some`, only paths accepted by the filter are included.
    /// If `filter` is `None`, all paths are considered valid and are included.
    pub fn vec_from_filtered_stringvec(
        filter: Option<&Pathfilter>,
        list: Vec<String>,
    ) -> crate::Result<Vec<Self>> {
        let paths = list
            .into_iter()
            .filter(|path| !path.is_empty())
            .map(PathBuf::from)
            .collect();
        Self::vec_from_filtered_pathbufvec(filter, paths)
    }

    /// Parallelly filters and reads a list of `PathBuf` objects.
    ///
    /// All scheduled reads are allowed to finish. If any accepted path cannot
    /// be read as UTF-8 text, the operation returns an error after joining all
    /// workers instead of representing that file as empty.
    pub fn vec_from_filtered_pathbufvec(
        filter: Option<&Pathfilter>,
        list: Vec<PathBuf>,
    ) -> crate::Result<Vec<Self>> {
        if list.is_empty() {
            return Ok(Vec::new());
        }

        let num_threads = std::thread::available_parallelism()
            .map(|count| count.get())
            .unwrap_or(1);
        let chunk_size = list.len().div_ceil(num_threads);
        let results = Arc::new(Mutex::new(Vec::with_capacity(list.len())));
        let errors = Arc::new(Mutex::new(Vec::new()));
        let filter = filter.cloned();
        let pool = ThreadPool::new(num_threads)?;

        for batch in list.chunks(chunk_size) {
            let batch = batch.to_vec();
            let results = Arc::clone(&results);
            let errors = Arc::clone(&errors);
            let filter = filter.clone();

            pool.execute(move || {
                let mut local_results = Vec::with_capacity(batch.len());
                let mut local_errors = Vec::new();

                for path in batch {
                    if !filter.as_ref().is_none_or(|filter| filter.contains(&path)) {
                        continue;
                    }

                    match std::fs::read_to_string(&path) {
                        Ok(data) => local_results.push(Self { path, data }),
                        Err(error) => {
                            local_errors.push(format!("{}: {error}", path.display()));
                        }
                    }
                }

                if !local_results.is_empty() {
                    results
                        .lock()
                        .unwrap_or_else(|poisoned| poisoned.into_inner())
                        .extend(local_results);
                }
                if !local_errors.is_empty() {
                    errors
                        .lock()
                        .unwrap_or_else(|poisoned| poisoned.into_inner())
                        .extend(local_errors);
                }
            })?;
        }

        pool.join()?;

        let errors = Arc::into_inner(errors)
            .ok_or("file read errors still have multiple references")?
            .into_inner()
            .map_err(|_| "file read error mutex was poisoned")?;

        if !errors.is_empty() {
            return Err(io::Error::other(errors.join("\n")).into());
        }

        let mut results = Arc::into_inner(results)
            .ok_or("file results still have multiple references")?
            .into_inner()
            .map_err(|_| "file result mutex was poisoned")?;
        results.shrink_to_fit();
        Ok(results)
    }
}

impl FromStr for FileEntry {
    type Err = io::Error;

    #[inline(always)]
    fn from_str(pathstr: &str) -> Result<Self, Self::Err> {
        Self::from_path(Path::new(pathstr))
    }
}

impl fmt::Display for FileEntry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "=== Path: {} ===\n=== Content: ===\n{}",
            self.path.display(),
            self.data
        )
    }
}
