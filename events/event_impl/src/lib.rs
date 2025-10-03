pub mod event_port;
/// Description:
/// This library allows other parts to subscribing to some event
/// (e.g.: Statistic definition changed, New data logged, Plot definition changed, ...)
/// and provides an interface to send those events.
/// It follows a fire-and-forget pattern, i.e. the publish event function sends an event
/// and exists without any response whatsoever.
/// There do not need to be any listeners for an event.
pub mod unix_socket;
