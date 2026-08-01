// SPDX-License-Identifier: Apache-2.0
// Copyright (c) 2026 Witold Kaminski

use crate::threadpool::ThreadPool;
use std::{
    path::{Path, PathBuf},
    sync::Arc,
    sync::Mutex,
};
use crate::Result;
use crate::fsscanner_base::{
    collect_files_fast,
};

/// Processes all files in a directory tree that match a given extension,
/// generating corresponding output paths and invoking a user‑provided callback
/// for each file.
///
/// This function:
/// - Recursively scans `input_root` for files with the given `extension`.
/// - For each matching file, computes its relative path and constructs an
///   output path under `output_root`, replacing the file extension with `suffix`.
/// - Executes the provided `callback` in a thread pool (128 threads),
///   passing `(input_path, output_path)` to it.
/// - Ensures that parent directories for the output file exist before calling
///   the callback.
///
/// # Parameters
///
/// * `input_root`
///   Root directory to scan for input files. All matching files under this
///   directory (recursively) will be processed.
///
/// * `output_root`
///   Root directory where output files should be written. The directory
///   structure mirrors the layout under `input_root`.
///
/// * `extension`
///   File extension to filter input files (e.g. `"txt"`). Only files ending
///   with this extension are passed to the callback.
///
/// * `suffix`
///   The extension to use for generated output files. This replaces the input
///   file’s extension when constructing the output path.
///
/// * `callback`
///   A function or closure that receives `(input_path, output_path)` and
///   performs the actual processing.
///   It must return `Result<()>` and be `Send + Sync + 'static` because it is
///   executed inside a thread pool.
///
/// # Behavior
///
/// 1. All matching files are collected first to avoid holding directory
///    iterators across threads.
/// 2. Each file is processed in parallel using a thread pool.
/// 3. The output path is constructed by:
///    - stripping the `input_root` prefix from the input file,
///    - appending that relative path to `output_root`,
///    - replacing the extension with `suffix`.
/// 4. Parent directories for the output file are created automatically.
/// 5. Errors from the callback are printed to stderr but do not stop execution.
///
/// # Returns
///
/// Always returns `Ok(())` unless the initial file collection fails (which is
/// unlikely unless `collect_files_fast` propagates errors).
///
/// # Example
///
/// ```rust,ignore
/// process_dir_map(
///     "src",
///     "out",
///     "rs",
///     "txt",
///     |input, output| {
///         std::fs::write(output, std::fs::read_to_string(input)?)?;
///         Ok(())
///     },
/// )?;
/// ```
pub fn process_dir_map<F>(
    input_root: &str,
    output_root: &str,
    extension: &str,
    suffix: &str,
    callback: F,
) -> Result<()>
where
    F: Fn(&Path, &Path) -> Result<()> + Send + Sync + 'static,
{
    let pool = ThreadPool::new(128);
    let callback = Arc::new(callback);

    // Pre-collect files
    let mut files = Vec::new();
    collect_files_fast(Path::new(input_root), extension, &mut files);

    // Avoid cloning strings inside the loop
    let input_root = Arc::new(PathBuf::from(input_root));
    let output_root = Arc::new(PathBuf::from(output_root));
    let suffix = Arc::new(suffix.to_string());

    for input_path in files {
        let callback = callback.clone();
        let input_root = input_root.clone();
        let output_root = output_root.clone();
        let suffix = suffix.clone();

        pool.execute(move || {
            let rel = input_path.strip_prefix(&*input_root).unwrap();

            // Preallocate output path buffer
            let mut output_path = PathBuf::with_capacity(
                output_root.as_os_str().len() + rel.as_os_str().len() + 8
            );
            output_path.push(&*output_root);
            output_path.push(rel);
            output_path.set_extension(&*suffix);

            if let Some(parent) = output_path.parent() {
                let _ = std::fs::create_dir_all(parent);
            }

            if let Err(err) = callback(&input_path, &output_path) {
                eprintln!("Error processing {}: {:?}", input_path.display(), err);
            }
        });
    }

    Ok(())
}

pub fn process_dir_map_multi<F>(
    input_root: &str,
    output_root: &str,
    extension: &str,
    suffixes: &[&str],
    callback: F,
) -> Result<()>
where
    F: Fn(&Path, &[PathBuf]) -> Result<()> + Send + Sync + 'static,
{
    let pool = ThreadPool::new(128);
    let callback = Arc::new(callback);

    // Pre-collect files
    let mut files = Vec::new();
    collect_files_fast(Path::new(input_root), extension, &mut files);

    // Avoid cloning strings inside the loop
    let input_root = Arc::new(PathBuf::from(input_root));
    let output_root = Arc::new(PathBuf::from(output_root));
    let suffixes = Arc::new(
        suffixes.iter().map(|s| s.to_string()).collect::<Vec<_>>()
    );

    for input_path in files {
        let callback = callback.clone();
        let input_root = input_root.clone();
        let output_root = output_root.clone();
        let suffixes = suffixes.clone();
        pool.execute(move || {
            let rel = input_path.strip_prefix(&*input_root).unwrap();

            // Build the first output path to determine the parent directory
            let mut first_out = PathBuf::new();
            first_out.push(&*output_root);
            first_out.push(rel);

            // Parent directory is the same for all suffixes
            if let Some(parent) = first_out.parent() {
                let _ = std::fs::create_dir_all(parent);
            }

            // Now build all output paths
            let mut outputs = Vec::with_capacity(suffixes.len());
            for suffix in suffixes.iter() {
                let mut out = first_out.clone();
                out.set_extension(suffix);
                outputs.push(out);
            }

            if let Err(err) = callback(&input_path, &outputs) {
                eprintln!("Error processing {}: {:?}", input_path.display(), err);
            }
        });
    }

    Ok(())
}

/// Processes all files under a given input directory that match a specific extension,
/// invoking a user‑provided callback for each file, and producing a final accumulated state.
///
/// This function:
/// - Recursively collects all files under `input_root` with the given `extension`.
/// - For each file, computes its relative path and maps it into the `output_root`,
///   replacing the file extension with `suffix`.
/// - Calls the provided `callback` with a mutable reference to shared state,
///   the input file path, and the computed output file path.
/// - Runs all callbacks in parallel using a thread pool.
/// - Returns the final state after all files have been processed.
///
/// # Type Parameters
/// - `State`: The accumulator type that is shared and mutated across all callback invocations.
/// - `F`: The callback function type.
///
/// # Parameters
/// - `state`:
///   The initial state value.
///   This state is wrapped in an `Arc<Mutex<_>>` and shared across worker threads.
///   Each callback invocation receives a mutable reference to this state.
///
/// - `input_root`:
///   The root directory to scan for input files.
///   All files under this directory (recursively) with the given `extension` will be processed.
///
/// - `output_root`:
///   The root directory where output files should be mapped.
///   For each input file, its path relative to `input_root` is appended to `output_root`,
///   and its extension is replaced with `suffix`.
///
/// - `extension`:
///   The file extension to filter input files by (e.g., `"txt"`).
///   Only files ending with this extension are passed to the callback.
///
/// - `suffix`:
///   The extension to assign to the output file paths (e.g., `"out"`).
///   This replaces the original extension of each input file.
///
/// - `callback`:
///   A function invoked for each matching file.
///   Signature:
///   `Fn(&mut State, &Path, &Path) -> Result<()>`
///   - First argument: a mutable reference to the shared state
///   - Second argument: the input file path
///   - Third argument: the computed output file path
///     The callback may mutate the shared state and may return an error.
///
/// # Returns
/// Returns the final accumulated `State` after all files have been processed.
///
/// # Errors
/// Returns an error if the final state cannot be extracted from the mutex
/// (e.g., if the mutex was poisoned).
///
/// # Concurrency
/// - Uses a thread pool with 128 worker threads.
/// - The shared state is protected by a `Mutex`, so callbacks are executed in parallel
///   but state mutations are serialized.
/// - File system operations (directory creation, path mapping) occur inside worker threads.
///
/// # Notes
/// - Errors inside callbacks are logged to stderr but do **not** stop processing.
/// - If directory creation fails for an output path, the error is logged and processing continues.
#[allow(clippy::collapsible_if)]
pub fn process_dir_state_and_map<State, F>(
    state: State,
    input_root: &str,
    output_root: &str,
    extension: &str,
    suffix: &str,
    callback: F,
) -> Result<State>
where
    State: Send + 'static,
    F: Fn(&mut State, &Path, &Path) -> Result<()> + Send + Sync + 'static,
{

    let state = Arc::new(Mutex::new(state));
    let callback = Arc::new(callback);

    let input_root = PathBuf::from(input_root);
    let output_root = PathBuf::from(output_root);

    let mut files = Vec::new();
    collect_files_fast(&input_root, extension, &mut files);

    {
        let pool = ThreadPool::new(128);
        for input_path in files {
            let state = Arc::clone(&state);
            let callback = Arc::clone(&callback);
            let input_root = input_root.clone();
            let output_root = output_root.clone();
            let suffix = suffix.to_string();

            pool.execute(move || {
                let rel = match input_path.strip_prefix(&input_root) {
                    Ok(r) => r,
                    Err(e) => {
                        eprintln!("strip_prefix failed for {}: {:?}", input_path.display(), e);
                        return;
                    }
                };

                let mut output_path = output_root.clone();
                output_path.push(rel);
                output_path.set_extension(&suffix);

                if let Some(parent) = output_path.parent() {
                    if let Err(e) = std::fs::create_dir_all(parent) {
                        eprintln!("Failed to create directory {}: {:?}", parent.display(), e);
                        return;
                    }
                }

                let mut guard = match state.lock() {
                    Ok(g) => g,
                    Err(poisoned) => poisoned.into_inner(),
                };

                if let Err(err) = callback(&mut *guard, &input_path, &output_path) {
                    eprintln!("Error processing {}: {:?}", input_path.display(), err);
                }
            });
        }
    }

    let final_state = Arc::try_unwrap(state)
        .map_err(|_| "State still has multiple references")?
        .into_inner()
        .map_err(|_| "State mutex was poisoned")?;

    Ok(final_state)
}

