use color_eyre::eyre::{eyre, Result};
use owl::{cec, os, Recv, Send, Spawn};
use tokio::signal;
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, info};

#[tokio::main]
async fn main() -> Result<()> {
    init_tracing()?;
    color_eyre::install()?;

    info!("starting owl...");
    let run_token = CancellationToken::new();
    let (cec_handle, cec) = cec::Job::spawn(run_token.clone()).await?;
    let (os_handle, mut os) = os::Job::spawn(run_token.clone()).await?;
    let handles = [cec_handle, os_handle];

    let owl_task = tokio::spawn(async move {
        while let Ok(event) = os.recv().await {
            if let Err(e) = cec.send(event.into()).await {
                error!("failed to send cec command: {e}");
            }
        }
    });

    info!("owl ready!");

    #[allow(clippy::ignored_unit_patterns)]
    {
        tokio::select! {
            _ = signal::ctrl_c() => {
                debug!("received CTRL+C");
                run_token.cancel();
            },
            _ = owl_task => error!("owl stopped unexpectedly?!"),
            _ = run_token.cancelled() => error!("run token cancelled?!"),
        }
    }

    info!("stopping owl...");
    for handle in handles {
        handle
            .join()
            .map_err(|e| eyre!("failed to join job handle: {e:?}"))??;
    }

    info!("owl stopped!");
    Ok(())
}

fn init_tracing() -> Result<()> {
    use tracing_error::ErrorLayer;
    use tracing_subscriber::{fmt, prelude::*, EnvFilter};

    let fmt_layer = fmt::layer();
    let filter_layer = EnvFilter::try_from_default_env()
        .or_else(|_| EnvFilter::try_new("owl=trace,owl::os::windows::internal=debug"))?;

    tracing_subscriber::registry()
        .with(filter_layer)
        .with(fmt_layer)
        .with(ErrorLayer::default())
        .try_init()?;

    Ok(())
}
