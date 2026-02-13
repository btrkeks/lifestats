# Code Quality Guidelines

## 1. Architecture & Dependency Flow

- **Strict Onion Architecture**: Dependencies point **inward**. Core knows nothing of Infrastructure.
- **Explicit Boundaries**: Use DTOs/Interfaces to decouple layers.

## 2. Context Locality (High Cohesion)

- **Small, Self-Contained Files**: Target < 200 lines. Focus on a single concept.
- **Semantic Naming**: Avoid generic names like `index.ts` or `utils.ts`. File names should describe the business intent (e.g., `calculateOrderTotal.ts`).
- **Explicit Imports**: No implicit global state.

## 3. Separation of Data & Behavior

- **Anemic Data**: Simple structs for data. Public properties, **no logic**.
- **Pure Logic**: Stateless functions returning new data.
- **Strict Determinism**: Functions must not instantiate `new Date()`, `Math.random()`, or UUIDs. Time and ID generation must be passed in as arguments.

## 4. Orchestration vs. Operation

- **The Orchestrator**: Manages flow, handles errors (try/catch), and injects dependencies (Time, DB).
- **The Operator**: Performs calculations. **Bubbles errors up** (no try/catch).

## 5. Type Safety & Contracts

- **Strict Typing**: No `any`. Trust the type system internally.
- **Boundary Validation**: Validate external data (Zod/Serde) at the entry point.
- **Return Types**: Always explicitly declare function return types.

## 6. Single Level of Abstraction (SLA)

- **Consistent Abstraction**: Keep all statements in a function at the same level of abstraction.
- **No Mixing**: Don't mix high-level domain logic (e.g., "sync issues") with low-level details (e.g., "parse JSON").
- **Extract & Name**: Move low-level operations into well-named helper functions to maintain clarity and readability.

## 7. No Backward Compatibility Code

- **No Legacy Paths**: Do not keep old code paths, shims, feature flags, or adapters “just in case”.
- **Break Cleanly**: If a change is breaking, make it explicitly breaking (version bump / migration), then delete the old behavior.
- **Migrate, Don’t Support**: Prefer one-time migrations (data/code) over runtime compatibility layers.
- **Delete Dead Code Fast**: If something is deprecated, remove it promptly once the new path is in place.
