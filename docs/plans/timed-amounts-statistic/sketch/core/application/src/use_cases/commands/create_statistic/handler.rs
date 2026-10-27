// EXCERPT — core/application/src/use_cases/commands/create_statistic/handler.rs
// The match on StatisticType is exhaustive → CE once the variant exists.
let statistic = match statistic_type {
    // ...
    StatisticTypeDto::TimedAmounts => {
        // One way to create: the source's PUT provisions with the right tags.
        // A hand-made TimedAmounts statistic could never receive data.
        return Err(ApplicationError::InvalidInput(
            "TimedAmounts statistics are created by their source integration".to_string(),
        ));
    }
    // ...
};

// --- core/infrastructure/web_api/src/routes/statistics.rs create_statistic (:430) ---
// The route currently maps EVERY handler error to ApiError::InternalError. Change it to
// `.map_err(ApiError::from)` (error.rs:149 already maps InvalidInput -> 400), or the
// rejection above surfaces as 500. Test: POST /statistics {"statistic_type":"TimedAmounts"} -> 400.
