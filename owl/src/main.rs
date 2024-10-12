use color_eyre::eyre::{eyre, Context, Result};
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

    let owl_handle = tokio::spawn(async move {
        loop {
            let result: Result<()> = async {
                let event = os.recv().await.context("failed to receive os event")?;
                cec.send(event.into())
                    .await
                    .context("failed to send cec event")?;
                Result::Ok(())
            }
            .await;

            match result {
                Ok(()) => {}
                Err(e) => {
                    error!("owl error: {e:?}");
                }
            }
        }
    });

    info!("owl ready!");

    #[allow(clippy::ignored_unit_patterns, clippy::redundant_pub_crate)]
    {
        tokio::select! {
            _ = signal::ctrl_c() => {
                debug!("received CTRL+C");
                run_token.cancel();
            },
            _ = owl_handle => error!("owl stopped unexpectedly?!"),
            _ = run_token.cancelled() => error!("run token cancelled?!"),
        }
    }

    info!("stopping owl...");
    cec_handle
        .join()
        .map_err(|e| eyre!("failed to join cec job: {e:?}"))??;
    os_handle
        .join()
        .map_err(|e| eyre!("failed to join os job: {e:?}"))??;

    info!("owl stopped!");
    Ok(())
}

fn init_tracing() -> Result<()> {
    use tracing_error::ErrorLayer;
    use tracing_subscriber::{fmt, prelude::*, EnvFilter};

    let fmt_layer = fmt::layer();
    let filter_layer =
        EnvFilter::try_from_default_env().or_else(|_| EnvFilter::try_new("owl=trace"))?;
    // .or_else(|_| EnvFilter::try_new("owl=trace,owl::os::windows=debug"))?;

    tracing_subscriber::registry()
        .with(filter_layer)
        .with(fmt_layer)
        .with(ErrorLayer::default())
        .try_init()?;

    Ok(())
}
