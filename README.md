# fsscanner

`fsscanner` is a small Rust library for recursively scanning directory trees and processing matching files.

It is intended for tools that need predictable filesystem traversal without pulling in a large dependency tree: source scanners, code generators, build tooling, indexers, batch converters, and similar utilities.

The library provides both low-level file collection helpers and higher-level processing functions. Files can be selected by one or more extensions, processed sequentially, or dispatched to a thread pool. Mapping helpers can mirror an input directory structure into an output tree while replacing file extensions before invoking the user callback.

## Design goals

- **Small dependency surface** — keep the implementation close to the Rust standard library and avoid unnecessary third-party crates. This reduces dependency and supply-chain complexity.
- **Simple, explicit API** — filesystem traversal and file processing remain separate building blocks, while convenience functions cover common batch-processing workflows.
- **Safe traversal of deep trees** — directory walking is iterative rather than recursive, avoiding call-stack growth for deeply nested directory structures.
- **Symlink cycle protection** — symbolic-link directory targets are tracked during traversal so cyclic directory graphs do not result in endless scanning.
- **Efficient filtering** — files can be collected by a single extension, multiple extensions, or without an extension filter.
- **Optional parallel processing** — higher-level helpers can execute independent file-processing callbacks through a lightweight internal thread pool.
- **Path-preserving output mapping** — processing helpers can reproduce the relative input directory structure below a separate output root and replace file suffixes automatically.
- **Library-first design** — `fsscanner` performs traversal and orchestration; application-specific work stays in caller-provided callbacks.
- **Consistent test layout** — tests live under `tests/`, mirror the relative `src/` hierarchy, and use the source filename with a `_test.rs` suffix.

## Main building blocks

### Filesystem scanning

The `fsscanner_base` module contains the basic traversal functions:

- `collect_files_all` — collect all regular files below a root directory.
- `collect_files_fast` — collect files matching one extension.
- `collect_files_fast_multi` — collect files matching any extension in a supplied set.

Traversal uses an explicit internal stack instead of recursive function calls. Normal directories follow a fast path; symbolic-link directories are canonicalized and tracked to prevent traversal cycles.

### Sequential processing

The `fsscanner_st` module provides helpers that scan first and then invoke a callback for each matching file. Variants are available for a single extension or several extensions, with optional caller-owned state.

This is useful when processing is inexpensive, ordering is easier to reason about sequentially, or shared mutable state does not warrant synchronization.

### Parallel processing and output mapping

The `fsscanner_mt` module provides higher-level helpers for parallel batch processing.

Typical workflows can:

1. scan an input tree for matching files,
2. derive each file's path relative to the input root,
3. reproduce that path below a separate output root,
4. replace the file extension with one or more output suffixes, and
5. invoke a user callback in the internal thread pool.

This makes the library useful for workloads such as source transformation, document conversion, code generation, indexing, or other file-oriented pipelines where each input file can largely be processed independently.

## Example

```rust
use std::path::Path;
use fsscanner::fsscanner_base::collect_files_fast;

fn main() {
    let mut files = Vec::new();
    collect_files_fast(Path::new("src"), "rs", &mut files);

    for file in files {
        println!("{}", file.display());
    }
}
```

For processing workloads, the scanner can instead drive a callback directly:

```rust,ignore
use fsscanner::fsscanner_mt::process_dir_map;

process_dir_map(
    "input",
    "output",
    "txt",
    "processed",
    |input, output| {
        let data = std::fs::read_to_string(input)?;
        std::fs::write(output, data)?;
        Ok(())
    },
)?;
```

## Error handling

Traversal is deliberately tolerant of individual filesystem entries that cannot be read: inaccessible entries are skipped so a scan can continue through the remaining tree.

The current higher-level processing helpers likewise report callback failures to standard error and continue processing other files. Callers that require fail-fast or aggregated error semantics should account for that behavior when choosing an API.

## Scope

`fsscanner` is intentionally not a general-purpose filesystem abstraction. Its scope is narrower: efficiently discover files in directory trees and provide reusable orchestration for common file-processing pipelines while keeping the implementation and dependency surface small.

## License

Licensed under the Apache License 2.0. See [LICENSE](LICENSE).
