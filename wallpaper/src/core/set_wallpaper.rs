use std::process::Command;
use anyhow::Result;
use backend::util::paths::get_plot_path;

pub fn update_wallpaper() -> Result<()> {
    let plot_path = get_plot_path()?;
    let wallpaper1 = plot_path.join("wallpaper1.png");
    let wallpaper2 = plot_path.join("wallpaper2.png");

    Command::new("xwallpaper")
        .arg("--output")
        .arg("HDMI-2")
        .arg("--center")
        .arg(&wallpaper1)
        .arg("--output")
        .arg("HDMI-1")
        .arg("--center")
        .arg(&wallpaper2)
        .output()?;

    Ok(())
}