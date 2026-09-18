use fsscanner::fsscanner_st::{
    process_dir_state_with_one, process_dir_state_with_some, process_dir_with_one,
    process_dir_with_some,
};
use fsscanner::Result;
use std::io;
use std::path::Path;

fn fail(_: &Path) -> Result<()> {
    Err(io::Error::other("callback failed").into())
}

#[test]
fn process_dir_with_one_propagates_callback_error() {
    let result = process_dir_with_one("src", "rs", fail);
    assert!(result.is_err());
}

#[test]
fn process_dir_with_some_propagates_callback_error() {
    let result = process_dir_with_some("src", &["rs"], fail);
    assert!(result.is_err());
}

#[test]
fn process_dir_state_with_one_propagates_callback_error() {
    let result = process_dir_state_with_one(0usize, "src", "rs", |state, _| {
        *state += 1;
        Err(io::Error::other("callback failed").into())
    });

    assert!(result.is_err());
}

#[test]
fn process_dir_state_with_some_propagates_callback_error() {
    let result = process_dir_state_with_some(0usize, "src", &["rs"], |state, _| {
        *state += 1;
        Err(io::Error::other("callback failed").into())
    });

    assert!(result.is_err());
}
