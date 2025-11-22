# Default recipe when running `just`
default: fix

# Apply both fmt fix and clippy fix
fix:
    cargo fmt --all
    cargo clippy --fix --allow-dirty --allow-staged

# Run all tests
test:
    cargo test --all-features --all-targets

# Run tests with verbose output
test-verbose:
    cargo test --all-features --all-targets -- --nocapture

# Check code without fixing
check:
    cargo check --all-features

# Clippy check without fixing
clippy:
    cargo clippy --all-features

# Format check without applying
fmt-check:
    cargo fmt --all -- --check

# Build in release mode
build:
    cargo build --release

# Clean build artifacts
clean:
    cargo clean