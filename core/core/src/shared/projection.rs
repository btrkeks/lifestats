#[derive(Debug, thiserror::Error)]
#[error("The resulting aggregate exceeds the supported integer range")]
pub struct ProjectionOverflow;

/// Preserve contributions outside this source, including signed corrections.
pub fn replace_source_projection(
    current: i64,
    old: i64,
    new: i64,
) -> Result<i64, ProjectionOverflow> {
    let result = i128::from(current) - i128::from(old) + i128::from(new);
    i64::try_from(result).map_err(|_| ProjectionOverflow)
}
