// CHANGED error mapping in BOTH get_statistic_plot_image and get_statistic_plot_data:
// ApplicationError::InvalidInput(message) => ApiError::BadRequest(message).
// Preserve existing not-found and infrastructure handling for other errors.
// This is necessary: current code otherwise maps unsupported TimedAmounts plot
// requests from GetPlotData to an InternalError (500).
