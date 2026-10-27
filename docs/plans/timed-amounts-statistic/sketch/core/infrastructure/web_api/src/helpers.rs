// DELETE core/infrastructure/web_api/src/helpers.rs (and `pub mod helpers;` in src/lib.rs).
// json_to_drawable_data / json_to_drawable_goal / percentage_to_color have no
// callers (verified by grep over all *.rs); they duplicate
// core/application/src/services/plot_data_builder.rs. Deleting removes one
// exhaustive site that would otherwise need a TimedAmounts arm nobody runs.
