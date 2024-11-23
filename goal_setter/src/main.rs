// use anyhow::Result;
// use backend;
// use backend::db_ops::open_db;
// use backend::statistic_getter::get_all_statistics;
// use backend::statistics::{AnyStatistic, Statistic, StatisticTypeSpecific};
// use chrono::{Local, NaiveDate};
// use eframe::egui;
// use eframe::egui::{UiBuilder, ViewportBuilder, ViewportCommand};
// use serde::{Deserialize, Serialize};
// use std::fs;

// struct LifestatsApp {
//     statistics: Vec<AnyStatistic>,
// }

// impl LifestatsApp {
//     fn new(cc: &eframe::CreationContext<'_>) -> Self {
//         Self::configure_fonts(&cc.egui_ctx);
//         let statistics = get_all_statistics().unwrap();
//         Self { statistics }
//     }

//     fn configure_fonts(ctx: &eframe::egui::Context) -> Option<()> {
//         let font_file = Self::find_cjk_font()?;
//         let font_name = font_file.split('/').last()?.split('.').next()?.to_string();
//         let font_file_bytes = std::fs::read(font_file).ok()?;

//         let font_data = eframe::egui::FontData::from_owned(font_file_bytes);
//         let mut font_def = eframe::egui::FontDefinitions::default();
//         font_def.font_data.insert(font_name.to_string(), font_data);

//         let font_family = eframe::epaint::FontFamily::Proportional;
//         font_def
//             .families
//             .get_mut(&font_family)?
//             .insert(0, font_name);

//         ctx.set_fonts(font_def);
//         Some(())
//     }

//     fn find_cjk_font() -> Option<String> {
//         #[cfg(unix)]
//         {
//             use std::process::Command;
//             // linux/macOS command: fc-list
//             let output = Command::new("sh").arg("-c").arg("fc-list").output().ok()?;
//             let stdout = std::str::from_utf8(&output.stdout).ok()?;
//             #[cfg(target_os = "macos")]
//             let font_line = stdout
//                 .lines()
//                 .find(|line| line.contains("Regular") && line.contains("Hiragino Sans GB"))
//                 .unwrap_or("/System/Library/Fonts/Hiragino Sans GB.ttc");
//             #[cfg(target_os = "linux")]
//             let font_line = stdout
//                 .lines()
//                 .find(|line| line.contains("Regular") && line.contains("CJK"))
//                 .unwrap_or("/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc");

//             let font_path = font_line.split(':').next()?.trim();
//             Some(font_path.to_string())
//         }
//         #[cfg(windows)]
//         {
//             let font_file = {
//                 // c:/Windows/Fonts/msyh.ttc
//                 let mut font_path = PathBuf::from(std::env::var("SystemRoot").ok()?);
//                 font_path.push("Fonts");
//                 font_path.push("msyh.ttc");
//                 font_path.to_str()?.to_string().replace("\\", "/")
//             };
//             Some(font_file)
//         }
//     }
// }

// impl eframe::App for LifestatsApp {
//     fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
//         egui::CentralPanel::default().show(ctx, |ui| {
//             let app_rect = ui.max_rect();

//             let title_bar_height = 32.0;
//             let title_bar_rect = {
//                 let mut rect = app_rect;
//                 rect.max.y = rect.min.y + title_bar_height;
//                 rect
//             };
//             title_bar_ui(ui, title_bar_rect, "Set goals for today");

//             let content_rect = {
//                 let mut rect = app_rect;
//                 rect.min.y = title_bar_rect.max.y;
//                 rect
//             }
//             .shrink(4.0);
//             let mut content_ui = ui.new_child(UiBuilder::new().max_rect(content_rect));

//             for (index, statistic) in self.statistics.iter().enumerate() {
//                 match &statistic {
//                     AnyStatistic::IntegerAmount(s) => {
//                         content_ui.horizontal(|ui| {
//                             ui.label(&format!("{}: ", s.get_name()));
//                             let mut value = s.get_goal().unwrap_or(0);
//                             if ui.add(egui::DragValue::new(&mut value)).changed() {
//                                 // let db = open_db()?;
//                                 // s.set_goal(&db, Local::now().date_naive(), value);
//                             }
//                         });
//                     }
//                     AnyStatistic::TimeAmount(_) => {}
//                     AnyStatistic::PointInTime(_) => {}
//                     AnyStatistic::TimeSpans(_) => {}
//                     AnyStatistic::TimeSpan(_) => {}
//                     AnyStatistic::Habit(_) => {}
//                 }
//             }

//             if content_ui.button("Save Goals").clicked() {
//                 println!("Saving goals");
//             }
//         });
//     }
// }

// fn title_bar_ui(ui: &mut egui::Ui, title_bar_rect: eframe::epaint::Rect, title: &str) {
//     use egui::{vec2, Align2, FontId, Id, PointerButton, Sense, UiBuilder};

//     let painter = ui.painter();

//     let title_bar_response = ui.interact(
//         title_bar_rect,
//         Id::new("title_bar"),
//         Sense::click_and_drag(),
//     );

//     // Paint the title:
//     painter.text(
//         title_bar_rect.center(),
//         Align2::CENTER_CENTER,
//         title,
//         FontId::proportional(20.0),
//         ui.style().visuals.text_color(),
//     );

//     // Paint the line under the title:
//     painter.line_segment(
//         [
//             title_bar_rect.left_bottom() + vec2(1.0, 0.0),
//             title_bar_rect.right_bottom() + vec2(-1.0, 0.0),
//         ],
//         ui.visuals().widgets.noninteractive.bg_stroke,
//     );

//     // Interact with the title bar (drag to move window):
//     if title_bar_response.double_clicked() {
//         let is_maximized = ui.input(|i| i.viewport().maximized.unwrap_or(false));
//         ui.ctx()
//             .send_viewport_cmd(ViewportCommand::Maximized(!is_maximized));
//     }

//     if title_bar_response.drag_started_by(PointerButton::Primary) {
//         ui.ctx().send_viewport_cmd(ViewportCommand::StartDrag);
//     }

//     ui.allocate_new_ui(
//         UiBuilder::new()
//             .max_rect(title_bar_rect)
//             .layout(egui::Layout::right_to_left(egui::Align::Center)),
//         |ui| {
//             ui.spacing_mut().item_spacing.x = 0.0;
//             ui.visuals_mut().button_frame = false;
//             ui.add_space(8.0);
//         },
//     );
// }

// fn main() -> Result<(), eframe::Error> {
//     env_logger::init();
//     let options = eframe::NativeOptions {
//         viewport: egui::ViewportBuilder::default()
//             .with_decorations(false) // Hide the OS-specific "chrome" around the window
//             .with_inner_size([400.0, 100.0])
//             .with_min_inner_size([400.0, 100.0])
//             .with_transparent(true), // To have rounded corners we need transparency

//         ..Default::default()
//     };

//     eframe::run_native(
//         "Lifestats Goal Setting",
//         options,
//         Box::new(|cc| Ok(Box::new(LifestatsApp::new(cc)))),
//     )
// }

fn main() {}
