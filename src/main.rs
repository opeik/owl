use color_eyre::eyre::{eyre, Result};
use owl::*;
use tokio::signal;
use tokio_util::sync::CancellationToken;
use tracing::{error, info};

#[tokio::main]
async fn main() -> Result<()> {
    init_tracing();

    info!("starting owl...");
    let run_token = CancellationToken::new();
    let (cec_thread, cec) = cec::Job::spawn(run_token.clone()).await?;
    let (os_thread, mut os) = os::Job::spawn(run_token.clone()).await?;

    let owl_task = tokio::spawn(async move {
        while let Ok(event) = os.recv().await {
            if let Err(e) = cec.send(event.into()).await {
                error!("failed to send cec command: {e}");
            }
        }
    });

    tokio::select! {
        _ = signal::ctrl_c() => run_token.cancel(),
        _ = owl_task => error!("owl stopped unexpectedly?!"),
        _ = run_token.cancelled() => {},
    }

    info!("stopping owl...");
    for thread in [cec_thread, os_thread] {
        thread
            .join()
            .map_err(|e| eyre!("failed to join thread: {e:?}"))??;
    }

    Ok(())
}

fn init_tracing() {
    use tracing_error::ErrorLayer;
    use tracing_subscriber::{fmt, prelude::*, EnvFilter};

    // let fmt_layer = fmt::layer().without_time();
    let fmt_layer = fmt::layer();
    let filter_layer = EnvFilter::try_from_default_env()
        .or_else(|_| EnvFilter::try_new("owl=trace"))
        .expect("failed to create tracing environment filter");

    tracing_subscriber::registry()
        .with(filter_layer)
        .with(fmt_layer)
        .with(ErrorLayer::default())
        .init();
}
