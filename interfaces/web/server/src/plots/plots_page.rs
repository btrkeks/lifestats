use rocket::fs::NamedFile;
use rocket::response::status::NotFound;
use rocket_dyn_templates::{Template, context};
use std::path::PathBuf;
use backend::util::paths;
use plotting_backend::plots::plot_getter::get_all_plots;
use plotting_backend::plots::plot_types::Plot;

#[get("/")]
pub async fn show_plots_page() -> Template {
    let plots = get_all_plots().unwrap_or_default();

    Template::render("index", context! {
        active_page: "plots",
        plots: plots.iter().map(|plot| context! {
            id: plot.get_id(),
            title: plot.get_title()
        }).collect::<Vec<_>>()
    })
}

#[get("/plots/<file..>")]
pub async fn serve_plot(file: PathBuf) -> Result<NamedFile, NotFound<String>> {
    let path = paths::get_plot_path()
        .map_err(|e| NotFound(format!("Error getting plots path: {}", e)))?
        .join(file);

    NamedFile::open(&path)
        .await
        .map_err(|e| NotFound(format!("Error opening file: {}", e)))
}