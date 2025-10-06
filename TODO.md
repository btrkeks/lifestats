# `TODO.md`: Refinements for Integration Bundles Feature

The initial implementation is structurally sound but requires key refinements to ensure security, robustness, and maintainability. This plan prioritizes the most critical fixes first.

## Part 4: Transactional Bundle Persistence

These changes ensure provisioning a bundle is an atomic unit of work and that callers receive the database-assigned statistic IDs.

- **4.1. Extend the Statistic Repository Port:**
  - **File:** `core/core/src/statistic/ports/statistic_repository.rs`
  - **Action:** Replace the existing `save_metadata` method with explicit operations:
    ```rust
    fn create_metadata(&self, statistic: AnyStatistic) -> Result<AnyStatistic, RepositoryError>;
    fn update_metadata(&self, statistic: AnyStatistic) -> Result<(), RepositoryError>;
    fn create_many_metadata(&self, statistics: Vec<AnyStatistic>) -> Result<Vec<AnyStatistic>, RepositoryError>;
    ```
    (Consume `AnyStatistic` so ownership matches the persistence layer’s need to rebuild it with new IDs.)
  - **Rationale:** Clarifies intent (create vs. update), requires implementers to return persisted entities with real IDs, and provides an atomic bulk insert entry point.

- **4.2. Update Call Sites in the Application Layer:**
  - **Files:**
    - `core/application/src/use_cases/integration/service.rs`
    - Any other module still calling `save_metadata`
  - **Action:**
    - For integration provisioning, collect the `AnyStatistic`s from the factory, call `create_many_metadata`, and build the response map from the returned stats (using the `stat_key` tag).
    - For other callers (goal management, logging, etc.), swap to `create_metadata` when inserting new statistics and `update_metadata` when modifying existing ones. If a path still needs “upsert” semantics, keep a small helper in the application layer that chooses between the two based on whether the ID is zero.
  - **Rationale:** Moves the responsibility for ID assignment and atomicity out of the application loop and keeps each use case explicit about whether it is creating or updating statistics.

- **4.3. Implement Atomic Persistence in `SqliteStatisticRepository`:**
  - **File:** `core/infrastructure/persistence/src/repository.rs`
  - **Action:**
    - Introduce an `InsertStatisticDto` (no `id` field) for fresh inserts so Diesel lets SQLite allocate the primary key.
    - Implement `create_metadata` and `create_many_metadata` using Diesel transactions. Each insert should capture the generated ID (via `last_insert_rowid()` followed by a fetch, or `RETURNING` if available) and rebuild an `AnyStatistic` with that ID before returning.
    - `update_metadata` should issue a straightforward `UPDATE` on the existing row.
    - Update any existing helper functions/tests to use the new API.
  - **Rationale:** Guarantees the bundle is persisted atomically, eliminates the “ID 0” bug, and keeps persistence logic encapsulated in the adapter.

- **4.4. Document and Test the JSON Tag Filter:**
  - **File:** `core/infrastructure/persistence/src/repository.rs`
  - **Action:** Add a doc comment explaining the `json_each` query used in `find_by_tags`, and add focused integration tests covering matching/mismatching tag scenarios now that the repository API is being touched.
  - **Rationale:** Improves maintainability of the custom SQL and ensures the new transactional work doesn’t regress tag filtering.
