# Git Commit Message Conventions

This document defines the commit message format for the LifeStats project.

## Format

We follow the **Conventional Commits** specification with crate-based scopes:

```
<type>(<scope>): <description>

[optional body]

[optional footer(s)]
```

### Type

The type must be one of the following:

- **feat**: A new feature
- **fix**: A bug fix
- **refactor**: Code change that neither fixes a bug nor adds a feature
- **perf**: Performance improvement
- **test**: Adding or updating tests
- **docs**: Documentation changes
- **chore**: Maintenance tasks (dependencies, tooling, etc.)
- **style**: Code style changes (formatting, whitespace, etc.)
- **ci**: Changes to CI/CD configuration

### Scope

The scope indicates which crate or component is affected:

**Core crates:**
- `core` - Domain model (core/core)
- `application` - Use case handlers (core/application)
- `plot-domain` - Plot domain types

**Infrastructure crates:**
- `persistence` - Database/repository (core/infrastructure/persistence)
- `plotting` - Plot rendering (core/infrastructure/plotting)
- `web-api` - HTTP server (core/infrastructure/web_api)
- `fitbit-client` - Fitbit integration (core/infrastructure/fitbit_client)
- `system-clock` - Clock implementation

**Other components:**
- `bootstrap` - Dependency injection (lifestats-bootstrap)
- `stopwatch` - Stopwatch logger
- `midi` - MIDI logger
- `web-ui` - Browser-based UI
- `wallpaper` - Desktop wallpaper updater
- `events` - Event system

**Special scopes:**
- `workspace` - Changes affecting multiple crates
- `deps` - Dependency updates

Scope may be omitted if the change affects the entire project (e.g., `docs: update README`).

### Description

- Use imperative mood ("add feature" not "added feature")
- Don't capitalize first letter
- No period at the end
- Keep under 72 characters

### Body (optional)

- Provide additional context about what and why (not how)
- Wrap at 72 characters
- Separate from subject with blank line

### Footer (optional)

- Reference issues: `Closes #123`, `Fixes #456`
- Note breaking changes: `BREAKING CHANGE: description`

## Examples

### Simple commits

```
feat(web-api): add endpoint for bulk logging
fix(persistence): resolve time zone handling in TimeSpans queries
refactor(application): extract validation from LogValueCommandHandler
test(core): add property tests for goal calculations
docs: update ARCHITECTURE.md with CQRS diagrams
chore(deps): upgrade Diesel to 2.1.4
```

### Commit with body

```
feat(fitbit-client): add rate limiting to API client

Fitbit API has strict rate limits. This adds exponential backoff
and respects the Retry-After header to prevent 429 errors during
bulk sync operations.

Closes #42
```

### Breaking change

```
refactor(core): change Logable trait to use Result for validation

BREAKING CHANGE: calculate_new_log_value now returns Result instead
of panicking on invalid input. All implementations must be updated.

Migration guide:
- Wrap return values in Ok()
- Return Err() for validation failures
```

### Multiple scopes

For changes spanning multiple related crates, choose the most relevant scope or use `workspace`:

```
refactor(workspace): rename StatisticRepository methods for consistency
```

## Architecture-Aware Commits

Since LifeStats follows Hexagonal Architecture, consider these patterns:

### Domain changes (core)
```
feat(core): add Computable trait for derived statistics
refactor(core): extract TimeFrame validation logic
```

### Application layer (use cases)
```
feat(application): implement DeleteStatisticCommand handler
refactor(application): simplify LogValueCommand error handling
```

### Infrastructure adapters
```
feat(persistence): add query method for statistics by tag
feat(web-api): add CORS middleware
fix(fitbit-client): handle token refresh edge case
```

### Cross-layer changes
```
feat(workspace): add support for recurring goals
```

## Tips

- **Commit early and often** - Small, focused commits are easier to review and revert
- **One logical change per commit** - Don't mix refactoring with feature work
- **Reference the architecture** - Mention layer boundaries when relevant
- **Test changes separately** - Consider separate commits for tests when adding features

## Tools

These conventions enable:
- Automated changelog generation
- Semantic versioning automation
- Easy filtering: `git log --grep="^feat"`
- Clear architectural change tracking

## References

- [Conventional Commits specification](https://www.conventionalcommits.org/)
- [LifeStats Architecture](./ARCHITECTURE.md)
