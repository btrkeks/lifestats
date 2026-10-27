// EXCERPT — core/infrastructure/persistence/src/db/mappers.rs
//
// Two silent traps today (gotchas #1, #2): the FromSql decoder is a hand-written
// string match, and from_any_statistic ends in `_ =>`. Both become compile-checked.
// Needs `strum = "0.27"`, `strum_macros = "0.27"` in persistence/Cargo.toml.

#[derive(Debug, Serialize, Deserialize, AsExpression, FromSqlRow, Clone, Copy, strum_macros::EnumIter)]
#[diesel(sql_type = Text)]
pub enum StatisticTypeDto {
    IntegerAmount,
    PointInTime,
    TimeSpans,
    TimedAmounts, // + NEW
    Habit,
}

impl StatisticTypeDto {
    pub fn as_str(&self) -> &'static str {
        match self {
            // ...
            StatisticTypeDto::TimedAmounts => "TimedAmounts", // CE
        }
    }
}

impl FromSql<Text, Sqlite> for StatisticTypeDto {
    fn from_sql(bytes: <Sqlite as Backend>::RawValue<'_>) -> deserialize::Result<Self> {
        use strum::IntoEnumIterator;
        let str_value = <String as FromSql<Text, Sqlite>>::from_sql(bytes)?;
        // Derived from the variant list: a new variant can't be written but unreadable,
        // which would fail every find_all_metadata (GET /statistics, rail, dashboard).
        Self::iter()
            .find(|t| t.as_str() == str_value)
            .ok_or_else(|| anyhow!("Invalid statistic type '{str_value}'").into())
    }
}

impl StatisticDto {
    pub fn from_any_statistic(statistic: &AnyStatistic) -> Result<Self> {
        let (statistic_type, metadata) = match statistic {
            // ...
            AnyStatistic::TimedAmounts(inner) => (StatisticTypeDto::TimedAmounts, inner.get_metadata()),
            // Was `_ =>`: now explicit, so the next AnyStatistic variant is a compile error here.
            AnyStatistic::Summing(_) | AnyStatistic::TimeDifference(_) => {
                return Err(anyhow!("SQLite persistence does not support this statistic variant yet"));
            }
        };
        // ...
    }

    pub fn into_any_statistic(self, default_goal: Option<DefaultGoalRaw>) -> Result<AnyStatistic> {
        // ...
        match self.statistic_type {
            // ...
            StatisticTypeDto::TimedAmounts => {
                let (goal, _) = parse_default_goal::<TimedAmountsGoal>(default_goal, start_date)?;
                Ok(AnyStatistic::TimedAmounts(TimedAmountsStatistic::new(metadata, goal, start_date)))
            }
        }
    }
}

// InsertStatisticDto::from_any_statistic delegates to StatisticDto (:186): no change.
// No migration: statistic_type is unconstrained TEXT; values and goals are JSON TEXT.
