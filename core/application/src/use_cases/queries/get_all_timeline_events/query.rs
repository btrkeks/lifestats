use crate::{cqrs::Query, dto::summary::TimelineEvent};
use chrono::NaiveDate;

/// Query to retrieve all timeline events for a specific date.
pub struct GetAllTimelineEventsQuery {
    pub date: NaiveDate,
}

impl Query for GetAllTimelineEventsQuery {
    type Result = Vec<TimelineEvent>;
}
