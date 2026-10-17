@_default:
    just --list

# Compile
assemble:
    cargo build

# Run tests
test:
    cargo test

# Lint and format check
lint:
    cargo clippy -- -D warnings
    cargo fmt -- --check
    prim fmt --check --exclude 'tests/fixtures/**'
    prim lint .
    shellcheck install.sh bootstrap
    python3 scripts/test-precommit-hook.py

# Pre-push checks: changed-file hygiene, Clippy, and type checking.
sanity:
    prim lint --since "$(git rev-parse --abbrev-ref --symbolic-full-name '@{upstream}' 2>/dev/null || printf 'origin/main')"
    cargo clippy --all-targets -- -D warnings
    cargo check --all-targets --locked

# Validate commits in a pull request range.
ci-commits base head:
    git std lint --range "{{base}}..{{head}}"

# Check all targets against the minimum supported Rust version.
msrv-check:
    cargo +1.85.0 check --locked --all-targets

# Run all checks (test + lint)
check: test lint docs-check

# Assemble + check
build: assemble check

# Validate commits on branch and build — run before PR
verify: check build

# Format Rust and Markdown
fmt:
    cargo fmt
    prim fmt --exclude 'tests/fixtures/**'

# Generate and open rustdoc
doc:
    cargo doc --open

# Generate man pages into target/man/ (one .1 per command)
man:
    cargo run --example mangen --release --quiet

# Build the mdBook docs into book/
book:
    mdbook build

# Check repository links, rendered book and publication isolation
docs-check:
    python3 scripts/test-check-docs.py
    python3 scripts/check-docs.py

# Serve the mdBook docs with live reload (opens in browser)
book-serve:
    mdbook serve --open

# Bump version, update changelog, commit, and tag
release:
    git std bump

# Publish crate to crates.io
publish: check
    cargo publish

# Remove build artifacts
clean:
    cargo clean
