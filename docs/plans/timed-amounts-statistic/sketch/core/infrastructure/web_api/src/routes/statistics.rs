// EXCERPT — core/infrastructure/web_api/src/routes/statistics.rs  log_value (:155, CE)
// One way to write: no LogValuePayload variant exists for TimedAmounts, so the
// application handler's `_ =>` mismatch arm is never reached for this type.
AnyStatistic::TimedAmounts(_) => {
    return Err(ApiError::BadRequest(
        "TimedAmounts statistics are written by their source integration \
         (PUT /integrations/opennutritracker/days/{day})"
            .to_string(),
    ));
}
// log_time_frame (:348): its TimeSpans-only check already rejects. OK-AS-IS.
