#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use fsscanner::fileentry::FileEntry;
    use fsscanner::pathfilter::Pathfilter;

    #[test]
    fn test_from_path_success() {
        // Cargo.toml exists in the root of every standard Rust package directory
        let path = PathBuf::from("Cargo.toml");
        let entry = FileEntry::from_path(&path).expect("Cargo.toml must be readable");

        assert_eq!(entry.path, path);
        // The file content must contain the standard package header configuration
        assert!(
            entry.data.contains("[package]"),
            "Cargo.toml should contain the '[package]' header string"
        );
    }

    #[test]
    fn test_from_path_non_existent_file() {
        // A path that is guaranteed not to exist in your project tree
        let path = PathBuf::from("this_file_does_not_exist_at_all.txt");

        let result = FileEntry::from_path(&path);

        assert!(result.is_err(), "missing files must return an I/O error");
    }

    #[test]
    fn test_load_propagates_read_errors() {
        let mut entry = FileEntry {
            path: PathBuf::from("this_file_does_not_exist_at_all.txt"),
            data: String::from("keep me"),
        };

        assert!(entry.load().is_err());
        assert_eq!(entry.data, "keep me");
    }

    #[test]
    fn test_new_load_propagates_read_errors() {
        let entry = FileEntry {
            path: PathBuf::from("this_file_does_not_exist_at_all.txt"),
            data: String::new(),
        };

        assert!(entry.new_load().is_err());
    }

    #[test]
    fn test_from_str_trait() {
        // Test the FromStr trait using the standard .parse() method interface
        let path_str = "Cargo.toml";

        let entry: Result<FileEntry, _> = path_str.parse();

        assert!(entry.is_ok());
        let entry = entry.unwrap();
        assert_eq!(entry.path, PathBuf::from(path_str));
        assert!(entry.data.contains("[package]"));
    }

    #[test]
    fn test_display_formatting() {
        let path = PathBuf::from("Cargo.toml");
        let entry = FileEntry {
            path,
            data: String::from("[package]\nname = \"test\""),
        };

        let display_string = format!("{}", entry);

        // Verifies the structural layout matching your exact fmt::Display tokens
        let expected = "=== Path: Cargo.toml ===\n=== Content: ===\n[package]\nname = \"test\"";
        assert_eq!(display_string, expected);
    }

    #[test]
    fn test_vec_from_filtered_stringvec() {
        let str_list = vec![
            String::from("Cargo.toml"),
            String::from("this_file_does_not_exist_at_all.txt"),
        ];

        // If Pathfilter::new() requires specialized parameters, adjust this initialization
        let filter = Pathfilter::from_versioned_project();

        let entries = FileEntry::vec_from_filtered_stringvec(Some(&filter), str_list);

        // If your filter configuration permits Cargo.toml, check the item exists in the collection
        let target_path = PathBuf::from("Cargo.toml");
        if filter.contains(&target_path) {
            let found = entries.iter().any(|e| e.path == target_path && e.data.contains("[package]"));
            assert!(found, "Cargo.toml was not correctly extracted or evaluated into the target vector");
        }
    }

    #[test]
    fn test_vec_from_filtered_pathbufvec() {
        let path_list = vec![
            PathBuf::from("Cargo.toml"),
        ];

        let filter = Pathfilter::from_versioned_project();
        let entries = FileEntry::vec_from_filtered_pathbufvec(Some(&filter), path_list);

        let target_path = PathBuf::from("Cargo.toml");
        if filter.contains(&target_path) {
            assert!(!entries.is_empty(), "The resulting filtered entry vector should not be empty");
            assert_eq!(entries[0].path, target_path);
            assert!(entries[0].data.contains("[package]"));
        }
    }
}

