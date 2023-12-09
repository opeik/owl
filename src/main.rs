use std::time::Duration;

use color_eyre::Result;
use owl::{os::Event, *};
use tokio::signal;
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, info, level_filters::LevelFilter};
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

#[tokio::main]
async fn main() -> Result<()> {
    init_tracing()?;

    info!("starting owl...");
    let cancel_token = CancellationToken::new();
    let (cec_thread, cec_job) = cec::Job::spawn(cancel_token.clone());
    let (os_thread, mut os_job) = os::Job::spawn(cancel_token.clone());
    let job_threads = [cec_thread, os_thread];

    let owl_task = tokio::spawn(async move {
        while let Ok(event) = os_job.recv_event().await {
            if let Err(e) = cec_job.send_cmd(event.into()).await {
                error!("failed to send cec command: {e}");
            }
        }
    });

    tokio::select! {
        _ = signal::ctrl_c() => {
            info!("stopping owl...");
            cancel_token.cancel();
        }
        _ = cancel_token.cancelled() => {
            debug!("stop requested...")
        }
        _ = owl_task => {
            error!("owl stopped unexpectedly?!");
        }
    }

    info!("waiting for jobs to stop...");
    for thread in job_threads {
        thread.join().unwrap()?;
    }

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
