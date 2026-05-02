use std::collections::HashMap;

use lifestats_core::progress::ProgressItem;

use crate::cqrs::Query;

#[derive(Debug, Clone, Default)]
pub struct ListProgressItemsQuery {
    pub tags: Option<HashMap<String, String>>,
}

impl Query for ListProgressItemsQuery {
    type Result = Vec<ProgressItem>;
}
