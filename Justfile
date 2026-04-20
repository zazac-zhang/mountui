set positional-arguments

# List available recipes
default:
    @just --list

# Build the project
build:
    cargo build

# Check for compilation errors (faster than build)
check:
    cargo check

# Run clippy lints
clippy:
    cargo clippy -- -D warnings

# Format check
fmt:
    cargo fmt --check

# Auto-format code
fmt-fix:
    cargo fmt

# Run tests
test:
    cargo test

# Run the application
run *args:
    cargo run -- {{ args }}

# Clean build artifacts
clean:
    cargo clean

# Full CI check (fmt + clippy + build + test)
ci: fmt clippy build test
