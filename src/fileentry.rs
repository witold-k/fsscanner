// SPDX-License-Identifier: Apache-2.0
// Copyright (c) 2026 Witold Kaminski

//! High-performance file content representation using a localized custom ThreadPool.
//!
//! This module provides the [`FileEntry`] struct, which pairs a file path with its
//! text content. It spawns a temporary, highly efficient thread pool per batch operation
//! to read files in parallel, ensuring clean and deterministic join behavior via RAII.

use serde::{Deserialize, Serialize};
use std::fmt;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::sync::{Arc, Mutex};
use crate::pathfilter::Pathfilter;
use crate::threadpool::ThreadPool;

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
    #[inline(always)]
    pub fn from_path(path: &Path) -> Self {
        let data = std::fs::read_to_string(path).unwrap_or_default();
        Self {
            path: path.to_path_buf(),
            data,
        }
    }

    /// Parallelly filters and reads a list of path strings by spinning up a local `ThreadPool`.
    pub fn vec_from_filtered_stringvec(
        filter: &Pathfilter,
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
        let filter = Arc::new(filter.clone());

        // Berechnung der Batch-Größe
        let chunk_size = (max_capacity + num_threads - 1).div_ceil(num_threads);

        {
            let pool = ThreadPool::new(num_threads);

            // Optimization: Wir nutzen into_iter(), um die Ownership direkt zu besitzen.
            // Über einen regulären Iterator teilen wir die Elemente in exakte Vektor-Batches auf.
            let mut iterator = list.into_iter();

            loop {
                // Erstellt ein Paket (Batch) mit maximal `chunk_size` Elementen
                let batch: Vec<String> = iterator
                    .by_ref()
                    .take(chunk_size)
                    .filter(|s| !s.is_empty())
                    .collect();

                if batch.is_empty() {
                    break;
                }

                let results_clone = Arc::clone(&results);
                let filter_clone = Arc::clone(&filter);

                pool.execute(move || {
                    let mut local_buf = Vec::with_capacity(batch.len());

                    for pathstr in batch {
                        let path_ref = Path::new(&pathstr);
                        if filter_clone.contains(path_ref) {
                            let data = std::fs::read_to_string(path_ref).unwrap_or_default();
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

        let mut final_vec = Arc::into_inner(results).unwrap().into_inner().unwrap();
        final_vec.shrink_to_fit();
        final_vec
    }

    /// Parallelly filters and reads a list of `PathBuf` objects by spinning up a local `ThreadPool`.
    pub fn vec_from_filtered_pathbufvec(
        filter: &Pathfilter,
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
        let filter = Arc::new(filter.clone());
        let chunk_size = (max_capacity + num_threads - 1).div_ceil(num_threads);

        {
            let pool = ThreadPool::new(num_threads);
            let mut iterator = list.into_iter();

            loop {
                let batch: Vec<PathBuf> = iterator.by_ref().take(chunk_size).collect();
                if batch.is_empty() {
                    break;
                }

                let results_clone = Arc::clone(&results);
                let filter_clone = Arc::clone(&filter);

                pool.execute(move || {
                    let mut local_buf = Vec::with_capacity(batch.len());

                    for path in batch {
                        if filter_clone.contains(&path) {
                            let data = std::fs::read_to_string(&path).unwrap_or_default();
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

        let mut final_vec = Arc::into_inner(results).unwrap().into_inner().unwrap();
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

