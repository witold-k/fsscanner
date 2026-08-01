// SPDX-License-Identifier: Apache-2.0
// Copyright (c) 2026 Witold Kaminski

use fsscanner::fsscanner_mt::process_dir_map;


#[cfg(test)]
mod tests {
    use super::*;
    use std::path::{Path, PathBuf};
    use std::sync::{Arc, Mutex};

    // A simple local walker (same logic as your optimized version)
    fn collect_rs_files(root: &Path, out: &mut Vec<PathBuf>) {
        let mut stack = vec![root.to_path_buf()];

        while let Some(dir) = stack.pop() {
            let entries = match std::fs::read_dir(&dir) {
                Ok(e) => e,
                Err(_) => continue,
            };

            for entry in entries.flatten() {
                let path = entry.path();
                let ft = match entry.file_type() {
                    Ok(ft) => ft,
                    Err(_) => continue,
                };

                if ft.is_dir() {
                    stack.push(path);
                } else if ft.is_file() &&
                    path.extension().map(|e| e == "rs").unwrap_or(false) {
                        out.push(path);
                    }
            }
        }
    }

    #[test]
    fn test_process_dir_map_scans_crate_rs_files() {
        // The root of THIS crate
        let crate_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));

        // Collect expected files using our local walker
        let mut expected = Vec::new();
        collect_rs_files(&crate_root, &mut expected);
        expected.sort();

        // Track files seen by process_dir_map
        let seen = Arc::new(Mutex::new(Vec::<PathBuf>::new()));

        // Run the directory processor
        process_dir_map(
            crate_root.to_str().unwrap(),
            "/dev/null", // output_root unused for this test
            "rs",
            "ignored",
            {
                let seen = seen.clone();
                move |input, _output| {
                    seen.lock().unwrap().push(input.to_path_buf());
                    Ok(())
                }
            },
        )
        .unwrap();

        // Compare results
        let mut seen = seen.lock().unwrap().clone();
        seen.sort();

        assert_eq!(seen, expected);
    }
}

