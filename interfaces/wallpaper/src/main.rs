pub mod core;
use core::overlay_image::overlay_images;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    overlay_images(
        "background.png",    // 1920x1080 image
        "overlay.png",       // 400x300 image
        100,                 // X position
        100,                 // Y position
        "../../../output.png"
    )?;

    println!("Image overlay completed successfully!");
    Ok(())
}