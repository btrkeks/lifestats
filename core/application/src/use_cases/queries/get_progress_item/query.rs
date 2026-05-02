use lifestats_core::progress::ProgressItem;
use lifestats_core::shared::ids::ProgressItemId;

use crate::cqrs::Query;

pub struct GetProgressItemQuery {
    pub item_id: ProgressItemId,
}

impl Query for GetProgressItemQuery {
    type Result = ProgressItem;
}
