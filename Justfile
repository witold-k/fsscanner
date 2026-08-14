# SPDX-License-Identifier: Apache-2.0
# Copyright (c) 2026 Witold Kaminski

user_name        := env("USER")
current_location := justfile()
current_dir      := justfile_directory()
module_name      := file_name(current_dir)
# target_dir       := `cargo metadata --no-deps --format-version=1 | jq -r '.target_directory'`

default: build

build:
    cargo build
    RUST_BACKTRACE=1 cargo test
    cargo clippy

build-jvm:
    RUSTFLAGS="-C panic=unwind" cargo jvm build

build-java-c:
    cargo build --target wasm32-wasip1 --release
    cd java && mvn -f pom-chicory.xml install
    java -jar java/target/fsscanner-runner-1.0-SNAPSHOT.jar

build-java-w:
    cargo build --target wasm32-wasip2 --release
    cd java && mvn -f pom-wasmtime.xml install
    java --enable-native-access=ALL-UNNAMED -jar java/target/fsscanner-wasmtime.jar

fix:
    aifix -t fix_rust_code -f {{current_dir}} -f {{current_dir}}/..

doc:
    aifix -t doc_rust_code -f {{current_dir}} -f {{current_dir}}/..

release:
	RUST_BACKTRACE=1 cargo build --release
	cp target/release/collect_to_md ~/bin/

install-local:
    cp target/release/collect_to_md ~/bin/

clean:
	@cargo clean -p {{module_name}}
	@cd java && mvn -f pom-chicory.xml clean
	@cd java && mvn -f pom-wasmtime.xml clean

cover:
	CARGO_INCREMENTAL=0 RUSTFLAGS='-Cinstrument-coverage' LLVM_PROFILE_FILE='target/coverage/cargo-test-%p-%m.profraw' cargo test
	grcov . --binary-path ./target/debug/deps/ -s . -t html --branch --ignore-not-existing --ignore '../*' --ignore "/*" -o target/coverage/html
	firefox target/coverage/html/index.html

