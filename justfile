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
    dprint check
    npx markdownlint-cli '**/*.md' --ignore node_modules

# Run all checks (test + lint)
check: test lint

# Assemble + check
build: assemble check

# Validate commits on branch and build — run before PR
verify: check build

# Format Rust and Markdown
fmt:
    cargo fmt
    dprint fmt
    npx markdownlint-cli '**/*.md' --ignore node_modules --fix

# Generate and open rustdoc
doc:
    cargo doc --open

# Bump version, update changelog, commit, and tag
release:
    git std bump

# Publish crate to crates.io
publish: check
    cargo publish

# Remove build artifacts
clean:
    cargo clean
