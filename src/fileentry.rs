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
    /// If the file cannot be read, the content is set to an empty string.
    #[inline(always)]
    pub fn from_path(path: &Path) -> Self {
        let data = std::fs::read_to_string(path).unwrap_or_default();

        Self {
            path: path.to_path_buf(),
            data,
        }
    }

    /// Parallelly filters and reads a list of path strings by spinning up
    /// a local `ThreadPool`.
    ///
    /// If `filter` is `Some`, only paths accepted by the filter are included.
    /// If `filter` is `None`, all paths are considered valid and are included.
    pub fn vec_from_filtered_stringvec(
        filter: Option<&Pathfilter>,
        list: Vec<String>,
    ) -> Vec<Self> {
        if list.is_empty() {
            return Vec::new();
        }

        let num_threads = std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(4);

        let max_capacity = list.len();
        let results = Arc::new(Mutex::new(Vec::with_capacity(max_capacity)));

        // Clone the filter itself so it can safely be moved into worker threads.
        // `None` means that no filtering should be performed.
        let filter = filter.cloned();

        let chunk_size = (max_capacity + num_threads - 1).div_ceil(num_threads);

        {
            let pool = ThreadPool::new(num_threads);
            let mut iterator = list.into_iter();

            loop {
                let batch: Vec<String> = iterator
                    .by_ref()
                    .take(chunk_size)
                    .filter(|s| !s.is_empty())
                    .collect();

                if batch.is_empty() {
                    break;
                }

                let results_clone = Arc::clone(&results);
                let filter_clone = filter.clone();

                pool.execute(move || {
                    let mut local_buf = Vec::with_capacity(batch.len());

                    for pathstr in batch {
                        let path_ref = Path::new(&pathstr);

                        // No filter means every path is valid.
                        let is_valid = filter_clone
                            .as_ref()
                            .is_none_or(|filter| filter.contains(path_ref));

                        if is_valid {
                            let data =
                                std::fs::read_to_string(path_ref).unwrap_or_default();

                            local_buf.push(Self {
                                path: PathBuf::from(pathstr),
                                data,
                            });
                        }
                    }

                    if !local_buf.is_empty() {
                        let mut guard = results_clone.lock().unwrap();
                        guard.extend(local_buf);
                    }
                });
            }
        }

        let mut final_vec = Arc::into_inner(results)
            .unwrap()
            .into_inner()
            .unwrap();

        final_vec.shrink_to_fit();
        final_vec
    }

    /// Parallelly filters and reads a list of `PathBuf` objects by spinning up
    /// a local `ThreadPool`.
    ///
    /// If `filter` is `Some`, only paths accepted by the filter are included.
    /// If `filter` is `None`, all paths are considered valid and are included.
    pub fn vec_from_filtered_pathbufvec(
        filter: Option<&Pathfilter>,
        list: Vec<PathBuf>,
    ) -> Vec<Self> {
        if list.is_empty() {
            return Vec::new();
        }

        let num_threads = std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(4);

        let max_capacity = list.len();
        let results = Arc::new(Mutex::new(Vec::with_capacity(max_capacity)));

        // Clone the filter itself so it can safely be moved into worker threads.
        // `None` means that no filtering should be performed.
        let filter = filter.cloned();

        let chunk_size = (max_capacity + num_threads - 1).div_ceil(num_threads);

        {
            let pool = ThreadPool::new(num_threads);
            let mut iterator = list.into_iter();

            loop {
                let batch: Vec<PathBuf> =
                    iterator.by_ref().take(chunk_size).collect();

                if batch.is_empty() {
                    break;
                }

                let results_clone = Arc::clone(&results);
                let filter_clone = filter.clone();

                pool.execute(move || {
                    let mut local_buf = Vec::with_capacity(batch.len());

                    for path in batch {
                        // No filter means every path is valid.
                        let is_valid = filter_clone
                            .as_ref()
                            .is_none_or(|filter| filter.contains(&path));

                        if is_valid {
                            let data =
                                std::fs::read_to_string(&path).unwrap_or_default();

                            local_buf.push(Self { path, data });
                        }
                    }

                    if !local_buf.is_empty() {
                        let mut guard = results_clone.lock().unwrap();
                        guard.extend(local_buf);
                    }
                });
            }
        }

        let mut final_vec = Arc::into_inner(results)
            .unwrap()
            .into_inner()
            .unwrap();

        final_vec.shrink_to_fit();
        final_vec
    }
}

impl FromStr for FileEntry {
    type Err = std::convert::Infallible;

    #[inline(always)]
    fn from_str(pathstr: &str) -> Result<Self, Self::Err> {
        let path = PathBuf::from(pathstr);
        let data = std::fs::read_to_string(&path).unwrap_or_default();

        Ok(Self { path, data })
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
