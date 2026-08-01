// SPDX-License-Identifier: Apache-2.0
// Copyright (c) 2026 Witold Kaminski

use std::env;
use std::fs;
use std::cell::RefCell;
use std::path::Path;
use fsscanner::fsscanner_st::process_dir_with_some;
use fsscanner::Result;

fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();

    if args.len() < 3 {
        eprintln!("Usage: collect_to_md <directory> <suffix1> [suffix2] [suffix3] ...");
        std::process::exit(1);
    }

    let dir = &args[1];
    //let suffixes: Vec<String> = args[2..].iter().map(|s| s.to_string()).collect();
    let suffixes: Vec<&str> = args[2..].iter().map(|s| s.as_str()).collect();
    let buffer = RefCell::new(String::new());

    process_dir_with_some(dir, &suffixes, |filename: &Path| {
        println!("{:?}", filename);
        let mut buffer = buffer.borrow_mut();

        // Add header
        buffer.push_str("# ");
        buffer.push_str(&filename.display().to_string());
        buffer.push('\n');

        // Add file contents
        match fs::read_to_string(filename) {
            Ok(contents) => buffer.push_str(&contents),
            Err(err) => buffer.push_str(&format!("(error reading file: {})", err)),
        }

        buffer.push('\n');

        Ok(())
    })?;

    println!("{}", buffer.borrow());

    Ok(())
}

