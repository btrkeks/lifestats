use anyhow::Result;
use lifestats_stopwatch_service::StopwatchClient;

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();

    println!("Stopwatch Status Monitor (real-time updates via DBus signals)");
    println!("Press Ctrl+C to exit\n");

    let client = StopwatchClient::new().await?;

    // First, get the current state
    if let Some(status) = client.get_status().await? {
        println!(
            "Current state: {} - {} ({})",
            status.statistic_name,
            status.format_duration(),
            if status.is_running {
                "running"
            } else {
                "stopped"
            }
        );
    } else {
        println!("No stopwatch currently running");
    }

    println!("\nWaiting for updates...\n");

    // Then listen for real-time updates
    client
        .listen_for_updates(|status| {
            println!(
                "[{}] {} - {} ({})",
                chrono::Local::now().format("%H:%M:%S"),
                status.statistic_name,
                status.format_duration(),
                if status.is_running {
                    "running"
                } else {
                    "stopped"
                }
            );
        })
        .await?;

    Ok(())
}
