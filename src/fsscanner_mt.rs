// SPDX-License-Identifier: Apache-2.0
// Copyright (c) 2026 Witold Kaminski

use crate::fsscanner_base::collect_files_fast;
use crate::threadpool::ThreadPool;
use crate::Result;
use std::error::Error;
use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

#[derive(Debug)]
struct ProcessingErrors(Vec<String>);

impl fmt::Display for ProcessingErrors {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} parallel processing error(s)", self.0.len())?;
        for error in &self.0 {
            write!(f, "\n- {error}")?;
        }
        Ok(())
    }
}

impl Error for ProcessingErrors {}

fn worker_count() -> usize {
    std::thread::available_parallelism()
        .map(|count| count.get())
        .unwrap_or(1)
}

fn finish(
    pool: ThreadPool,
    errors: Arc<Mutex<Vec<String>>>,
) -> Result<()> {
    pool.join()?;

    let errors = Arc::into_inner(errors)
        .ok_or("processing errors still have multiple references")?
        .into_inner()
        .map_err(|_| "processing error mutex was poisoned")?;

    if errors.is_empty() {
        Ok(())
    } else {
        Err(ProcessingErrors(errors).into())
    }
}

fn record_error(errors: &Mutex<Vec<String>>, path: &Path, error: impl fmt::Display) {
    let mut errors = errors.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    errors.push(format!("{}: {error}", path.display()));
}

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
    let pool = ThreadPool::new(worker_count())?;
    let callback = Arc::new(callback);
    let errors = Arc::new(Mutex::new(Vec::new()));

    let input_root = Arc::new(PathBuf::from(input_root));
    let output_root = Arc::new(PathBuf::from(output_root));
    let suffix = Arc::new(suffix.to_string());

    let mut files = Vec::new();
    collect_files_fast(&input_root, extension, &mut files);

    for input_path in files {
        let callback = Arc::clone(&callback);
        let input_root = Arc::clone(&input_root);
        let output_root = Arc::clone(&output_root);
        let suffix = Arc::clone(&suffix);
        let errors = Arc::clone(&errors);

        pool.execute(move || {
            let rel = match input_path.strip_prefix(&*input_root) {
                Ok(rel) => rel,
                Err(error) => {
                    record_error(&errors, &input_path, error);
                    return;
                }
            };

            let mut output_path = PathBuf::with_capacity(
                output_root.as_os_str().len() + rel.as_os_str().len() + 8,
            );
            output_path.push(&*output_root);
            output_path.push(rel);
            output_path.set_extension(&*suffix);

            if let Some(parent) = output_path.parent()
                && let Err(error) = std::fs::create_dir_all(parent)
            {
                record_error(&errors, &input_path, error);
                return;
            }

            if let Err(error) = callback(&input_path, &output_path) {
                record_error(&errors, &input_path, error);
            }
        })?;
    }

    finish(pool, errors)
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
    let pool = ThreadPool::new(worker_count())?;
    let callback = Arc::new(callback);
    let errors = Arc::new(Mutex::new(Vec::new()));

    let input_root = Arc::new(PathBuf::from(input_root));
    let output_root = Arc::new(PathBuf::from(output_root));
    let suffixes = Arc::new(
        suffixes.iter().map(|suffix| suffix.to_string()).collect::<Vec<_>>(),
    );

    let mut files = Vec::new();
    collect_files_fast(&input_root, extension, &mut files);

    for input_path in files {
        let callback = Arc::clone(&callback);
        let input_root = Arc::clone(&input_root);
        let output_root = Arc::clone(&output_root);
        let suffixes = Arc::clone(&suffixes);
        let errors = Arc::clone(&errors);

        pool.execute(move || {
            let rel = match input_path.strip_prefix(&*input_root) {
                Ok(rel) => rel,
                Err(error) => {
                    record_error(&errors, &input_path, error);
                    return;
                }
            };

            let mut first_out = output_root.as_ref().clone();
            first_out.push(rel);

            if let Some(parent) = first_out.parent()
                && let Err(error) = std::fs::create_dir_all(parent)
            {
                record_error(&errors, &input_path, error);
                return;
            }

            let outputs = suffixes
                .iter()
                .map(|suffix| {
                    let mut output = first_out.clone();
                    output.set_extension(suffix);
                    output
                })
                .collect::<Vec<_>>();

            if let Err(error) = callback(&input_path, &outputs) {
                record_error(&errors, &input_path, error);
            }
        })?;
    }

    finish(pool, errors)
}

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
    let pool = ThreadPool::new(worker_count())?;
    let state = Arc::new(Mutex::new(state));
    let callback = Arc::new(callback);
    let errors = Arc::new(Mutex::new(Vec::new()));

    let input_root = Arc::new(PathBuf::from(input_root));
    let output_root = Arc::new(PathBuf::from(output_root));
    let suffix = Arc::new(suffix.to_string());

    let mut files = Vec::new();
    collect_files_fast(&input_root, extension, &mut files);

    for input_path in files {
        let state = Arc::clone(&state);
        let callback = Arc::clone(&callback);
        let input_root = Arc::clone(&input_root);
        let output_root = Arc::clone(&output_root);
        let suffix = Arc::clone(&suffix);
        let errors = Arc::clone(&errors);

        pool.execute(move || {
            let rel = match input_path.strip_prefix(&*input_root) {
                Ok(rel) => rel,
                Err(error) => {
                    record_error(&errors, &input_path, error);
                    return;
                }
            };

            let mut output_path = output_root.as_ref().clone();
            output_path.push(rel);
            output_path.set_extension(&*suffix);

            if let Some(parent) = output_path.parent()
                && let Err(error) = std::fs::create_dir_all(parent)
            {
                record_error(&errors, &input_path, error);
                return;
            }

            let mut state = state.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
            if let Err(error) = callback(&mut *state, &input_path, &output_path) {
                record_error(&errors, &input_path, error);
            }
        })?;
    }

    pool.join()?;

    let errors = Arc::into_inner(errors)
        .ok_or("processing errors still have multiple references")?
        .into_inner()
        .map_err(|_| "processing error mutex was poisoned")?;

    if !errors.is_empty() {
        return Err(ProcessingErrors(errors).into());
    }

    let state = Arc::into_inner(state)
        .ok_or("state still has multiple references")?
        .into_inner()
        .map_err(|_| "state mutex was poisoned")?;

    Ok(state)
}
