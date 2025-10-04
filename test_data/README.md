# Test Data Artifacts

This directory contains data snapshots and other artifacts needed for specialized tests, such as database compatibility tests.

These files are **not** committed to the repository and are listed in `.gitignore`.

## Setup for Database Compatibility Testing

The compatibility test (`core/infrastructure/persistence/tests/compatibility_test.rs`) ensures that new code can still read data from an older database schema.

To run it, use `just test-db-compat`.
