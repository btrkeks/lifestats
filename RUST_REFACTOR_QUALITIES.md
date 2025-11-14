# Clean Rust Code Style Guide (for LLM Refactors)

This document defines what “clean” Rust code means, optimized for both humans and LLM-based refactors.

---

## 1. Goals

- Prioritize **low coupling, high cohesion**: modules should be small, focused, and easy to understand in isolation.
- Prefer **simplicity over cleverness**: boring, explicit code is better than clever abstractions.
- Enable **local reasoning**: you should understand a function or module without chasing many other files.
- Follow **SOLID principles** where they improve clarity and maintainability.
- Make **behavior-preserving refactors** the default: do not change semantics unless explicitly requested.

---

## 2. Architecture & Modules

- Separate **core/domain** from **infrastructure** (ports & adapters / hexagonal architecture mindset).
- Core/domain modules must **not depend on** HTTP, databases, logging frameworks, or other infrastructure crates.
- Each module/file should have **one primary responsibility** (e.g., `*_domain.rs`, `*_service.rs`, `*_repo.rs`).
- Prefer files that are **short and focused**; if a file grows too large or mixed in concerns, split it.
- Treat module public APIs as **contracts**: avoid changing public types and function signatures casually.
- Explicitly mark important, stable modules or APIs with comments like `// PUBLIC API – avoid changing signatures`.

---

## 3. Types & Invariants (Make Impossible States Impossible)

- Avoid “stringly typed” code: use **newtypes** for IDs and domain concepts instead of raw `String`/`u64`, e.g. `UserId(String)`.
- Use **enums instead of booleans or magic strings** for states and modes, e.g. `Status::Open | Closed | Archived`.
- Encode invariants into types wherever possible so **invalid states cannot be constructed**.
- Prefer modeling state transitions with **enums or state types** instead of multiple loosely related `Option` fields.
- Document type invariants clearly in `///` comments so LLMs and humans know what must always be true.
- Prefer **owned types** and clear lifetimes where possible; avoid over-complicated lifetime gymnastics.

---

## 4. Error Handling

- Use `Result<T, E>` consistently; avoid unchecked `unwrap()`/`expect()` in non-test code.
- If `unwrap()` or `expect()` is truly safe, add a **clear justification** comment (e.g. `// SAFETY: ...` or `// Invariant: ...`).
- Prefer a **small, well-defined error enum per domain** over generic `anyhow::Error` in core logic.
- Use `anyhow::Error` mainly at **boundaries** (e.g., CLI, HTTP adapters) where errors are just logged or rendered.
- Do not format user-facing error strings deep in infrastructure; let higher layers handle **user-visible messages**.
- Never silently ignore errors; if errors are intentionally discarded, document why (`// Intentionally ignoring: ...`).

---

## 5. Traits, Generics & Abstractions

- Favor **small, focused traits** that express one capability (e.g. `Clock`, `RandomProvider`, `EmailSender`).
- Define traits as **ports** in the domain/application layer; implementations live in infrastructure modules.
- Avoid introducing generic abstractions **“just in case”**; only generalize when there is a real second use.
- Keep trait bounds **readable**; use `where` clauses for complex constraints instead of inline noise.
- Prefer **composition over deep trait hierarchies**; avoid “god traits” that do too much.
- Keep generics moderate; do not introduce complex type-level wizardry unless it clearly reduces overall complexity.

---

## 6. Async, Concurrency & Side Effects

- Separate **pure logic** from **side-effecting code**: pure functions for computation, async/IO functions for external interactions.
- Domain logic should be mostly **sync and pure**; async should appear mainly at infrastructure boundaries (e.g. DB, HTTP).
- Avoid hidden global mutable state; do not introduce singletons unless absolutely necessary.
- If using shared state (`Arc<Mutex<_>>`, `RwLock`, etc.), add a comment describing the **concurrency model** and assumptions.
- Be explicit about **ordering and atomicity requirements**; document when actions must happen together or in a transaction.
- Do not introduce complex concurrency patterns (channels, background tasks, etc.) without a clear reason and documentation.

---

## 7. Tests & Examples

- Treat tests as part of the **specification**: refactors must preserve test behavior.
- Favor **small, targeted unit tests** for core logic; they make LLM refactors safer and clearer.
- Where appropriate, add **property-based tests** (e.g. with `proptest`) to enforce important invariants.
- Use **doc tests** as examples for key public functions and types; they show intended usage patterns.
- Do not delete or weaken tests during refactors unless explicitly requested; if behavior must change, update tests thoughtfully.

---

## 8. Documentation & Comments

- Start each module with a short `//!` comment explaining its **responsibility** and **what it must not depend on**.
- Use concise `///` comments on public types and functions to describe:
  - What they do (1–2 sentences)
  - Key invariants / expectations
  - Important error conditions
- For functions with strong constraints, add sections like:
  - `/// # Invariants`
  - `/// # Errors`
- Use `// NOTE:` for important non-obvious decisions and `// SAFETY:` when justifying unsafe code or strong assumptions.
- Mark critical functions that shouldn’t be changed lightly with `// IMPORTANT: behavior relied upon across the system`.
- Optionally mark messy or legacy code with `// REFACTOR:` to indicate that restructuring is welcome if behavior is preserved.

---

## 9. Rules for LLM Refactors

These rules are specifically for LLMs touching this code base.

- **Scope:** Unless told otherwise, refactor **only within the given file** and keep public APIs (types, function signatures) unchanged.
- **Behavior:** Do not intentionally change observable behavior; if you are unsure what something does, leave it as-is and add a `// TODO (human):` comment.
- **Types & Invariants:** Preserve and respect all documented invariants, type guarantees, and error semantics.
- **Errors:** Do not replace structured error enums with generic errors; keep or improve error structure.
- **Abstractions:** Do not add new traits, generics, or shared state without a clear and documented benefit in simplicity or safety.
- **Dependencies:** Do not introduce new third-party crates or cross-layer dependencies unless explicitly instructed.
- **Style:** Keep existing naming conventions, module structure, and layering rules (core vs. infrastructure) intact.
- **Comments:** If you simplify or restructure complex logic, update comments and docstrings so they remain accurate.

---
