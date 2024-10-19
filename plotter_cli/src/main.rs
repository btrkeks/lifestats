use plotter_backend::regenerate_all_plots;

fn main() {
    regenerate_all_plots().expect("Error generating plots");
}
