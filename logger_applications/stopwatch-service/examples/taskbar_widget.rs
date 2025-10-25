use anyhow::Result;
use lifestats_stopwatch_service::StopwatchClient;

/// Simple taskbar widget that prints the current stopwatch status once
/// Usage: Add this to your i3blocks/polybar/waybar config
#[tokio::main]
async fn main() -> Result<()> {
    let client = StopwatchClient::new().await?;

    match client.get_status().await {
        Ok(Some(status)) => {
            if status.is_running {
                // Print with stopwatch emoji
                println!("⏱ {}", status.status_string());
            } else {
                // Print nothing when stopped (hides from taskbar)
                println!();
            }
        }
        Ok(None) | Err(_) => {
            // Service not available - print nothing
            println!();
        }
    }

    Ok(())
}
