# Run all standard tests
test:
    cargo test --workspace

# Run the database backward compatibility test
# Requires a DB snapshot as described in /test_data/README.md
test-db-compat:
    cd core/infrastructure/persistence && cargo test --test compatibility_test --features db-compat-test
