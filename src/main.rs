use color_eyre::Result;
use owl::{cec, os};
use tokio::signal;
use tracing::{info, level_filters::LevelFilter};
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

#[tokio::main]
async fn main() -> Result<()> {
    init_tracing()?;

    info!("starting daemon...");
    let (cec_thread, cmd_tx) = cec::spawn_thread();
    let (os_thread, mut event_rx) = os::windows::spawn_thread();

    while let Some(event) = event_rx.recv().await {
        cmd_tx.send(event.into()).await?;
    }

    signal::ctrl_c().await.unwrap();
    info!("received ctrl+c, stopping daemon...");

    Ok(())
}

fn init_tracing() -> Result<()> {
    color_eyre::install()?;
    tracing_subscriber::registry()
        .with(fmt::layer().without_time())
        .with(
            EnvFilter::builder()
                .with_default_directive(LevelFilter::ERROR.into())
                .parse("owl=trace")?,
        )
        .init();
    Ok(())
}
