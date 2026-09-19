use fsscanner::pathfilter::Pathfilter;
use std::path::{Path, PathBuf};

fn filter() -> Pathfilter {
    Pathfilter::new(vec![PathBuf::from("/project")])
}

#[cfg(unix)]
#[test]
fn accepts_paths_inside_root() {
    let filter = filter();
    assert!(filter.contains(Path::new("/project/src/lib.rs")));
    assert!(filter.contains(Path::new("src/lib.rs")));
}

#[cfg(unix)]
#[test]
fn rejects_paths_outside_root_and_relative_escape() {
    let filter = filter();
    assert!(!filter.contains(Path::new("/other/src/lib.rs")));
    assert!(!filter.contains(Path::new("../outside.rs")));
}

#[cfg(unix)]
#[test]
fn relative_paths_are_resolved_against_each_allowed_root() {
    let filter = Pathfilter::new(vec![PathBuf::from("/first"), PathBuf::from("/second")]);

    assert!(filter.contains(Path::new("src/lib.rs")));
    assert!(filter.contains(Path::new("/second/src/lib.rs")));
    assert!(!filter.contains(Path::new("/third/src/lib.rs")));
}

#[cfg(unix)]
#[test]
fn writes_are_restricted_to_primary_root() {
    let filter = Pathfilter::new(vec![PathBuf::from("/first"), PathBuf::from("/second")]);

    assert!(filter.can_write(Path::new("out/result.txt")));
    assert!(filter.can_write(Path::new("/first/out/result.txt")));
    assert!(!filter.can_write(Path::new("/second/out/result.txt")));
    assert!(!filter.can_write(Path::new("../escape.txt")));
}

#[cfg(unix)]
#[test]
fn blocks_vcs_and_build_directories_as_components() {
    let filter = filter();

    for blocked in [
        ".git", ".svn", ".hg", "buildscripts", "build-scripts", "common-scripts", "commonscripts",
    ] {
        let path = PathBuf::from("src").join(blocked).join("file");
        assert!(!filter.contains(&path), "component {blocked} must be blocked");
    }
}

#[cfg(unix)]
#[test]
fn does_not_block_partial_component_matches() {
    let filter = filter();

    for allowed in [
        "my.git-data/file",
        ".github/workflow.yml",
        "buildscripts-old/file",
        "common-scripts-backup/file",
    ] {
        assert!(filter.contains(Path::new(allowed)), "{allowed} must remain allowed");
    }
}

#[test]
fn empty_filter_rejects_everything() {
    let filter = Pathfilter::new(Vec::new());
    assert!(!filter.contains(Path::new("file")));
    assert!(!filter.can_write(Path::new("file")));
}


#[cfg(unix)]
#[test]
fn resolved_symlink_target_must_stay_inside_root() {
    use std::os::unix::fs::symlink;

    let base = std::env::temp_dir().join(format!("fsscanner-filter-{}", std::process::id()));
    let root = base.join("root");
    let outside = base.join("outside");
    let _ = std::fs::remove_dir_all(&base);
    std::fs::create_dir_all(&root).unwrap();
    std::fs::create_dir_all(&outside).unwrap();
    std::fs::write(root.join("inside.txt"), "inside").unwrap();
    std::fs::write(outside.join("outside.txt"), "outside").unwrap();
    symlink(root.join("inside.txt"), root.join("inside-link")).unwrap();
    symlink(outside.join("outside.txt"), root.join("outside-link")).unwrap();

    let filter = Pathfilter::new(vec![root.clone()]);
    assert!(filter.contains_resolved(&root.join("inside-link")));
    assert!(!filter.contains_resolved(&root.join("outside-link")));

    std::fs::remove_dir_all(base).unwrap();
}
