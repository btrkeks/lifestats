use crate::create_wallpaper::WallpaperError::InvalidWallpaperNumber;
use crate::mutable_image::MutableImage;
use crate::plot_position::PlotPosition;
use crate::plot_position_repository::PlotPositionRepository;
use anyhow::{anyhow, Context, Result};
use image::ImageReader;
use thiserror::Error;
use transfer_objects::plot_image::PlotImage;
use transfer_objects::statistic_id::PlotId;

#[derive(Error, Debug)]
pub enum WallpaperError {
    #[error("Failed to find position for plot ID {0}")]
    NoPlotPositionSet(PlotId),

    #[error("Wallpaper with number {0} does not exist")]
    InvalidWallpaperNumber(u32),

    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

pub fn update_wallpaper_with_images(
    wallpapers: &mut [MutableImage],
    plot_images: Box<[PlotImage]>,
    plot_position_repository: &impl PlotPositionRepository,
) -> Result<(), WallpaperError> {
    plot_images.into_iter().try_for_each(|plot_image| {
        update_wallpaper_with_image(wallpapers, &plot_image, plot_position_repository).or_else(
            |e| match e {
                // Ignore this error for now
                WallpaperError::NoPlotPositionSet(plot_id) => {
                    log::warn!("No plot position set for plot ID {}", plot_id);
                    Ok(())
                }
                other => Err(other),
            },
        )
    })
}

pub fn update_wallpaper_with_image(
    wallpapers: &mut [MutableImage],
    plot_image: &PlotImage,
    plot_position_repository: &impl PlotPositionRepository,
) -> Result<(), WallpaperError> {
    let plot_position = plot_position_repository
        .get_position_for_id(plot_image.plot_id)
        .with_context(|| format!("Failed to get position for plot id {}", plot_image.plot_id))?
        .ok_or(WallpaperError::NoPlotPositionSet(plot_image.plot_id))?;

    let wallpaper_nr = plot_position.get_wallpaper_nr();
    let (x, y) = plot_position.get_pixel_coordinates();

    wallpapers
        .get_mut((wallpaper_nr - 1) as usize)
        .ok_or_else(|| anyhow!("Wallpaper with number {} does not exist", wallpaper_nr))?
        .overlay(&plot_image.image, x, y);

    Ok(())
}

pub fn get_background_image() -> Result<MutableImage> {
    // TODO: This should be configurable

    // Get the home directory
    let home_dir = dirs::home_dir().ok_or(anyhow!("Could not obtain home directory"))?;

    // Construct the path to the image
    let image_path = home_dir.join(".local/share/src/dashboard/background.png");

    // Check if the file exists
    if !image_path.exists() {
        return Err(anyhow!("Image file not found: {}", image_path.display()));
    }

    let img = ImageReader::open(image_path)?.decode()?;

    MutableImage::from_dynamic_image(img)
}

pub struct PlotPositionRepositoryImpl;

impl PlotPositionRepositoryImpl {
    pub fn new() -> Self {
        Self {}
    }
}

impl PlotPositionRepository for PlotPositionRepositoryImpl {
    fn get_position_for_id(&self, plot_id: PlotId) -> Result<Option<PlotPosition>> {
        const COLUMN_WIDTH: usize = 485;
        const ROW_HEIGHT: usize = 210;
        const MARGIN: usize = 10;

        let id = plot_id.0;
        let pos = match id {
            1 => PlotPosition::new(2, MARGIN + 3 * COLUMN_WIDTH - 20, MARGIN + 0 * ROW_HEIGHT), // Timeline
            2 => PlotPosition::new(1, MARGIN + 0 * COLUMN_WIDTH, MARGIN + 3 * ROW_HEIGHT), // 睡眠時間
            4 => PlotPosition::new(2, MARGIN + 1 * COLUMN_WIDTH, MARGIN + 2 * ROW_HEIGHT), // 聞き取り練習の合計時間
            5 => PlotPosition::new(1, MARGIN + 2 * COLUMN_WIDTH, MARGIN + 4 * ROW_HEIGHT), // ネット利用合計時間
            6 => PlotPosition::new(1, MARGIN + 2 * COLUMN_WIDTH, MARGIN + 2 * ROW_HEIGHT), // ピアノ練習の合計時間
            7 => PlotPosition::new(1, MARGIN + 1 * COLUMN_WIDTH, MARGIN + 0 * ROW_HEIGHT), // Ankiレビュー時間
            8 => PlotPosition::new(1, MARGIN + 2 * COLUMN_WIDTH, MARGIN + 0 * ROW_HEIGHT), // ピアノ練習時間
            10 => PlotPosition::new(2, MARGIN + 2 * COLUMN_WIDTH, MARGIN + 0 * ROW_HEIGHT), // 読書時間
            11 => PlotPosition::new(2, MARGIN + 1 * COLUMN_WIDTH, MARGIN + 0 * ROW_HEIGHT), // 聞き取り練習の時間
            14 => PlotPosition::new(2, MARGIN + 2 * COLUMN_WIDTH, MARGIN + 2 * ROW_HEIGHT), // 読書の合計時間
            17 => PlotPosition::new(1, MARGIN + 1 * COLUMN_WIDTH, MARGIN + 2 * ROW_HEIGHT), // Ankiレビューを完了した瞬間
            18 => PlotPosition::new(1, MARGIN + 0 * COLUMN_WIDTH, MARGIN + 0 * ROW_HEIGHT), // 起床時間
            19 => PlotPosition::new(1, MARGIN + 0 * COLUMN_WIDTH, MARGIN + 1 * ROW_HEIGHT), // 就寝時間
            20 => PlotPosition::new(1, MARGIN + 0 * COLUMN_WIDTH, MARGIN + 2 * ROW_HEIGHT), // 入眠時間
            22 => PlotPosition::new(1, MARGIN + 3 * COLUMN_WIDTH, MARGIN + 0 * ROW_HEIGHT),
            23 => PlotPosition::new(1, MARGIN + 3 * COLUMN_WIDTH, MARGIN + 1 * ROW_HEIGHT),
            24 => PlotPosition::new(1, MARGIN + 3 * COLUMN_WIDTH, MARGIN + 2 * ROW_HEIGHT),
            25 => PlotPosition::new(1, MARGIN + 3 * COLUMN_WIDTH, MARGIN + 3 * ROW_HEIGHT),
            26 => PlotPosition::new(1, MARGIN + 3 * COLUMN_WIDTH, MARGIN + 4 * ROW_HEIGHT),
            _ => return Ok(None),
        };

        Ok(Some(pos))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plot_drawing::create_wallpapers;
    use anyhow::Result;
    use itertools::Itertools;
    use std::path::PathBuf;
    use transfer_objects::image::Image;
    use transfer_objects::plot_image::PlotImage;
    use transfer_objects::statistic_id::PlotId;

    /// Returns the path to the test resources directory
    fn test_resources_dir() -> PathBuf {
        let manifest_dir = env!("CARGO_MANIFEST_DIR");
        PathBuf::from(manifest_dir).join("tests").join("resources")
    }

    /// Load a test image from the resources directory
    fn load_test_plot_image(number: u32) -> Result<Image> {
        let path = test_resources_dir().join(format!("input/{}.png", number));
        let img = image::open(&path)?;

        Image::from_dynamic_image(img)
    }

    fn load_test_image(string: &str) -> Result<Image> {
        let path = test_resources_dir().join(format!("input/{}.png", string));
        let img = image::open(&path)?;

        Image::from_dynamic_image(img)
    }

    // fn save_test_image(image: &MutableImage, name: &str) {
    //     let path = test_resources_dir()
    //         .join("output")
    //         .join(format!("{}.png", name));
    //     image.save(&path).unwrap();
    // }

    // Helper function to create a test image with specified dimensions and color
    // fn create_test_image(width: usize, height: usize, color: Rgb<u8>) -> Image {
    //     let mut buffer = Vec::with_capacity(width * height * 3);
    //     for _ in 0..(width * height) {
    //         buffer.extend_from_slice(&[color.0[0], color.0[1], color.0[2]]);
    //     }
    //
    //     Image::new_from_buffer(width, height, buffer).unwrap()
    // }

    #[test]
    fn test_create_wallpaper() -> Result<()> {
        // Create a list of plot images using the test resources
        let plot_images: Vec<PlotImage> = vec![
            PlotImage {
                plot_id: PlotId(1),
                image: load_test_plot_image(1)?,
            },
            PlotImage {
                plot_id: PlotId(2),
                image: load_test_plot_image(2)?,
            },
            PlotImage {
                plot_id: PlotId(4),
                image: load_test_plot_image(4)?,
            },
            PlotImage {
                plot_id: PlotId(5),
                image: load_test_plot_image(5)?,
            },
            PlotImage {
                plot_id: PlotId(6),
                image: load_test_plot_image(6)?,
            },
            PlotImage {
                plot_id: PlotId(7),
                image: load_test_plot_image(7)?,
            },
            PlotImage {
                plot_id: PlotId(8),
                image: load_test_plot_image(8)?,
            },
            PlotImage {
                plot_id: PlotId(9),
                image: load_test_plot_image(9)?,
            },
            PlotImage {
                plot_id: PlotId(10),
                image: load_test_plot_image(10)?,
            },
            PlotImage {
                plot_id: PlotId(11),
                image: load_test_plot_image(11)?,
            },
            PlotImage {
                plot_id: PlotId(13),
                image: load_test_plot_image(13)?,
            },
            PlotImage {
                plot_id: PlotId(14),
                image: load_test_plot_image(14)?,
            },
            PlotImage {
                plot_id: PlotId(17),
                image: load_test_plot_image(17)?,
            },
            PlotImage {
                plot_id: PlotId(18),
                image: load_test_plot_image(18)?,
            },
            PlotImage {
                plot_id: PlotId(19),
                image: load_test_plot_image(19)?,
            },
            PlotImage {
                plot_id: PlotId(20),
                image: load_test_plot_image(20)?,
            },
        ];

        let repository = PlotPositionRepositoryImpl {};

        // Act
        let mut wallpapers = create_wallpapers(2)?;
        update_wallpaper_with_images(&mut wallpapers, Box::from(plot_images), &repository)?;

        // Assert
        let (result0, result1) = wallpapers
            .into_iter()
            .map(|image| image.clone().into_image().unwrap()) // TODO: Why clone??
            .collect_tuple()
            .unwrap();

        let wallpaper0 = load_test_image("wallpaper_0")?;
        let wallpaper1 = load_test_image("wallpaper_1")?;

        assert_eq!(result0, wallpaper0);
        assert_eq!(result1, wallpaper1);

        Ok(())
    }
}
