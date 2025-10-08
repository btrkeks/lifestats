/// Core CQRS trait definitions for the application layer
///
/// This module defines the foundational abstractions for Command Query Responsibility Segregation.
/// Commands represent write operations that change system state.
/// Queries represent read operations that return data without side effects.
use crate::error::ApplicationResult;

/// Marker trait for query objects
///
/// Queries are data retrieval requests that do not modify system state.
/// They return a typed Result.
pub trait Query {
    type Result;
}

/// Marker trait for command objects
///
/// Commands are requests to modify system state.
/// They may return data (e.g., created entity ID) or just success/failure.
pub trait Command {
    type Result;
}

/// Handler for query execution
///
/// Implements the logic to execute a specific query type.
/// Handlers are responsible for:
/// - Accessing repositories and ports
/// - Executing domain logic
/// - Returning DTOs suitable for the application boundary
pub trait QueryHandler<Q: Query> {
    fn handle(&self, query: Q) -> ApplicationResult<Q::Result>;
}

/// Handler for command execution
///
/// Implements the logic to execute a specific command type.
/// Handlers are responsible for:
/// - Validating command data
/// - Loading domain entities
/// - Executing domain methods
/// - Persisting changes
/// - Publishing events
/// - Managing transaction boundaries
pub trait CommandHandler<C: Command> {
    fn handle(&self, command: C) -> ApplicationResult<C::Result>;
}
