// SPDX-License-Identifier: Apache-2.0
// Copyright (c) 2026 Witold Kaminski

/// Generic parallel directory processor.
/// - Scans for files with `extension`
/// - Builds output paths under `output_root`
/// - Replaces extension with `suffix`
/// - Calls `callback(input_path, output_path)`
use std::{
    path::Path,
};
use crate::Result;
use crate::fsscanner_base::{
    collect_files_fast,
    collect_files_fast_multi,
};

pub fn process_dir_with_one<F>(
    input_root: &str,
    extension: &str,
    callback: F,
) -> Result<()>
where
    F: Fn(&Path) -> Result<()>
{
    // Collect matching files
    let mut files = Vec::new();
    collect_files_fast(Path::new(input_root), extension, &mut files);

    for input_path in files {
        if let Err(err) = callback(&input_path) {
            eprintln!(
                "Error processing {}: {:?}",
                input_path.display(),
                err
            );
        }
    }

    Ok(())
}

pub fn process_dir_state_with_one<State, F>(
    mut state: State,
    input_root: &str,
    extension: &str,
    callback: F,
) -> Result<State>
where
    F: Fn(&mut State, &Path) -> Result<()>
{
    // Collect matching files
    let mut files = Vec::new();
    collect_files_fast(Path::new(input_root), extension, &mut files);

    for input_path in files {
        if let Err(err) = callback(&mut state, &input_path) {
            eprintln!(
                "Error processing {}: {:?}",
                input_path.display(),
                err
            );
        }
    }

    Ok(state)
}

pub fn process_dir_with_some<F>(
    input_root: &str,
    extensions: &[&str],
    callback: F,
) -> Result<()>
where
    F: Fn(&Path) -> Result<()>,
{
    let mut files = Vec::new();
    collect_files_fast_multi(Path::new(input_root), extensions, &mut files);

    for input_path in files {
        if let Err(err) = callback(&input_path) {
            eprintln!(
                "Error processing {}: {:?}",
                input_path.display(),
                err
            );
        }
    }

    Ok(())
}

pub fn process_dir_state_with_some<State, F>(
    mut state: State,
    input_root: &str,
    extensions: &[&str],
    callback: F,
) -> Result<State>
where
    F: Fn(&mut State, &Path) -> Result<()>,
{
    let mut files = Vec::new();
    collect_files_fast_multi(Path::new(input_root), extensions, &mut files);

    for input_path in files {
        if let Err(err) = callback(&mut state, &input_path) {
            eprintln!(
                "Error processing {}: {:?}",
                input_path.display(),
                err
            );
        }
    }

    Ok(state)
}

