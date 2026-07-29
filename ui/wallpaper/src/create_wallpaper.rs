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
            15 => PlotPosition::new(1, MARGIN + 2 * COLUMN_WIDTH, MARGIN + 3 * ROW_HEIGHT), // アプリ開発
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
