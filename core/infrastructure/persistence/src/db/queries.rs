use std::collections::HashMap;

use anyhow::{Context, Result};
use diesel::prelude::*;
use diesel::SqliteConnection;
use lifestats_domain::shared::ids::StatisticId;

use crate::db::models::DefaultGoalRaw;
use crate::db::schema::default_goals;

pub(crate) fn fetch_default_goal(
    conn: &mut SqliteConnection,
    statistic_id: StatisticId,
) -> Result<Option<DefaultGoalRaw>> {
    default_goals::table
        .find(statistic_id.0 as i32)
        .first::<DefaultGoalRaw>(conn)
        .optional()
        .context("failed to fetch default goal")
}

pub(crate) fn fetch_default_goals(
    conn: &mut SqliteConnection,
    statistic_ids: &[i32],
) -> Result<HashMap<i32, DefaultGoalRaw>> {
    if statistic_ids.is_empty() {
        return Ok(HashMap::new());
    }

    let rows = default_goals::table
        .filter(default_goals::statistic_id.eq_any(statistic_ids))
        .load::<DefaultGoalRaw>(conn)
        .context("failed to fetch default goals batch")?;

    Ok(rows
        .into_iter()
        .map(|raw| (raw.statistic_id, raw))
        .collect())
}
