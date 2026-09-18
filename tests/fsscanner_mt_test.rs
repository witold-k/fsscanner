use fsscanner::fsscanner_mt::{
    process_dir_map, process_dir_map_multi, process_dir_state_and_map,
};
use fsscanner::Result;
use std::io;
use std::path::Path;

fn fail(_: &Path, _: &Path) -> Result<()> {
    Err(io::Error::other("callback failed").into())
}

#[test]
fn process_dir_map_propagates_worker_errors() {
    let output = std::env::temp_dir().join(format!(
        "fsscanner-mt-map-{}",
        std::process::id()
    ));
    let result = process_dir_map(
        "src",
        output.to_str().unwrap(),
        "rs",
        "out",
        fail,
    );
    let _ = std::fs::remove_dir_all(output);

    let error = result.expect_err("callback errors must be propagated");
    assert!(error.to_string().contains("callback failed"));
}

#[test]
fn process_dir_map_multi_propagates_worker_errors() {
    let output = std::env::temp_dir().join(format!(
        "fsscanner-mt-multi-{}",
        std::process::id()
    ));
    let result = process_dir_map_multi(
        "src",
        output.to_str().unwrap(),
        "rs",
        &["one", "two"],
        |_, _| Err(io::Error::other("multi callback failed").into()),
    );
    let _ = std::fs::remove_dir_all(output);

    let error = result.expect_err("callback errors must be propagated");
    assert!(error.to_string().contains("multi callback failed"));
}

#[test]
fn process_dir_state_and_map_propagates_worker_errors() {
    let output = std::env::temp_dir().join(format!(
        "fsscanner-mt-state-{}",
        std::process::id()
    ));
    let result = process_dir_state_and_map(
        0usize,
        "src",
        output.to_str().unwrap(),
        "rs",
        "out",
        |state, _, _| {
            *state += 1;
            Err(io::Error::other("state callback failed").into())
        },
    );
    let _ = std::fs::remove_dir_all(output);

    let error = result.expect_err("callback errors must be propagated");
    assert!(error.to_string().contains("state callback failed"));
}
