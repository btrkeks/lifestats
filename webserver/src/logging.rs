// // src/logging.rs
// use std::io::Write;
// use env_logger::{Builder};
// use log::LevelFilter;
// use chrono::Local;
// use anyhow::Result;
// // use backend::util::paths;
//
// pub fn setup_logging() -> Result<()> {
//     Builder::new()
//         .format(|buf, record| {
//             if record.target().starts_with("rocket") {
//                 writeln!(buf, "{}", record.args())
//             } else {
//                 writeln!(
//                     buf,
//                     "{} [{:5}] {} - {}",
//                     Local::now().format("%Y-%m-%d %H:%M:%S"),
//                     record.level(),
//                     record.target(),
//                     record.args()
//                 )
//             }
//         })
//         .filter(Some("rocket"), LevelFilter::Info)
//         .filter(None, LevelFilter::Info)
//         .write_style(env_logger::WriteStyle::Always)
//         .init();
//
//     Ok(())
// }