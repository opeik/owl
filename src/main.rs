use color_eyre::Result;
use owl::{
    cec::{self, Event},
    win32,
};
use tokio::{
    sync::mpsc::{self},
    task,
};
use tracing::{info, level_filters::LevelFilter};
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

#[tokio::main]
async fn main() -> Result<()> {
    init_tracing()?;

    let (tx, mut rx) = mpsc::channel::<Event>(32);
    owl::EVENT_TX.set(tx).unwrap();

    task::spawn_blocking(move || win32::init().unwrap());
    let cec = cec::init()?;

    while let Some(event) = rx.recv().await {
        event.handle(cec.clone()).await?;
    }

    info!("exiting...");
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
