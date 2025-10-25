use anyhow::Result;
use lifestats_stopwatch_service::StopwatchClient;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();

    println!("Stopwatch Status Monitor (polling mode)");
    println!("Press Ctrl+C to exit\n");

    let client = StopwatchClient::new().await?;

    loop {
        match client.get_status().await {
            Ok(Some(status)) => {
                if status.is_running {
                    println!(
                        "\r\x1b[K⏱  {} - {}",
                        status.statistic_name,
                        status.format_duration()
                    );
                } else {
                    println!("\r\x1b[KNo stopwatch running");
                }
            }
            Ok(None) => {
                println!("\r\x1b[KStopwatch service not available");
            }
            Err(e) => {
                eprintln!("\r\x1b[KError: {}", e);
            }
        }

        tokio::time::sleep(Duration::from_secs(1)).await;
    }
}
