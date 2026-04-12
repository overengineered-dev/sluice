default:
    @just --list

fmt:
    cargo fmt --all

fmt-check:
    cargo fmt --all -- --check

lint:
    cargo clippy --all-targets --all-features -- -D warnings

test:
    cargo test --all

bench:
    cargo bench -p sluice

fetch-chunk:
    ./scripts/fetch-chunk.sh

regen-fixture:
    cargo run --example gen_fixture -- fixtures/chunk-latest.gz fixtures/chunk-sample.gz 500

fetch-full:
    mkdir -p fixtures/full
    curl -L -o fixtures/full/nexus-maven-repository-index.gz \
        https://repo1.maven.org/maven2/.index/nexus-maven-repository-index.gz

run-chunk:
    cargo run --release -p sluice-cli -- --stats fixtures/chunk-latest.gz

run-full:
    cargo run --release -p sluice-cli -- --stats fixtures/full/nexus-maven-repository-index.gz > /tmp/sluice-full.jsonl
