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


#[test]
fn process_dir_map_preserves_nested_path_and_replaces_suffix() {
    let base = std::env::temp_dir().join(format!("fsscanner-map-path-{}", std::process::id()));
    let input = base.join("input");
    let output = base.join("output");
    let nested = input.join("a/b");
    let _ = std::fs::remove_dir_all(&base);
    std::fs::create_dir_all(&nested).unwrap();
    std::fs::write(nested.join("source.rs"), "content").unwrap();

    let expected = output.join("a/b/source.out");
    process_dir_map(
        input.to_str().unwrap(),
        output.to_str().unwrap(),
        "rs",
        "out",
        move |_, mapped| {
            assert_eq!(mapped, expected);
            Ok(())
        },
    )
    .unwrap();

    std::fs::remove_dir_all(base).unwrap();
}

#[test]
fn process_dir_map_multi_builds_all_requested_suffixes() {
    let base = std::env::temp_dir().join(format!("fsscanner-map-multi-path-{}", std::process::id()));
    let input = base.join("input");
    let output = base.join("output");
    let _ = std::fs::remove_dir_all(&base);
    std::fs::create_dir_all(&input).unwrap();
    std::fs::write(input.join("source.rs"), "content").unwrap();

    let expected_output = output.clone();
    process_dir_map_multi(
        input.to_str().unwrap(),
        output.to_str().unwrap(),
        "rs",
        &["one", "two"],
        move |_, mapped| {
            assert_eq!(
                mapped,
                &[
                    expected_output.join("source.one"),
                    expected_output.join("source.two"),
                ]
            );
            Ok(())
        },
    )
    .unwrap();

    std::fs::remove_dir_all(base).unwrap();
}
